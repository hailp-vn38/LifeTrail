//! Deterministic scenario fixtures and the end-to-end publish harness.
//!
//! Every assertion in the quality, gap and hole integration files reads the
//! public Daily View after a real `gps/1` upload, real PostGIS persistence and
//! the real processing worker. Raw GPS is compared before and after processing,
//! never through a private classifier, so this harness owns that single seam.
//!
//! Each integration file compiles this module into its own binary and uses only
//! part of it, so unused helpers are expected rather than dead.
#![allow(dead_code)]
use super::{at, ndjson, read, upload};
use axum::http::StatusCode;
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db, processing,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

/// Meters per degree of latitude on the sphere used by the server's geodesic
/// helper. Independent of the implementation under test.
pub const METER_PER_DEGREE: f64 = 111_194.926_644_558_74;
/// 0.0005 degrees of latitude: 55.6 m, one fixture leg and beyond the Stop radius.
pub const STEP: f64 = 0.0005;
pub const TOLERANCE_M: f64 = 0.5;
/// Scenario definitions, each a day of Raw GPS plus its disclosed evidence.
const FIXTURES: &str = include_str!("../fixtures/quality_gaps.json");

/// Every deterministic quality, gap and hole scenario, including a genuine
/// timestamp absence, continuous low-quality observations and an impossible
/// jump.
pub fn cases() -> Vec<Value> {
    serde_json::from_str(FIXTURES).unwrap()
}

pub fn case(name: &str) -> Value {
    cases()
        .into_iter()
        .find(|case| case["name"] == name)
        .unwrap_or_else(|| panic!("fixture case {name}"))
}

/// Fixture records as exact gps/1 rows, relative to 08:00 UTC.
pub fn records(case: &Value) -> Vec<Value> {
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

/// A processed Owner-local day plus the handles a test needs to keep asserting
/// against the same database and Device.
pub struct Published {
    pub router: axum::Router,
    pub view: Value,
    pub device_id: Uuid,
    pub pool: PgPool,
}

impl Published {
    /// Re-read the same day, for staleness and rebuild assertions.
    pub async fn reread(&self, date: &str) -> Value {
        read(
            &self.router,
            &format!("/api/v1/devices/{}/days/{date}", self.device_id),
        )
        .await
    }
}

/// Provision one Owner and Device, upload `records` and run the worker once.
pub async fn publish(records: &[Value]) -> Published {
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
    Published {
        router,
        view,
        device_id: device.id,
        pool,
    }
}

/// Every persisted GPS Record, for the Raw immutability invariant.
pub async fn raw_rows(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar("SELECT row_to_json(g)::text FROM gps_points g ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
}

pub fn kinds(view: &Value) -> Vec<String> {
    view["timeline"]
        .as_array()
        .expect("timeline")
        .iter()
        .map(|item| item["kind"].as_str().unwrap().to_owned())
        .collect()
}

pub fn trips(view: &Value) -> Vec<&Value> {
    view["timeline"]
        .as_array()
        .expect("timeline")
        .iter()
        .filter(|item| item["kind"] == "trip")
        .collect()
}

/// Published GPS Gap bounds, for comparison against a fixture's declared Gaps.
pub fn published_gaps(view: &Value) -> Vec<[String; 2]> {
    view["timeline"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["kind"] == "gap")
        .map(|item| {
            [
                item["observed_from_at"].as_str().unwrap().to_owned(),
                item["observed_until_at"].as_str().unwrap().to_owned(),
            ]
        })
        .collect()
}

/// Published Evidence Holes as `[reason, from, until, record_count]` tuples.
pub fn published_holes(view: &Value) -> Vec<Value> {
    view["evidence_holes"]
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
        .collect()
}

/// Published Trip observed bounds, for comparison against a fixture's declared Trips.
pub fn published_trip_bounds(view: &Value) -> Vec<[String; 2]> {
    trips(view)
        .iter()
        .map(|trip| {
            [
                trip["observed_from_at"].as_str().unwrap().to_owned(),
                trip["observed_until_at"].as_str().unwrap().to_owned(),
            ]
        })
        .collect()
}

/// Sum of every published Route Part length in the day.
pub fn published_distance_m(view: &Value) -> f64 {
    view["route_parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| part["distance_m"].as_f64().unwrap())
        .sum()
}
