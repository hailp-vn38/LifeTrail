//! Build one published Daily Snapshot body from derived activity.
use super::{
    clip,
    derived::{Derived, REDUCER_VERSION},
    events,
    holes::EvidenceHole,
    model::{Day, Input},
};
use serde_json::{Value, json};
use uuid::Uuid;

pub(super) fn body(input: &Input, day: &Day, manifest: Uuid, derived: &Derived) -> Value {
    let timeline = events::timeline(&derived.stops, &derived.trips, &derived.gaps, day);
    let route_parts = clip::visible_parts(&derived.parts, day.from, day.until);
    let duration = |kind: &str| -> i64 {
        timeline
            .iter()
            .filter(|item| item["kind"] == kind)
            .map(|item| item["daily_observed_duration_s"].as_i64().unwrap_or(0))
            .sum()
    };
    let count =
        |kind: &str| -> usize { timeline.iter().filter(|item| item["kind"] == kind).count() };
    let trip_duration = duration("trip");
    let stop_duration = duration("stop");
    let gap_duration = duration("gap");
    let distance_m: f64 = route_parts.iter().map(|part| part.visible_distance_m).sum();
    let evidence_holes = clip_intervals(&derived.evidence_holes, day);
    // Evidence describes supported activity coverage. An explicit GPS Gap is a
    // truthful absence rather than unresolved coverage, so it never downgrades
    // an otherwise reliable day.
    let supported = count("trip") + count("stop") > 0;
    let evidence = if !supported {
        "insufficient"
    } else if evidence_holes.is_empty() {
        "sufficient"
    } else {
        "partial"
    };
    json!({
        "device_id":input.claim.device_id,"date":day.date,"timezone":input.target.timezone,
        "processing_state":"processed","evidence_state":evidence,
        // Processed Route Parts are canonical; `route`, `start` and `end` stay
        // empty so processed parts never enter the legacy Raw playback contract.
        "route":null,"start":null,"end":null,"route_parts":route_parts,
        "timeline":timeline,"evidence_holes":evidence_holes,
        "summary":{
            // All three classification classes are published separately, so the
            // Owner can tell an impossible record from a merely poor one. The
            // counts always partition `point_count`.
            "point_count":day.point_count,"usable_point_count":day.usable_count,
            "low_quality_point_count":day.low_quality_count,
            "excluded_point_count":day.excluded_count,
            "distance_m":distance_m,
            "duration_s":trip_duration+stop_duration+gap_duration,
            "trip_duration_s":trip_duration,"stop_duration_s":stop_duration,"gap_duration_s":gap_duration,
            "trip_count":count("trip"),"stop_count":count("stop"),"gap_count":count("gap"),
            "first_fix_at":day.first,"last_fix_at":day.last
        },
        "provenance":{
            "manifest_version":manifest,"source_raw_generation":input.target.input_generation,
            "processed_through_generation":input.target.input_generation,
            "processing_target":input.target.target_id,"processing_target_generation":input.target.target_generation,
            "timezone_generation":input.target.timezone_generation,"reducer_version":REDUCER_VERSION,"projection_schema_version":1
        }
    })
}

fn clip_intervals(intervals: &[EvidenceHole], day: &Day) -> Vec<Value> {
    intervals
        .iter()
        .filter_map(|interval| {
            let from = interval.observed_from_at.max(day.from);
            let until = interval.observed_until_at.min(day.until);
            (from < until).then(|| {
                json!({
                    "observed_from_at":from,"observed_until_at":until,
                    "reason":interval.reason,"source_record_count":interval.source_record_count
                })
            })
        })
        .collect()
}
