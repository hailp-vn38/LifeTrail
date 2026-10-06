//! Ticket 05: how the Daily View publishes the three quality classes.
//!
//! Quality classification is the split between Raw GPS Records that support
//! activity, that exist but are of poor quality, and that are impossible. All
//! assertions read the public Daily View after a real `gps/1` upload, real
//! PostGIS persistence and the real processing worker.
mod support;

use lifetrail_server::{db, processing};
use serde_json::{Value, json};
use support::{
    at,
    scenarios::{self, METER_PER_DEGREE, STEP, TOLERANCE_M},
};
use uuid::Uuid;

/// The three published classes partition the Raw total for every day.
fn assert_classes_partition(view: &Value, context: &str) {
    let summary = &view["summary"];
    let total = summary["point_count"].as_u64().expect("point_count");
    let classes: u64 = [
        "usable_point_count",
        "low_quality_point_count",
        "excluded_point_count",
    ]
    .iter()
    .map(|field| {
        summary[*field]
            .as_u64()
            .unwrap_or_else(|| panic!("{context}: {field} is published"))
    })
    .sum();
    assert_eq!(
        classes, total,
        "{context}: the three classes partition the day"
    );
}

/// An impossible jump and a poor-quality observation are published as different
/// classes, so the Daily View can tell them apart without opening Raw GPS.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn impossible_jump_and_poor_quality_publish_distinct_classes() {
    let jump = scenarios::case("one impossible jump");
    let poor = scenarios::case("continuous low-quality observations");

    let excluded = scenarios::publish(&scenarios::records(&jump)).await.view;
    let low_quality = scenarios::publish(&scenarios::records(&poor)).await.view;

    // Both days record a comparable number of records, so the differing counts
    // describe the classification rather than a difference in Raw volume.
    assert_eq!(excluded["summary"]["point_count"], 7);
    assert_eq!(low_quality["summary"]["point_count"], 11);

    // The impossible record is the only rejected one, and nothing is poor.
    assert_eq!(excluded["summary"]["usable_point_count"], 6);
    assert_eq!(excluded["summary"]["excluded_point_count"], 1);
    assert_eq!(excluded["summary"]["low_quality_point_count"], 0);
    assert_eq!(
        excluded["evidence_holes"][0]["reason"],
        "insufficient_geometry"
    );

    // The five poor records are low-quality coverage, not impossible positions.
    assert_eq!(low_quality["summary"]["usable_point_count"], 6);
    assert_eq!(low_quality["summary"]["low_quality_point_count"], 5);
    assert_eq!(low_quality["summary"]["excluded_point_count"], 0);
    assert_eq!(
        low_quality["evidence_holes"][0]["reason"],
        "insufficient_quality"
    );

    // The distinction is load-bearing: no day may report a poor record as
    // excluded, which is what a two-way `point_count - usable_count` published.
    assert_ne!(
        excluded["summary"]["excluded_point_count"], low_quality["summary"]["excluded_point_count"],
        "a poor-quality record is never published as an impossible one"
    );
    for (view, context) in [
        (&excluded, "impossible jump"),
        (&low_quality, "poor quality"),
    ] {
        assert_classes_partition(view, context);
    }
}

/// Displacement is measured from the previous trusted position, so a legitimately
/// fast-moving Device stays excluded until it returns near the stale position.
/// This pins that deliberate trade-off: the rejected record stays in Raw GPS,
/// and recovery happens rather than poisoning the rest of the day.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn a_fast_moving_device_recovers_only_after_returning_near_the_trusted_position() {
    let base = at(8, 0);
    let leg = STEP * METER_PER_DEGREE;
    // Slow approach, one impossible jump, then real movement that keeps away from
    // the stale trusted position: 5.5 km north, then 1.1 km further north, then
    // back within the implied-speed budget of the last trusted fix.
    let records = vec![
        scenario_record(base, 10.7700),
        scenario_record(base + 60_000, 10.7705),
        scenario_record(base + 120_000, 10.8200),
        scenario_record(base + 180_000, 10.8201),
        scenario_record(base + 240_000, 10.8210),
        scenario_record(base + 300_000, 10.7705),
    ];
    let view = scenarios::publish(&records).await.view;

    assert_eq!(view["summary"]["point_count"], 6);
    // The jump and the record measured from the stale trusted position are both
    // excluded; the remaining four recover once the device is back in range.
    assert_eq!(view["summary"]["excluded_point_count"], 2);
    assert_eq!(view["summary"]["low_quality_point_count"], 0);
    assert_eq!(view["summary"]["usable_point_count"], 4);
    assert_classes_partition(&view, "latent over-exclusion");

    // Nothing rejected contributes distance: only the two short legs between
    // reliable records are published, and Raw GPS keeps all six records.
    let resumed = 0.0009 * METER_PER_DEGREE;
    let summed = scenarios::published_distance_m(&view);
    assert!(
        (summed - leg - resumed).abs() < TOLERANCE_M,
        "rejected records contribute no distance: {summed}"
    );
    let holes = view["evidence_holes"].as_array().unwrap();
    assert!(!holes.is_empty(), "the rejected run is disclosed as a hole");
    for hole in holes {
        assert_eq!(hole["reason"], "insufficient_geometry");
    }
}

/// An entirely unusable day is still a successful, truthful empty view, and its
/// records are reported as poor quality rather than as impossible positions.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn entirely_unusable_day_publishes_a_successful_empty_view() {
    let case = scenarios::case("entirely unusable day");
    let published = scenarios::publish(&scenarios::records(&case)).await;
    let view = &published.view;

    assert_eq!(view["processing_state"], "processed");
    assert_eq!(view["processing"]["data_freshness"], "current");
    assert_eq!(view["timeline"], json!([]));
    assert_eq!(view["route_parts"], json!([]));
    assert!(view["route"].is_null());
    assert!(view["start"].is_null());
    assert!(view["end"].is_null());
    // No reliable activity is derivable, so the evidence is insufficient.
    assert_eq!(view["evidence_state"], "insufficient");
    assert_eq!(view["summary"]["point_count"], 5);
    assert_eq!(view["summary"]["usable_point_count"], 0);
    assert_eq!(view["summary"]["low_quality_point_count"], 5);
    assert_eq!(view["summary"]["excluded_point_count"], 0);
    assert_eq!(view["summary"]["distance_m"], json!(0.0));
    assert_eq!(view["summary"]["duration_s"], 0);
    // Raw bounds still describe what the Device recorded.
    assert_eq!(view["summary"]["first_fix_at"], "2026-10-05T08:00:00Z");
    assert_eq!(view["summary"]["last_fix_at"], "2026-10-05T08:04:00Z");
    // The records exist, so their poor quality is disclosed rather than absent.
    assert_eq!(view["evidence_holes"][0]["reason"], "insufficient_quality");
    assert_classes_partition(view, "entirely unusable day");
}

/// The quality policy is centrally configurable, and changing it advances the
/// processing target so an older publication reads as stale until rebuilt.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn configured_quality_policy_reclassifies_observations_and_retires_publication() {
    let base = at(8, 0);
    // A qualifying dwell whose fixes report a poor dilution of precision.
    let dwell: Vec<Value> = (0..5)
        .map(|minute| {
            json!({
                "ts_ms": base + minute * 60_000, "lat": 10.7700, "lon": 106.7000,
                "fix_quality": 1, "satellites": 8, "hdop": 9.0
            })
        })
        .collect();
    let published = scenarios::publish(&dwell).await;
    let view = &published.view;
    let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();

    // The default policy rejects HDOP above the configured maximum, so the dwell
    // is not Stop evidence.
    assert_eq!(view["timeline"], json!([]));
    assert_eq!(view["summary"]["usable_point_count"], 0);
    assert_eq!(view["summary"]["low_quality_point_count"], 5);
    assert_eq!(view["evidence_holes"][0]["reason"], "insufficient_quality");
    assert_eq!(view["evidence_state"], "insufficient");

    let generation: i64 = sqlx::query_scalar(
        "SELECT target_generation FROM device_processing_control WHERE device_id=$1",
    )
    .bind(published.device_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE device_processing_control SET max_hdop=20 WHERE device_id=$1")
        .bind(published.device_id)
        .execute(&pool)
        .await
        .unwrap();
    // The publication built by the earlier policy stays readable, and reads as
    // stale until it is rebuilt under the new one.
    let retained = published.reread("2026-10-05").await;
    assert_eq!(retained["processing_state"], "processed");
    assert_eq!(retained["processing"]["data_freshness"], "stale");
    assert_eq!(retained["summary"]["usable_point_count"], 0);
    assert_eq!(retained["summary"]["low_quality_point_count"], 5);

    processing::queue_day(&pool, published.device_id, "2026-10-05".parse().unwrap())
        .await
        .unwrap();
    assert!(processing::process_next(&pool).await.unwrap());

    // The rebuilt publication classifies the same Raw records as reliable.
    let rebuilt = published.reread("2026-10-05").await;
    assert_eq!(rebuilt["processing"]["data_freshness"], "current");
    assert_eq!(rebuilt["summary"]["usable_point_count"], 5);
    assert_eq!(rebuilt["summary"]["low_quality_point_count"], 0);
    assert_eq!(rebuilt["summary"]["excluded_point_count"], 0);
    assert_eq!(rebuilt["evidence_holes"], json!([]));
    assert_eq!(rebuilt["evidence_state"], "sufficient");
    assert_classes_partition(&rebuilt, "relaxed policy");
    let dwell = &rebuilt["timeline"][0];
    assert_eq!(dwell["kind"], "stop");
    assert_eq!(dwell["observed_duration_s"], 240);

    // Provenance records the policy and the reducer that produced the revision.
    let revision = Uuid::parse_str(dwell["activity_revision"].as_str().unwrap()).unwrap();
    let config: Value = sqlx::query_scalar("SELECT config FROM activity_revisions WHERE id=$1")
        .bind(revision)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(config["max_hdop"], json!(20.0));
    assert_eq!(config["max_implied_speed_mps"], json!(70.0));
    assert_eq!(config["jump_distance_floor_m"], json!(100.0));
    let algorithms: Vec<&str> = config["algorithms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert!(
        algorithms.contains(&"raw-quality-classification-v1")
            && algorithms.contains(&"observed-gap-detection-v1"),
        "{algorithms:?}"
    );
    assert_eq!(rebuilt["provenance"]["reducer_version"], 2);
    let after: i64 = sqlx::query_scalar(
        "SELECT target_generation FROM device_processing_control WHERE device_id=$1",
    )
    .bind(published.device_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after, generation + 1, "a policy change advances the target");
}

/// One accepted gps/1 record at the default usable acquisition metadata.
fn scenario_record(ts_ms: i64, lat: f64) -> Value {
    json!({
        "ts_ms": ts_ms, "lat": lat, "lon": 106.7000,
        "fix_quality": 1, "satellites": 8, "hdop": 1.2
    })
}
