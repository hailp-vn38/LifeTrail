//! Ticket 05: quality failures, GPS Gaps and Evidence Holes.
//!
//! Every assertion here reads the public Daily View after a real `gps/1`
//! upload, real PostGIS persistence and the real processing worker. Raw GPS is
//! compared before and after processing, never through a private classifier.
mod support;

use axum::http::StatusCode;
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db, processing,
};
use serde_json::{Value, json};
use support::{ndjson, read, upload};
use uuid::Uuid;

/// Meters per degree of latitude on the sphere used by the server's geodesic
/// helper. Independent of the implementation under test.
const METER_PER_DEGREE: f64 = 111_194.926_644_558_74;
/// 0.0005 degrees of latitude: 55.6 m, one fixture leg and beyond the Stop radius.
const STEP: f64 = 0.0005;
const TOLERANCE_M: f64 = 0.5;

/// Deterministic quality, gap and hole scenarios, including a genuine
/// timestamp absence, continuous low-quality observations and an impossible
/// jump.
fn cases() -> Vec<Value> {
    serde_json::from_str(include_str!("fixtures/quality_gaps.json")).unwrap()
}

fn case(name: &str) -> Value {
    cases()
        .into_iter()
        .find(|case| case["name"] == name)
        .unwrap_or_else(|| panic!("fixture case {name}"))
}

fn at(hour: u32, minute: u32) -> i64 {
    chrono::DateTime::parse_from_rfc3339(&format!("2026-10-05T{hour:02}:{minute:02}:00Z"))
        .unwrap()
        .timestamp_millis()
}

/// Fixture records as exact gps/1 rows, relative to 08:00 UTC.
fn records(case: &Value) -> Vec<Value> {
    let base = at(8, 0);
    case["records"]
        .as_array()
        .expect("fixture records")
        .iter()
        .map(|record| {
            json!({
                "ts_ms": base + record["ts_s"].as_i64().expect("offset") * 1_000,
                "lat": record["lat"], "lon": 106.7000,
                "fix_quality": record["fix_quality"],
                "satellites": record.get("satellites").and_then(Value::as_i64).unwrap_or(8),
                "hdop": record.get("hdop").and_then(Value::as_f64).unwrap_or(1.2),
                "speed_mps": record.get("speed_mps").and_then(Value::as_f64),
            })
        })
        .collect()
}

/// Provision one Owner and Device, upload `records` and run the worker once.
async fn publish(records: &[Value]) -> (axum::Router, Value, Uuid) {
    let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("TRUNCATE users CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    let owner = db::create_owner(&pool, "Owner", "UTC").await.unwrap();
    let token = generate_device_token();
    let device = db::create_device(&pool, owner.id, "Device", &token)
        .await
        .unwrap();
    let router = app::router(AppState { db: pool.clone() }, None);
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &ndjson(records))
            .await
            .0,
        StatusCode::OK
    );
    let raw = raw_rows(&pool).await;
    assert!(processing::process_next(&pool).await.unwrap());
    let view = read(
        &router,
        &format!("/api/v1/devices/{}/days/2026-10-05", device.id),
    )
    .await;
    assert_eq!(
        raw,
        raw_rows(&pool).await,
        "Raw GPS is unchanged by processing"
    );
    (router, view, device.id)
}

/// Every persisted GPS Record, for the Raw immutability invariant.
async fn raw_rows(pool: &sqlx::PgPool) -> Vec<String> {
    sqlx::query_scalar("SELECT row_to_json(g)::text FROM gps_points g ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
}

fn kinds(view: &Value) -> Vec<String> {
    view["timeline"]
        .as_array()
        .expect("timeline")
        .iter()
        .map(|item| item["kind"].as_str().unwrap().to_owned())
        .collect()
}

fn trips(view: &Value) -> Vec<&Value> {
    view["timeline"]
        .as_array()
        .expect("timeline")
        .iter()
        .filter(|item| item["kind"] == "trip")
        .collect()
}

/// A genuine absence of Raw observations is a GPS Gap, never a Stop, movement
/// or a straight Route connector, and contributes only its own duration.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn genuine_absence_publishes_a_gap_event_and_ends_trip_continuity() {
    let case = case("genuine timestamp absence");
    let (_router, view, _device) = publish(&records(&case)).await;

    assert_eq!(view["processing_state"], "processed");
    assert_eq!(kinds(&view), ["trip", "gap", "trip"]);

    let gap = &view["timeline"][1];
    assert_eq!(gap["observed_from_at"], "2026-10-05T08:02:00Z");
    assert_eq!(gap["observed_until_at"], "2026-10-05T08:13:00Z");
    assert_eq!(gap["observed_duration_s"], 660);
    assert_eq!(gap["daily_observed_duration_s"], 660);
    assert_eq!(gap["continues_before"], json!(false));
    assert_eq!(gap["continues_after"], json!(false));
    // A Gap asserts neither movement nor a stationary location.
    assert!(gap.get("center").is_none());
    assert!(gap.get("movement_segments").is_none());
    assert_eq!(view["evidence_holes"], json!([]));
    // An explicit GPS Gap alone does not downgrade otherwise reliable activity.
    assert_eq!(view["evidence_state"], "sufficient");
    assert!(view.get("unresolved_intervals").is_none());

    // Gap duration is contributed, and no Stop is inferred inside it.
    assert_eq!(view["summary"]["gap_count"], 1);
    assert_eq!(view["summary"]["gap_duration_s"], 660);
    assert_eq!(view["summary"]["stop_count"], 0);
    assert_eq!(view["summary"]["trip_count"], 2);
    assert_eq!(view["summary"]["trip_duration_s"], 120 + 120);
    assert_eq!(view["summary"]["duration_s"], 120 + 660 + 120);

    // Two Trips with disconnected geometry; no part bridges the absence.
    let parts = view["route_parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2);
    for part in parts {
        support::assert_route_part(part, "gap-adjacent part", TOLERANCE_M);
    }
    assert_eq!(parts[0]["observed_until_at"], "2026-10-05T08:02:00Z");
    assert_eq!(parts[1]["observed_from_at"], "2026-10-05T08:13:00Z");
    assert_ne!(parts[0]["trip_id"], parts[1]["trip_id"]);
    // Distance is the published legs only; the Gap adds no connector.
    let leg = STEP * METER_PER_DEGREE;
    let summed: f64 = parts
        .iter()
        .map(|part| part["distance_m"].as_f64().unwrap())
        .sum();
    assert!((summed - 4.0 * leg).abs() < TOLERANCE_M, "{summed}");
    assert!(
        (view["summary"]["distance_m"].as_f64().unwrap() - summed).abs() < 1e-9,
        "daily distance sums published part lengths only"
    );
}

/// An impossible jump stays in Raw GPS but contributes no derived geometry,
/// distance or activity, and leaves the activity on either side open.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn impossible_jump_leaves_raw_intact_but_publishes_no_excursion() {
    let case = case("one impossible jump");
    let (_router, view, _device) = publish(&records(&case)).await;

    // Every Raw record is counted, and the rejected one is counted separately.
    assert_eq!(view["summary"]["point_count"], 7);
    assert_eq!(view["summary"]["usable_point_count"], 6);
    assert_eq!(view["summary"]["excluded_point_count"], 1);
    assert_eq!(
        view["summary"]["usable_point_count"].as_u64().unwrap()
            + view["summary"]["excluded_point_count"].as_u64().unwrap(),
        view["summary"]["point_count"].as_u64().unwrap(),
        "usable and excluded counts never exceed the Raw total"
    );

    // The jump is disclosed as Evidence Hole coverage, never as a GPS Gap.
    let holes = view["evidence_holes"].as_array().unwrap();
    assert_eq!(holes.len(), 1, "{holes:?}");
    assert_eq!(holes[0]["reason"], "insufficient_geometry");
    assert_eq!(holes[0]["observed_from_at"], "2026-10-05T08:02:00Z");
    assert_eq!(holes[0]["observed_until_at"], "2026-10-05T08:04:00Z");
    assert_eq!(view["summary"]["gap_count"], 0, "a jump is not a Gap");
    assert_eq!(view["summary"]["gap_duration_s"], 0);
    assert_eq!(view["evidence_state"], "partial");

    // No published geometry reaches the impossible position.
    let parts = view["route_parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2, "movement either side stays separate Trips");
    for part in parts {
        for coordinate in part["geometry"]["coordinates"].as_array().unwrap() {
            assert_ne!(*coordinate, json!([106.7, 10.82]), "{coordinate}");
        }
    }
    let leg = STEP * METER_PER_DEGREE;
    let summed: f64 = parts
        .iter()
        .map(|part| part["distance_m"].as_f64().unwrap())
        .sum();
    assert!(
        (summed - 4.0 * leg).abs() < TOLERANCE_M,
        "the jump contributes no distance: {summed}"
    );
    assert!((view["summary"]["distance_m"].as_f64().unwrap() - summed).abs() < 1e-9);

    // Activity beside the hole keeps open actual boundaries rather than
    // asserting a start or end the observations cannot prove.
    let trips = trips(&view);
    assert_eq!(trips.len(), 2);
    assert_eq!(trips[0]["end_boundary"], "open");
    assert!(trips[0]["actual_end_at"].is_null());
    assert!(trips[0]["full_duration_s"].is_null());
    assert_eq!(trips[1]["start_boundary"], "open");
    assert!(trips[1]["actual_start_at"].is_null());
    assert!(trips[1]["full_duration_s"].is_null());
}

/// Poor observations at the *same* timestamps as a real absence behave
/// differently: existing records become Evidence Hole coverage, never a Gap.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn low_quality_observations_at_the_same_timestamps_are_evidence_holes() {
    let absent = case("genuine timestamp absence");
    let poor = case("continuous low-quality observations");
    // Both scenarios describe 08:00..08:15; only one of them is missing data.
    // The default observation gap is 300 seconds.
    let observed_every = |records: &[Value]| {
        records
            .iter()
            .map(|record| record["ts_ms"].as_i64().expect("ts_ms"))
            .collect::<Vec<i64>>()
    };
    let poor_times = observed_every(&records(&poor));
    let absent_times = observed_every(&records(&absent));
    assert!(
        poor_times
            .windows(2)
            .all(|pair| pair[1] - pair[0] <= 300_000),
        "the poor scenario keeps every minute observed: {poor_times:?}"
    );
    assert!(
        absent_times
            .windows(2)
            .any(|pair| pair[1] - pair[0] > 300_000),
        "the absent scenario really misses observations: {absent_times:?}"
    );
    let (_router, view, _device) = publish(&records(&poor)).await;

    assert_eq!(view["processing_state"], "processed");
    assert_eq!(view["summary"]["point_count"], 11);
    assert_eq!(view["summary"]["usable_point_count"], 6);
    assert_eq!(view["summary"]["excluded_point_count"], 5);
    // Every record exists, so no interval without observations is a GPS Gap.
    assert_eq!(view["summary"]["gap_count"], 0);
    assert_eq!(view["summary"]["gap_duration_s"], 0);
    assert_eq!(kinds(&view), ["trip", "trip"]);

    // The unreliable run is disclosed as Evidence Hole coverage with a reason.
    let holes = view["evidence_holes"].as_array().unwrap();
    assert_eq!(holes.len(), 1, "{holes:?}");
    assert_eq!(holes[0]["reason"], "insufficient_quality");
    assert_eq!(holes[0]["observed_from_at"], "2026-10-05T08:02:00Z");
    assert_eq!(holes[0]["observed_until_at"], "2026-10-05T08:13:00Z");
    assert_eq!(holes[0]["source_record_count"], 7);
    // Reliable activity plus unresolved coverage is partial evidence.
    assert_eq!(view["evidence_state"], "partial");

    // Neither side of the hole is claimed to be one Trip, and no Stop is
    // inferred across it.
    let trips = trips(&view);
    assert_eq!(trips.len(), 2, "the hole splits movement into two Trips");
    assert_eq!(view["summary"]["stop_count"], 0);
    assert_ne!(trips[0]["id"], trips[1]["id"]);
    let parts = view["route_parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2);
    // No geometry bridges the hole, and the Trip beside it stays open.
    assert_eq!(parts[0]["observed_until_at"], "2026-10-05T08:02:00Z");
    assert_eq!(parts[1]["observed_from_at"], "2026-10-05T08:13:00Z");
    assert_eq!(trips[0]["end_boundary"], "open");
    assert!(trips[0]["actual_end_at"].is_null());
    assert_eq!(trips[1]["start_boundary"], "open");
    assert!(trips[1]["actual_start_at"].is_null());
}

/// An entirely unusable day is still a successful, truthful empty view.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn entirely_unusable_day_publishes_a_successful_empty_view() {
    let case = case("entirely unusable day");
    let (_router, view, _device) = publish(&records(&case)).await;

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
    assert_eq!(view["summary"]["excluded_point_count"], 5);
    assert_eq!(view["summary"]["distance_m"], json!(0.0));
    assert_eq!(view["summary"]["duration_s"], 0);
    // Raw bounds still describe what the Device recorded.
    assert_eq!(view["summary"]["first_fix_at"], "2026-10-05T08:00:00Z");
    assert_eq!(view["summary"]["last_fix_at"], "2026-10-05T08:04:00Z");
    // The records exist, so their poor quality is disclosed rather than absent.
    assert_eq!(view["evidence_holes"][0]["reason"], "insufficient_quality");
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
    let (router, view, device) = publish(&dwell).await;
    let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();

    // The default policy rejects HDOP above the configured maximum, so the dwell
    // is not Stop evidence.
    assert_eq!(view["timeline"], json!([]));
    assert_eq!(view["summary"]["usable_point_count"], 0);
    assert_eq!(view["evidence_holes"][0]["reason"], "insufficient_quality");
    assert_eq!(view["evidence_state"], "insufficient");

    let generation: i64 = sqlx::query_scalar(
        "SELECT target_generation FROM device_processing_control WHERE device_id=$1",
    )
    .bind(device)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE device_processing_control SET max_hdop=20 WHERE device_id=$1")
        .bind(device)
        .execute(&pool)
        .await
        .unwrap();
    let path = format!("/api/v1/devices/{device}/days/2026-10-05");
    // The publication built by the earlier policy stays readable, and reads as
    // stale until it is rebuilt under the new one.
    let retained = read(&router, &path).await;
    assert_eq!(retained["processing_state"], "processed");
    assert_eq!(retained["processing"]["data_freshness"], "stale");
    assert_eq!(retained["summary"]["usable_point_count"], 0);

    processing::queue_day(&pool, device, "2026-10-05".parse().unwrap())
        .await
        .unwrap();
    assert!(processing::process_next(&pool).await.unwrap());

    // The rebuilt publication classifies the same Raw records as reliable.
    let rebuilt = read(&router, &path).await;
    assert_eq!(rebuilt["processing"]["data_freshness"], "current");
    assert_eq!(rebuilt["summary"]["usable_point_count"], 5);
    assert_eq!(rebuilt["evidence_holes"], json!([]));
    assert_eq!(rebuilt["evidence_state"], "sufficient");
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
    .bind(device)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after, generation + 1, "a policy change advances the target");
}

/// Every deterministic scenario publishes exactly the disclosed evidence: Gap
/// events, Evidence Holes with reasons, Trips and truthful counts.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn quality_gap_fixtures_publish_truthful_evidence() {
    for case in cases() {
        let name = case["name"].as_str().unwrap();
        let records = records(&case);
        let (_router, view, _device) = publish(&records).await;

        assert_eq!(view["processing_state"], "processed", "{name}");
        assert_eq!(
            json!(kinds(&view)),
            case["kinds"],
            "{name}: chronological kinds"
        );
        assert_eq!(view["evidence_state"], case["evidence_state"], "{name}");

        let published_gaps: Vec<[&str; 2]> = view["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["kind"] == "gap")
            .map(|item| {
                [
                    item["observed_from_at"].as_str().unwrap(),
                    item["observed_until_at"].as_str().unwrap(),
                ]
            })
            .collect();
        assert_eq!(
            json!(published_gaps),
            case["gaps"],
            "{name}: GPS Gaps are Timeline events"
        );
        assert_eq!(
            view["summary"]["gap_count"].as_u64().unwrap() as usize,
            case["gaps"].as_array().unwrap().len(),
            "{name}: gap_count"
        );

        let holes: Vec<Value> = view["evidence_holes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|hole| {
                json!([
                    hole["reason"],
                    hole["observed_from_at"],
                    hole["observed_until_at"],
                    hole["source_record_count"],
                ])
            })
            .collect();
        assert_eq!(json!(holes), case["holes"], "{name}: Evidence Holes");

        let trips = trips(&view);
        let bounds: Vec<[&str; 2]> = trips
            .iter()
            .map(|trip| {
                [
                    trip["observed_from_at"].as_str().unwrap(),
                    trip["observed_until_at"].as_str().unwrap(),
                ]
            })
            .collect();
        assert_eq!(json!(bounds), case["trip_bounds"], "{name}: Trips");
        assert_eq!(
            view["summary"]["trip_count"].as_u64().unwrap() as usize,
            trips.len(),
            "{name}: trip_count"
        );

        // Rejected geometry and connectors never inflate published distance.
        let legs = case["trip_legs"].as_f64().expect("trip_legs");
        let summed: f64 = view["route_parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| part["distance_m"].as_f64().unwrap())
            .sum();
        assert_eq!(
            view["route_parts"].as_array().unwrap().len(),
            trips.len(),
            "{name}"
        );
        assert!(
            (summed - legs * STEP * METER_PER_DEGREE).abs() < TOLERANCE_M,
            "{name}: published distance {summed} carries no rejected geometry"
        );

        // Raw counts are exhaustive and never claim more than was recorded.
        assert_eq!(view["summary"]["point_count"], records.len(), "{name}");
        assert_eq!(
            view["summary"]["usable_point_count"], case["usable_point_count"],
            "{name}: usable"
        );
        assert_eq!(
            view["summary"]["excluded_point_count"], case["excluded_point_count"],
            "{name}: excluded"
        );
        assert_eq!(
            view["summary"]["usable_point_count"].as_u64().unwrap()
                + view["summary"]["excluded_point_count"].as_u64().unwrap(),
            view["summary"]["point_count"].as_u64().unwrap(),
            "{name}: counts never exceed the Raw total"
        );
    }
}
