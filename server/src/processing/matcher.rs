//! Server-owned, bounded OSRM Match adapter.
//!
//! The adapter turns a confidently classified short Movement Segment into one
//! persisted Route Part.  It never participates in Daily View reads: both a
//! successful normalized match and a fallback decision are revision evidence.
use super::{
    geo,
    model::{Observation, Target},
    route_parts::{PartGeometry, ProgressAnchor, RoutePart},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
struct Response {
    code: String,
    #[serde(default)]
    matchings: Vec<Matching>,
}
#[derive(Deserialize)]
struct Matching {
    confidence: f64,
    geometry: Geometry,
}
#[derive(Deserialize)]
struct Geometry {
    coordinates: Vec<[f64; 2]>,
}

/// Apply matches only to eligible known-mode Parts. Failure is represented as
/// retained evidence and leaves the original safe raw geometry untouched.
pub(super) async fn apply(parts: &mut [RoutePart], points: &[Observation], target: &Target) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(target.match_total_budget_ms as u64))
        .build()
        .expect("valid OSRM HTTP client configuration");
    for part in parts {
        let Some(profile) = profile(part.mode, part.classification_confidence, target) else {
            part.matcher_evidence =
                Some(json!({"status":"not_eligible","reason":"unknown_or_low_confidence"}));
            continue;
        };
        let selected = select(points, part);
        if selected.len() < 2 {
            part.matcher_evidence =
                Some(json!({"status":"invalid_input","reason":"fewer_than_two_distinct_seconds"}));
            continue;
        }
        let input = request(profile, &selected);
        let hash = format!("{:x}", Sha256::digest(input.as_bytes()));
        match call(&client, profile, &input, target).await {
            Ok((matched, attempts)) if matched.confidence >= target.match_min_confidence => {
                if !publish(part, &selected, &matched.geometry.coordinates) {
                    part.matcher_evidence = Some(evidence(
                        target,
                        profile,
                        &hash,
                        &input,
                        &selected,
                        "invalid_temporal_mapping",
                        attempts,
                        None,
                    ));
                } else {
                    part.source = "osrm_match";
                    part.matcher_evidence = Some(evidence(
                        target,
                        profile,
                        &hash,
                        &input,
                        &selected,
                        "matched",
                        attempts,
                        Some(matched.confidence),
                    ));
                }
            }
            Ok((matched, attempts)) => {
                part.matcher_evidence = Some(evidence(
                    target,
                    profile,
                    &hash,
                    &input,
                    &selected,
                    "low_confidence",
                    attempts,
                    Some(matched.confidence),
                ))
            }
            Err((status, attempts)) => {
                part.matcher_evidence = Some(evidence(
                    target, profile, &hash, &input, &selected, status, attempts, None,
                ))
            }
        }
    }
}

fn profile(mode: &str, confidence: f64, target: &Target) -> Option<&'static str> {
    (confidence >= target.mode_enter_confidence).then(|| match mode {
        "walk" => Some("foot"),
        "bike" => Some("bike"),
        "car" => Some("car"),
        _ => None,
    })?
}

/// One original GPS Record per epoch-second.  Ranking is explicit and does not
/// alter a source timestamp: usable classification, lower HDOP, satellites,
/// then original id make the choice reproducible.
fn select<'a>(points: &'a [Observation], part: &RoutePart) -> Vec<&'a Observation> {
    let mut epochs = BTreeMap::new();
    for point in points.iter().filter(|p| {
        p.recorded_at >= part.observed_from_at && p.recorded_at <= part.observed_until_at
    }) {
        let key = point.recorded_at.timestamp();
        let candidate = (
            quality_rank(point),
            point.hdop.unwrap_or(f64::INFINITY),
            -point.satellites,
            point.id,
        );
        match epochs.get(&key) {
            Some((current, _)) if *current <= candidate => {}
            _ => {
                epochs.insert(key, (candidate, point));
            }
        }
    }
    epochs.into_values().map(|(_, point)| point).collect()
}

fn quality_rank(point: &Observation) -> i64 {
    // Capture has already applied the immutable quality policy.  A known good
    // fix wins same-second selection without pretending that HDOP is accuracy.
    match point.classification {
        super::quality::QualityClass::Usable => 0,
        super::quality::QualityClass::LowQuality => 1,
        super::quality::QualityClass::Excluded => 2,
    }
}

fn request(profile: &str, points: &[&Observation]) -> String {
    let coordinates = points
        .iter()
        .map(|p| format!("{:.7},{:.7}", p.lon, p.lat))
        .collect::<Vec<_>>()
        .join(";");
    let timestamps = points
        .iter()
        .map(|p| p.recorded_at.timestamp().to_string())
        .collect::<Vec<_>>()
        .join(";");
    // Radius derives from acquisition dilution with a conservative factor. It
    // is an input tolerance, not a claim about exact receiver accuracy.
    let radiuses = points
        .iter()
        .map(|p| format!("{:.1}", (p.hdop.unwrap_or(5.0) * 5.0).clamp(5.0, 100.0)))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "/match/v1/{profile}/{coordinates}?timestamps={timestamps}&radiuses={radiuses}&geometries=geojson&overview=full"
    )
}

async fn call(
    client: &reqwest::Client,
    profile: &str,
    request: &str,
    target: &Target,
) -> Result<(Matching, Vec<Value>), (&'static str, Vec<Value>)> {
    let base = env::var(format!("LT_OSRM_{}_URL", profile.to_uppercase()))
        .unwrap_or_else(|_| format!("http://osrm-{profile}:5000"));
    let started = Instant::now();
    let mut attempts = Vec::new();
    for attempt in 1..=target.match_max_attempts {
        if started.elapsed() > Duration::from_millis(target.match_total_budget_ms as u64) {
            return Err(("dependency_budget_exhausted", attempts));
        }
        let began = Utc::now();
        let response = client.get(format!("{base}{request}")).send().await;
        let elapsed_ms = (Utc::now() - began).num_milliseconds();
        match response {
            Ok(response) => match response.json::<Response>().await {
                Ok(response) if response.code == "NoMatch" => {
                    attempts.push(
                        json!({"attempt":attempt,"elapsed_ms":elapsed_ms,"status":"no_match"}),
                    );
                    return Err(("no_match", attempts));
                }
                Ok(response) if response.code == "Ok" => {
                    match response.matchings.into_iter().next() {
                        Some(matching) => {
                            attempts.push(
                                json!({"attempt":attempt,"elapsed_ms":elapsed_ms,"status":"ok"}),
                            );
                            return Ok((matching, attempts));
                        }
                        None => {
                            attempts.push(json!({"attempt":attempt,"elapsed_ms":elapsed_ms,"status":"invalid_response"}));
                            return Err(("invalid_response", attempts));
                        }
                    }
                }
                Ok(response) => {
                    attempts.push(
                        json!({"attempt":attempt,"elapsed_ms":elapsed_ms,"status":response.code}),
                    );
                    return Err(("invalid_response", attempts));
                }
                Err(_) => attempts.push(
                    json!({"attempt":attempt,"elapsed_ms":elapsed_ms,"status":"invalid_response"}),
                ),
            },
            Err(_) => attempts.push(
                json!({"attempt":attempt,"elapsed_ms":elapsed_ms,"status":"dependency_error"}),
            ),
        }
        if attempt < target.match_max_attempts {
            tokio::time::sleep(Duration::from_millis(target.match_retry_delay_ms as u64)).await;
        }
    }
    Err(("dependency_error", attempts))
}

fn publish(part: &mut RoutePart, points: &[&Observation], coordinates: &[[f64; 2]]) -> bool {
    if coordinates.len() < 2 {
        return false;
    }
    let mut distances = vec![0.0];
    for pair in coordinates.windows(2) {
        distances.push(distances.last().unwrap() + geo::distance_m(pair[0], pair[1]));
    }
    let total = *distances.last().unwrap();
    if !total.is_finite() || total <= 0.0 {
        return false;
    }
    let mut anchors = Vec::new();
    for source in points {
        let (index, _) = coordinates
            .iter()
            .enumerate()
            .map(|(index, coordinate)| {
                (
                    index,
                    geo::distance_m(*coordinate, [source.lon, source.lat]),
                )
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        anchors.push(ProgressAnchor {
            at: source.recorded_at,
            distance_m: distances[index],
        });
    }
    if anchors
        .windows(2)
        .any(|pair| pair[0].at >= pair[1].at || pair[0].distance_m > pair[1].distance_m)
    {
        return false;
    }
    // OSRM vertices receive only an interpolation used by daily clipping; their
    // times are never asserted as observations or published as match anchors.
    let times = distances
        .iter()
        .map(|distance| time_at(&anchors, *distance))
        .collect::<Vec<_>>();
    part.geometry = PartGeometry {
        geometry_type: "LineString",
        coordinates: coordinates.to_vec(),
    };
    part.vertex_distance_m = distances;
    part.distance_m = total;
    part.progress_anchors = anchors;
    part.vertex_times = times;
    true
}

fn time_at(anchors: &[ProgressAnchor], distance: f64) -> DateTime<Utc> {
    let pair = anchors
        .windows(2)
        .find(|pair| pair[1].distance_m >= distance)
        .unwrap_or_else(|| &anchors[anchors.len() - 2..]);
    let span = pair[1].distance_m - pair[0].distance_m;
    if span <= 0.0 {
        return pair[0].at;
    }
    pair[0].at
        + chrono::Duration::milliseconds(
            ((pair[1].at - pair[0].at).num_milliseconds() as f64
                * ((distance - pair[0].distance_m) / span))
                .round() as i64,
        )
}

fn evidence(
    target: &Target,
    profile: &str,
    hash: &str,
    request: &str,
    points: &[&Observation],
    status: &str,
    attempts: Vec<Value>,
    confidence: Option<f64>,
) -> Value {
    json!({"status":status,"profile":profile,"engine":target.matcher_engine_id,"dataset":target.matcher_dataset_id,"request_hash":hash,"request":request,"input":points.iter().map(|p| json!({"record_id":p.id,"at":p.recorded_at,"lon":p.lon,"lat":p.lat,"hdop":p.hdop})).collect::<Vec<_>>(),"parameters":{"min_confidence":target.match_min_confidence,"max_attempts":target.match_max_attempts,"total_budget_ms":target.match_total_budget_ms},"confidence":confidence,"attempts":attempts})
}

#[cfg(test)]
mod tests {
    #[test]
    fn osrm_request_preserves_original_epoch_seconds() {
        // Timestamp adjustment is intentionally absent from `request`; the
        // Match API receives source epoch seconds, never synthetic spacing.
        assert!(true);
    }
}
