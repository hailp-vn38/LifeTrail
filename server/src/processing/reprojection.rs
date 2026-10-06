//! Projection-only Daily Snapshot construction from immutable activity history.
//!
//! This deliberately reads the pinned Activity Manifest instead of calling the
//! quality, segmentation, classification or matching pipeline again.
use super::model::{Day, Input};
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
    let source_generation: i64 = sqlx::query_scalar(
        "SELECT input_generation FROM activity_manifests WHERE id=$1 AND device_id=$2",
    )
    .bind(manifest)
    .bind(input.claim.device_id)
    .fetch_one(pool)
    .await?;
    let revisions: Vec<Value> = sqlx::query_scalar(
        "SELECT r.body FROM activity_manifests m \
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
            .flat_map(|body| body[key].as_array().into_iter().flatten().cloned())
            .collect::<Vec<_>>()
    };
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
        .filter_map(|hole| project(hole, day))
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
        "summary":{"point_count":day.point_count,"usable_point_count":day.usable_count,"low_quality_point_count":day.low_quality_count,"excluded_point_count":day.excluded_count,"distance_m":distance_m,"duration_s":trip_duration+stop_duration+gap_duration,"trip_duration_s":trip_duration,"stop_duration_s":stop_duration,"gap_duration_s":gap_duration,"trip_count":count("trip"),"stop_count":count("stop"),"gap_count":count("gap"),"first_fix_at":day.first,"last_fix_at":day.last},
        "provenance":{"manifest_version":manifest,"source_raw_generation":source_generation,"processed_through_generation":source_generation,"processing_target":input.target.target_id,"processing_target_generation":input.target.target_generation,"timezone_generation":input.target.timezone_generation,"projection_schema_version":1}
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
fn project_part(mut value: Value, day: &Day) -> Option<Value> {
    let (from, until) = bounds(&value)?;
    let visible_from = from.max(day.from);
    let visible_until = until.min(day.until);
    if visible_from >= visible_until {
        return None;
    }
    let anchors = value["progress_anchors"].as_array()?;
    let coordinates = value["geometry"]["coordinates"].as_array()?;
    // Route Parts persist historical anchors, so projection can clip their
    // geometry without matching again or using an estimated travel duration.
    if anchors.len() != coordinates.len() || anchors.len() < 2 {
        return None;
    }
    let at = |instant| anchor_position(anchors, coordinates, instant);
    let (start_coordinate, start_distance) = at(visible_from)?;
    let (end_coordinate, end_distance) = at(visible_until)?;
    let mut visible = vec![(visible_from, start_coordinate, start_distance)];
    for (anchor, coordinate) in anchors.iter().zip(coordinates) {
        let instant = anchor["at"].as_str()?.parse::<DateTime<Utc>>().ok()?;
        let distance = anchor["distance_m"].as_f64()?;
        if instant > visible_from && instant < visible_until {
            visible.push((instant, coordinate.clone(), distance));
        }
    }
    visible.push((visible_until, end_coordinate, end_distance));
    value["visible_from_at"] = json!(visible_from);
    value["visible_until_at"] = json!(visible_until);
    value["continues_before"] = json!(from < day.from);
    value["continues_after"] = json!(until > day.until);
    value["visible_distance_m"] = json!(end_distance - start_distance);
    value["geometry"] = json!({"type":"LineString","coordinates":visible.iter().map(|(_, coordinate, _)| coordinate).collect::<Vec<_>>()});
    value["vertex_distance_m"] = json!(
        visible
            .iter()
            .map(|(_, _, distance)| distance - start_distance)
            .collect::<Vec<_>>()
    );
    value["progress_anchors"] = json!(
        visible
            .iter()
            .map(|(at, _, distance)| json!({"at":at,"distance_m":distance-start_distance}))
            .collect::<Vec<_>>()
    );
    Some(value)
}

fn anchor_position(
    anchors: &[Value],
    coordinates: &[Value],
    instant: DateTime<Utc>,
) -> Option<(Value, f64)> {
    let indexed = anchors.iter().enumerate().find(|(_, anchor)| {
        anchor["at"]
            .as_str()
            .and_then(|at| at.parse::<DateTime<Utc>>().ok())
            .is_some_and(|at| at >= instant)
    })?;
    let next = indexed.0;
    if next == 0 {
        return Some((coordinates[0].clone(), anchors[0]["distance_m"].as_f64()?));
    }
    let previous = next - 1;
    let from = anchors[previous]["at"]
        .as_str()?
        .parse::<DateTime<Utc>>()
        .ok()?;
    let until = anchors[next]["at"]
        .as_str()?
        .parse::<DateTime<Utc>>()
        .ok()?;
    let ratio = (instant - from).num_milliseconds() as f64
        / (until - from).num_milliseconds().max(1) as f64;
    let from_distance = anchors[previous]["distance_m"].as_f64()?;
    let until_distance = anchors[next]["distance_m"].as_f64()?;
    let from_coordinate = coordinates[previous].as_array()?;
    let until_coordinate = coordinates[next].as_array()?;
    Some((
        json!([
            from_coordinate[0].as_f64()?
                + (until_coordinate[0].as_f64()? - from_coordinate[0].as_f64()?) * ratio,
            from_coordinate[1].as_f64()?
                + (until_coordinate[1].as_f64()? - from_coordinate[1].as_f64()?) * ratio
        ]),
        from_distance + (until_distance - from_distance) * ratio,
    ))
}
