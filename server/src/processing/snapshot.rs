use super::{
    evidence::UnresolvedInterval,
    model::{Day, Input},
    stops::Stop,
};
use serde_json::{Value, json};
use uuid::Uuid;

pub(super) fn body(
    input: &Input,
    day: &Day,
    manifest: Uuid,
    stops: &[Stop],
    intervals: &[UnresolvedInterval],
) -> Value {
    let timeline: Vec<_> = stops
        .iter()
        .filter_map(|stop| {
            let from = stop.observed_from_at.max(day.from);
            let until = stop.observed_until_at.min(day.until);
            if from >= until {
                return None;
            }
            let mut item = serde_json::to_value(stop).expect("Stop serializes");
            item["visible_from_at"] = json!(from);
            item["visible_until_at"] = json!(until);
            item["daily_observed_duration_s"] = json!((until - from).num_seconds());
            item["continues_before"] = json!(stop.observed_from_at < day.from);
            item["continues_after"] = json!(stop.observed_until_at > day.until);
            Some(item)
        })
        .collect();
    let stop_duration: i64 = timeline
        .iter()
        .map(|s| s["daily_observed_duration_s"].as_i64().unwrap())
        .sum();
    let visible_intervals: Vec<_> = intervals.iter().filter_map(|hole| {
        let from = hole.observed_from_at.max(day.from);
        let until = hole.observed_until_at.min(day.until);
        (from < until).then(|| json!({"observed_from_at":from,"observed_until_at":until,"reason":hole.reason,"source_record_count":hole.source_record_count}))
    }).collect();
    let evidence = if timeline.is_empty() {
        "insufficient"
    } else if visible_intervals.is_empty() {
        "sufficient"
    } else {
        "partial"
    };
    let (evidence_holes, unresolved_intervals): (Vec<_>, Vec<_>) = visible_intervals
        .into_iter()
        .partition(|interval| interval["reason"] == "unusable_observations");
    json!({
        "device_id":input.claim.device_id,"date":day.date,"timezone":input.target.timezone,
        "processing_state":"processed","evidence_state":evidence,
        "route":null,"start":null,"end":null,"route_parts":[],"timeline":timeline,"evidence_holes":evidence_holes,"unresolved_intervals":unresolved_intervals,
        "summary":{
            "point_count":day.point_count,"usable_point_count":day.usable_count,
            "excluded_point_count":day.point_count-day.usable_count,
            "distance_m":0.0,"duration_s":stop_duration,"trip_duration_s":0,"stop_duration_s":stop_duration,"gap_duration_s":0,
            "trip_count":0,"stop_count":timeline.len(),"gap_count":0,"first_fix_at":day.first,"last_fix_at":day.last
        },
        "provenance":{
            "manifest_version":manifest,"source_raw_generation":input.target.input_generation,
            "processed_through_generation":input.target.input_generation,
            "processing_target":input.target.target_id,"processing_target_generation":input.target.target_generation,
            "timezone_generation":input.target.timezone_generation,"reducer_version":1,"projection_schema_version":1
        }
    })
}
