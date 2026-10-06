use super::model::{Day, Input};
use serde_json::{Value, json};
use uuid::Uuid;

pub(super) fn body(input: &Input, day: &Day, manifest: Uuid) -> Value {
    json!({
        "device_id":input.claim.device_id,"date":day.date,"timezone":input.target.timezone,
        "processing_state":"processed","evidence_state":"insufficient",
        "route":null,"start":null,"end":null,"route_parts":[],"timeline":[],"evidence_holes":[],
        "summary":{
            "point_count":day.point_count,"usable_point_count":day.usable_count,
            "excluded_point_count":day.point_count-day.usable_count,
            "distance_m":0.0,"duration_s":0,"trip_duration_s":0,"stop_duration_s":0,"gap_duration_s":0,
            "trip_count":0,"stop_count":0,"gap_count":0,"first_fix_at":day.first,"last_fix_at":day.last
        },
        "provenance":{
            "manifest_version":manifest,"source_raw_generation":input.target.input_generation,
            "processed_through_generation":input.target.input_generation,
            "processing_target":input.target.target_id,"processing_target_generation":input.target.target_generation,
            "timezone_generation":input.target.timezone_generation,"reducer_version":1,"projection_schema_version":1
        }
    })
}
