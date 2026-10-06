//! Bounded OSRM Match publication. Chunk joins are technical Route Part seams,
//! never activity boundaries; an unsafe seam restores the original raw part.
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
const SEAM_TOLERANCE_M: f64 = 50.0;

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
struct Chunk<'a> {
    index: usize,
    points: Vec<&'a Observation>,
}
struct PublishedChunk<'a> {
    part: RoutePart,
    points: Vec<&'a Observation>,
    index: usize,
}

pub(super) async fn apply(parts: &mut Vec<RoutePart>, points: &[Observation], target: &Target) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(target.match_total_budget_ms as u64))
        .build()
        .expect("valid OSRM HTTP client configuration");
    let mut result = Vec::new();
    for original in std::mem::take(parts) {
        let Some(profile) = profile(original.mode, original.classification_confidence, target)
        else {
            result.push(reason(
                original,
                "not_eligible",
                "unknown_or_low_confidence",
            ));
            continue;
        };
        let selected = select(points, &original);
        if selected.len() < 2 {
            result.push(reason(
                original,
                "invalid_input",
                "fewer_than_two_distinct_seconds",
            ));
            continue;
        }
        let plans = chunks(selected, target);
        if plans.len() == 1 {
            result.push(
                match_chunk(
                    &client,
                    original,
                    profile,
                    plans.into_iter().next().unwrap(),
                    target,
                )
                .await,
            );
            continue;
        }
        let mut candidates = Vec::new();
        for plan in plans {
            let raw = raw_chunk(&original, &plan.points, plan.index);
            let index = plan.index;
            let source = plan.points.clone();
            candidates.push(PublishedChunk {
                part: match_chunk(&client, raw, profile, plan, target).await,
                points: source,
                index,
            });
        }
        if let Some(assembled) = assemble(candidates) {
            result.extend(assembled);
        } else {
            result.push(reason(
                original,
                "whole_segment_fallback",
                "unsafe_chunk_seam",
            ));
        }
    }
    *parts = result;
}

fn chunks<'a>(selected: Vec<&'a Observation>, target: &Target) -> Vec<Chunk<'a>> {
    let max = target.match_chunk_max_points.max(2) as usize;
    let overlap = (target.match_chunk_overlap_points.max(1) as usize).min(max - 1);
    chunk_ranges(selected.len(), max, overlap)
        .into_iter()
        .enumerate()
        .map(|(index, (start, end))| Chunk {
            index,
            points: selected[start..end].to_vec(),
        })
        .collect()
}
fn chunk_ranges(len: usize, max: usize, overlap: usize) -> Vec<(usize, usize)> {
    let (mut start, mut out) = (0, Vec::new());
    while start < len {
        let end = (start + max).min(len);
        out.push((start, end));
        if end == len {
            break;
        }
        start = end - overlap;
    }
    out
}
async fn match_chunk(
    client: &reqwest::Client,
    mut part: RoutePart,
    profile: &str,
    chunk: Chunk<'_>,
    target: &Target,
) -> RoutePart {
    let input = request(profile, &chunk.points);
    let hash = format!("{:x}", Sha256::digest(input.as_bytes()));
    match call(client, profile, &input, target).await {
        Ok((matched, attempts))
            if matched.confidence >= target.match_min_confidence
                && publish(&mut part, &chunk.points, &matched.geometry.coordinates) =>
        {
            part.source = "osrm_match";
            part.matcher_evidence = Some(evidence(
                target,
                profile,
                &hash,
                &input,
                &chunk.points,
                "matched",
                attempts,
                Some(matched.confidence),
                chunk.index,
            ));
        }
        Ok((matched, attempts)) => {
            part.matcher_evidence = Some(evidence(
                target,
                profile,
                &hash,
                &input,
                &chunk.points,
                if matched.confidence < target.match_min_confidence {
                    "low_confidence"
                } else {
                    "invalid_temporal_mapping"
                },
                attempts,
                Some(matched.confidence),
                chunk.index,
            ))
        }
        Err((status, attempts)) => {
            part.matcher_evidence = Some(evidence(
                target,
                profile,
                &hash,
                &input,
                &chunk.points,
                status,
                attempts,
                None,
                chunk.index,
            ))
        }
    };
    part
}
fn raw_chunk(original: &RoutePart, points: &[&Observation], index: usize) -> RoutePart {
    let coordinates: Vec<_> = points.iter().map(|p| [p.lon, p.lat]).collect();
    let vertex_distance_m = distances(&coordinates);
    RoutePart {
        id: format!("{}:chunk:{index}", original.id),
        kind: original.kind,
        trip_id: original.trip_id.clone(),
        movement_segment_id: original.movement_segment_id.clone(),
        source: "raw",
        mode: original.mode,
        classification_confidence: original.classification_confidence,
        observed_from_at: points[0].recorded_at,
        observed_until_at: points.last().unwrap().recorded_at,
        distance_m: *vertex_distance_m.last().unwrap(),
        progress_anchors: points
            .iter()
            .zip(&vertex_distance_m)
            .map(|(p, d)| ProgressAnchor {
                at: p.recorded_at,
                distance_m: *d,
            })
            .collect(),
        vertex_distance_m,
        quality: original.quality,
        source_record_count: points.len(),
        geometry: PartGeometry {
            geometry_type: "LineString",
            coordinates,
        },
        matcher_evidence: None,
        vertex_times: points.iter().map(|p| p.recorded_at).collect(),
    }
}

fn assemble(mut chunks: Vec<PublishedChunk<'_>>) -> Option<Vec<RoutePart>> {
    for index in 0..chunks.len().saturating_sub(1) {
        let (left, right) = chunks.split_at_mut(index + 1);
        let (left, right) = (&mut left[index], &mut right[0]);
        let seam = seam(left, right)?;
        trim_end(&mut left.part, seam.left_at, seam.left_progress)?;
        trim_start(&mut right.part, seam.right_at, seam.right_progress)?;
        annotate(&mut left.part, &seam, "end", left.index, right.index);
        annotate(&mut right.part, &seam, "start", left.index, right.index);
    }
    Some(chunks.into_iter().map(|c| c.part).collect())
}
struct Seam {
    id: i64,
    left_at: DateTime<Utc>,
    right_at: DateTime<Utc>,
    left_progress: f64,
    right_progress: f64,
    separation_m: f64,
}
fn seam(left: &PublishedChunk<'_>, right: &PublishedChunk<'_>) -> Option<Seam> {
    let mut candidates = Vec::new();
    for source in &left.points {
        if let Some(other) = right.points.iter().find(|p| p.id == source.id) {
            let la = left
                .part
                .progress_anchors
                .iter()
                .find(|a| a.at == source.recorded_at)?;
            let ra = right
                .part
                .progress_anchors
                .iter()
                .find(|a| a.at == other.recorded_at)?;
            let separation = geo::distance_m(
                coordinate_at(&left.part, la.distance_m)?,
                coordinate_at(&right.part, ra.distance_m)?,
            );
            if separation <= SEAM_TOLERANCE_M {
                let midpoint = (left.points.len() / 2)
                    .abs_diff(left.points.iter().position(|p| p.id == source.id).unwrap())
                    as f64;
                candidates.push((
                    quality_rank(source),
                    separation,
                    1_i32,
                    midpoint,
                    source.id,
                    Seam {
                        id: source.id,
                        left_at: la.at,
                        right_at: ra.at,
                        left_progress: la.distance_m,
                        right_progress: ra.distance_m,
                        separation_m: separation,
                    },
                ));
            }
        }
    }
    candidates
        .into_iter()
        .min_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.1.total_cmp(&b.1))
                .then(a.2.cmp(&b.2))
                .then(a.3.total_cmp(&b.3))
                .then(a.4.cmp(&b.4))
        })
        .map(|x| x.5)
}
fn coordinate_at(part: &RoutePart, progress: f64) -> Option<[f64; 2]> {
    part.vertex_distance_m
        .iter()
        .position(|d| *d >= progress)
        .map(|i| part.geometry.coordinates[i])
}
fn trim_end(part: &mut RoutePart, at: DateTime<Utc>, progress: f64) -> Option<()> {
    let end = part.vertex_distance_m.iter().position(|d| *d >= progress)?;
    if end == 0 {
        return None;
    }
    part.geometry.coordinates.truncate(end + 1);
    part.vertex_distance_m.truncate(end + 1);
    part.vertex_times.truncate(end + 1);
    part.progress_anchors.retain(|a| a.at <= at);
    part.observed_until_at = at;
    part.distance_m = *part.vertex_distance_m.last()?;
    Some(())
}
fn trim_start(part: &mut RoutePart, at: DateTime<Utc>, progress: f64) -> Option<()> {
    let start = part.vertex_distance_m.iter().position(|d| *d >= progress)?;
    if start + 1 >= part.geometry.coordinates.len() {
        return None;
    }
    let offset = part.vertex_distance_m[start];
    part.geometry.coordinates.drain(..start);
    part.vertex_distance_m.drain(..start);
    part.vertex_times.drain(..start);
    for d in &mut part.vertex_distance_m {
        *d -= offset;
    }
    part.progress_anchors.retain(|a| a.at >= at);
    for a in &mut part.progress_anchors {
        a.distance_m -= offset;
    }
    part.observed_from_at = at;
    part.distance_m = *part.vertex_distance_m.last()?;
    Some(())
}
fn annotate(part: &mut RoutePart, seam: &Seam, side: &str, left: usize, right: usize) {
    let mut value = part.matcher_evidence.take().unwrap_or_else(|| json!({}));
    value["seam"] = json!({"decision":"accepted_shared_source","side":side,"left_chunk":left,"right_chunk":right,"shared_source_id":seam.id,"left_progress_m":seam.left_progress,"right_progress_m":seam.right_progress,"separation_m":seam.separation_m,"bearing":"missing_ranked_after_available"});
    part.matcher_evidence = Some(value)
}
fn reason(mut part: RoutePart, status: &str, reason: &str) -> RoutePart {
    part.matcher_evidence = Some(json!({"status":status,"reason":reason}));
    part
}
fn profile(mode: &str, confidence: f64, target: &Target) -> Option<&'static str> {
    (confidence >= target.mode_enter_confidence).then(|| match mode {
        "walk" => Some("foot"),
        "bike" => Some("bike"),
        "car" => Some("car"),
        _ => None,
    })?
}
fn select<'a>(points: &'a [Observation], part: &RoutePart) -> Vec<&'a Observation> {
    let mut epochs = BTreeMap::new();
    for point in points.iter().filter(|p| {
        p.classification.is_usable()
            && p.recorded_at >= part.observed_from_at
            && p.recorded_at <= part.observed_until_at
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
    epochs.into_values().map(|(_, p)| p).collect()
}
fn quality_rank(point: &Observation) -> i64 {
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
        match client.get(format!("{base}{request}")).send().await {
            Ok(response) => match response.json::<Response>().await {
                Ok(response) if response.code == "NoMatch" => {
                    attempts.push(json!({"attempt":attempt,"elapsed_ms":(Utc::now()-began).num_milliseconds(),"status":"no_match"}));
                    return Err(("no_match", attempts));
                }
                Ok(response) if response.code == "Ok" => {
                    if let Some(matching) = response.matchings.into_iter().next() {
                        attempts.push(json!({"attempt":attempt,"elapsed_ms":(Utc::now()-began).num_milliseconds(),"status":"ok"}));
                        return Ok((matching, attempts));
                    } else {
                        return Err(("invalid_response", attempts));
                    }
                }
                _ => attempts.push(json!({"attempt":attempt,"status":"invalid_response"})),
            },
            Err(_) => attempts.push(json!({"attempt":attempt,"status":"dependency_error"})),
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
    let vertex_distance_m = distances(coordinates);
    let total = *vertex_distance_m.last().unwrap();
    if !total.is_finite() || total <= 0.0 {
        return false;
    }
    let progress_anchors: Vec<_> = points
        .iter()
        .map(|source| {
            let (index, _) = coordinates
                .iter()
                .enumerate()
                .map(|(i, c)| (i, geo::distance_m(*c, [source.lon, source.lat])))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            ProgressAnchor {
                at: source.recorded_at,
                distance_m: vertex_distance_m[index],
            }
        })
        .collect();
    if progress_anchors
        .windows(2)
        .any(|p| p[0].at >= p[1].at || p[0].distance_m > p[1].distance_m)
    {
        return false;
    }
    part.vertex_times = vertex_distance_m
        .iter()
        .map(|d| time_at(&progress_anchors, *d))
        .collect();
    part.geometry = PartGeometry {
        geometry_type: "LineString",
        coordinates: coordinates.to_vec(),
    };
    part.distance_m = total;
    part.vertex_distance_m = vertex_distance_m;
    part.progress_anchors = progress_anchors;
    true
}
fn distances(coordinates: &[[f64; 2]]) -> Vec<f64> {
    let mut out = vec![0.0];
    for pair in coordinates.windows(2) {
        out.push(out.last().unwrap() + geo::distance_m(pair[0], pair[1]));
    }
    out
}
fn time_at(anchors: &[ProgressAnchor], distance: f64) -> DateTime<Utc> {
    let pair = anchors
        .windows(2)
        .find(|p| p[1].distance_m >= distance)
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
    chunk: usize,
) -> Value {
    json!({"status":status,"profile":profile,"engine":target.matcher_engine_id,"dataset":target.matcher_dataset_id,"request_hash":hash,"request":request,"input":points.iter().map(|p|json!({"record_id":p.id,"at":p.recorded_at,"lon":p.lon,"lat":p.lat,"hdop":p.hdop,"selection_reason":"epoch_quality_hdop_satellites_id"})).collect::<Vec<_>>(),"chunk":{"id":chunk,"source_record_ids":points.iter().map(|p|p.id).collect::<Vec<_>>()},"parameters":{"min_confidence":target.match_min_confidence,"max_points":target.match_chunk_max_points,"overlap_points":target.match_chunk_overlap_points},"confidence":confidence,"attempts":attempts})
}

#[cfg(test)]
mod tests {
    use super::chunk_ranges;

    #[test]
    fn chunk_bounds_keep_shared_sources() {
        assert_eq!(
            chunk_ranges(157, 80, 5),
            vec![(0, 80), (75, 155), (150, 157)]
        );
    }
}
