//! Daily Snapshot construction from authoritative immutable manifest slices.
//!
//! This deliberately reads the pinned Activity Manifest instead of calling the
//! quality, segmentation or classification pipeline again.
use super::{
    model::{Day, Input},
    progress_projection::project_part,
};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn body(
    pool: &PgPool,
    input: &Input,
    day: &Day,
    manifest: Uuid,
) -> Result<Value, sqlx::Error> {
    let (source_generation, source_target): (i64, String) = sqlx::query_as(
        "SELECT input_generation,target_id FROM activity_manifests WHERE id=$1 AND device_id=$2",
    )
    .bind(manifest)
    .bind(input.claim.device_id)
    .fetch_one(pool)
    .await?;
    let revisions: Vec<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object('body',r.body,'from_at',entry->>'from_at','until_at',entry->>'until_at','target_generation',r.target_generation,'reducer_version',r.config->'reducer_version') FROM activity_manifests m \
         CROSS JOIN LATERAL jsonb_array_elements(m.entries) entry \
         JOIN activity_revisions r ON r.id=(entry->>'activity_revision')::uuid \
         WHERE m.id=$1 AND m.device_id=$2 ORDER BY r.observed_from_at",
    )
    .bind(manifest)
    .bind(input.claim.device_id)
    .fetch_all(pool)
    .await?;
    let values = |key| {
        revisions
            .iter()
            .flat_map(|body| {
                body["body"][key]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|item| {
                        let instant =
                            |v: &Value| v.as_str().and_then(|s| s.parse::<DateTime<Utc>>().ok());
                        match (
                            instant(
                                &item[if key == "quality_observations" {
                                    "recorded_at"
                                } else {
                                    "observed_from_at"
                                }],
                            ),
                            instant(&body["from_at"]),
                            instant(&body["until_at"]),
                        ) {
                            (Some(at), Some(from), Some(until)) => at >= from && at < until,
                            _ => false,
                        }
                    })
                    .cloned()
            })
            .collect::<Vec<_>>()
    };
    // Reprojection consumes immutable quality audit; it never reclassifies Raw
    // observations under today's policy, and does not invent zero usable fixes.
    let audit = values("quality_observations");
    let quality_count = |class: &str| {
        audit
            .iter()
            .filter(|point| {
                point["classification"] == class
                    && point["recorded_at"]
                        .as_str()
                        .and_then(|at| at.parse::<DateTime<Utc>>().ok())
                        .is_some_and(|at| at >= day.from && at < day.until)
            })
            .count() as i64
    };
    let usable_count = if input.projection_only {
        quality_count("usable")
    } else {
        day.usable_count
    };
    let low_quality_count = if input.projection_only {
        quality_count("low_quality")
    } else {
        day.low_quality_count
    };
    let excluded_count = if input.projection_only {
        quality_count("excluded")
    } else {
        day.excluded_count
    };
    let source_target_generation = revisions
        .first()
        .and_then(|r| r["target_generation"].as_i64())
        .unwrap_or(input.target.target_generation);
    let reducer_version = revisions
        .first()
        .and_then(|r| r["reducer_version"].as_i64())
        .unwrap_or(super::derived::REDUCER_VERSION);
    let stops = values("stops");
    let trips = values("trips");
    let gaps = values("gaps");
    let parts = values("route_parts");
    let holes = values("evidence_holes");
    let mut timeline = stops
        .into_iter()
        .chain(trips)
        .chain(gaps)
        .filter_map(|item| project(item, day))
        .collect::<Vec<_>>();
    timeline.sort_by_key(|item| item["visible_from_at"].as_str().map(str::to_owned));
    let route_parts = parts
        .into_iter()
        .filter_map(|part| project_part(part, day))
        .collect::<Vec<_>>();
    let evidence_holes = holes
        .into_iter()
        .filter_map(|mut hole| {
            let (from, until) = bounds(&hole)?;
            let from = from.max(day.from);
            let until = until.min(day.until);
            if from >= until {
                return None;
            }
            hole["observed_from_at"] = json!(from);
            hole["observed_until_at"] = json!(until);
            Some(hole)
        })
        .collect::<Vec<_>>();
    let count = |kind| timeline.iter().filter(|item| item["kind"] == kind).count();
    let duration = |kind| {
        timeline
            .iter()
            .filter(|item| item["kind"] == kind)
            .map(|item| item["daily_observed_duration_s"].as_i64().unwrap_or(0))
            .sum::<i64>()
    };
    let trip_duration = duration("trip");
    let stop_duration = duration("stop");
    let gap_duration = duration("gap");
    let distance_m = route_parts
        .iter()
        .map(|part| part["visible_distance_m"].as_f64().unwrap_or(0.0))
        .sum::<f64>();
    let supported = count("trip") + count("stop") > 0;
    Ok(json!({
        "device_id":input.claim.device_id,"date":day.date,"timezone":input.target.timezone,
        "processing_state":"processed","evidence_state": if !supported { "insufficient" } else if evidence_holes.is_empty() { "sufficient" } else { "partial" },
        "route":null,"start":null,"end":null,"route_parts":route_parts,"timeline":timeline,"evidence_holes":evidence_holes,
        "summary":{"point_count":day.point_count,"usable_point_count":usable_count,"low_quality_point_count":low_quality_count,"excluded_point_count":excluded_count,"distance_m":distance_m,"duration_s":trip_duration+stop_duration+gap_duration,"trip_duration_s":trip_duration,"stop_duration_s":stop_duration,"gap_duration_s":gap_duration,"trip_count":count("trip"),"stop_count":count("stop"),"gap_count":count("gap"),"first_fix_at":day.first,"last_fix_at":day.last},
        "provenance":{"manifest_version":manifest,"source_raw_generation":source_generation,"processed_through_generation":source_generation,"processing_target":source_target,"processing_target_generation":source_target_generation,"timezone_generation":input.target.timezone_generation,"reducer_version":reducer_version,"projection_schema_version":1}
    }))
}

fn bounds(value: &Value) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    Some((
        value["observed_from_at"].as_str()?.parse().ok()?,
        value["observed_until_at"].as_str()?.parse().ok()?,
    ))
}
fn project(mut value: Value, day: &Day) -> Option<Value> {
    let (from, until) = bounds(&value)?;
    let visible_from = from.max(day.from);
    let visible_until = until.min(day.until);
    (visible_from < visible_until).then(|| {
        value["visible_from_at"] = json!(visible_from);
        value["visible_until_at"] = json!(visible_until);
        value["daily_observed_duration_s"] = json!((visible_until - visible_from).num_seconds());
        value["continues_before"] = json!(from < day.from);
        value["continues_after"] = json!(until > day.until);
        value
    })
}
