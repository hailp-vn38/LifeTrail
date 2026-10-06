use std::env;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt as _;
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db,
};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt as _;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires real PostgreSQL/PostGIS; use docker compose --profile test run --rm server-tests"]
async fn daily_view_uses_owner_day_boundaries_orders_same_timestamps_and_projects_raw_gps() {
    let pool = test_pool().await;
    let (device_id, _) = create_device(&pool, "America/New_York").await;
    seed_route_points(&pool, device_id).await;
    let app = app::router(
        AppState { db: pool.clone() },
        Some("missing-static-dir".into()),
    );
    assert_dst_raw_projection(app.clone(), device_id).await;
    assert_single_point_distance(app.clone(), device_id).await;
    assert_empty_daily_view(app.clone(), device_id).await;
    assert_missing_device(app).await;
}

async fn seed_route_points(pool: &PgPool, device_id: Uuid) {
    let batches = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    for batch_id in batches {
        create_batch(pool, device_id, batch_id).await;
    }
    for (batch_id, recorded_at, lon, fix_quality) in [
        (batches[0], "2026-03-08 04:59:59+00", -1.0, 1),
        (batches[0], "2026-03-08 05:00:00+00", 0.0, 1),
        (batches[1], "2026-03-08 05:00:00+00", 1.0, 1),
        (batches[2], "2026-03-08 12:00:00+00", 1.5, 0),
        (batches[2], "2026-03-09 03:59:59+00", 2.0, 1),
        (batches[2], "2026-03-09 04:00:00+00", 3.0, 1),
        (batches[2], "2026-03-10 05:00:00+00", 20.0, 1),
    ] {
        insert_point(
            pool,
            device_id,
            batch_id,
            recorded_at,
            0.0,
            lon,
            fix_quality,
        )
        .await;
    }
}

async fn assert_dst_raw_projection(app: axum::Router, device_id: Uuid) {
    let response = get_daily_view(app, device_id, "2026-03-08").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["device_id"], device_id.to_string());
    assert_eq!(body["timezone"], "America/New_York");
    assert_eq!(body["processing_state"], "raw");
    assert_eq!(body["summary"]["point_count"], 3);
    assert_eq!(body["summary"]["duration_s"], 82_799);
    assert_eq!(body["summary"]["first_fix_at"], "2026-03-08T05:00:00Z");
    assert_eq!(body["summary"]["last_fix_at"], "2026-03-09T03:59:59Z");
    assert_distance(&body, 222_389.853, 0.01);
    assert_eq!(
        body["route"]["geometry"]["coordinates"],
        serde_json::json!([[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]])
    );
    // One timestamp per coordinate, index-aligned, from the same ordered points
    // (note the duplicated 05:00:00Z timestamp is preserved, not deduplicated).
    assert_eq!(
        body["route"]["properties"]["timestamps"],
        serde_json::json!([
            "2026-03-08T05:00:00Z",
            "2026-03-08T05:00:00Z",
            "2026-03-09T03:59:59Z"
        ])
    );
    assert_eq!(
        body["start"]["properties"]["recorded_at"],
        "2026-03-08T05:00:00Z"
    );
    assert_eq!(
        body["end"]["properties"]["recorded_at"],
        "2026-03-09T03:59:59Z"
    );
    assert!(body.get("raw_data_complete").is_none());
    assert!(body.get("trips").is_none());
    assert!(body.get("stops").is_none());
    assert!(body.get("events").is_none());
}

async fn assert_single_point_distance(app: axum::Router, device_id: Uuid) {
    let single = get_daily_view(app, device_id, "2026-03-10").await;
    assert_eq!(single.status(), StatusCode::OK);
    let single_body = response_json(single).await;
    assert_eq!(single_body["summary"]["point_count"], 1);
    assert_distance(&single_body, 0.0, 0.001);
    assert!(single_body["route"].is_null());
}

async fn assert_empty_daily_view(app: axum::Router, device_id: Uuid) {
    let empty = get_daily_view(app, device_id, "2026-03-11").await;
    assert_eq!(empty.status(), StatusCode::OK);
    let empty_body = response_json(empty).await;
    assert_eq!(empty_body["summary"]["point_count"], 0);
    assert_distance(&empty_body, 0.0, 0.001);
    assert_eq!(empty_body["summary"]["duration_s"], 0);
    assert!(empty_body["route"].is_null());
    assert!(empty_body["start"].is_null());
    assert!(empty_body["end"].is_null());
    assert!(empty_body["summary"]["first_fix_at"].is_null());
    assert!(empty_body["summary"]["last_fix_at"].is_null());
}

async fn assert_missing_device(app: axum::Router) {
    let missing = get_daily_view(app, Uuid::new_v4(), "2026-03-08").await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

async fn test_pool() -> PgPool {
    let database_url = env::var("LT_TEST_DATABASE_URL").expect("LT_TEST_DATABASE_URL must be set");
    let pool = db::connect(&database_url)
        .await
        .expect("connect test database");
    db::migrate(&pool).await.expect("run migrations");
    sqlx::query("TRUNCATE users CASCADE")
        .execute(&pool)
        .await
        .expect("clear isolated test database");
    pool
}

async fn create_device(pool: &PgPool, timezone: &str) -> (Uuid, String) {
    let owner = db::create_owner(pool, "LifeTrail Owner", timezone)
        .await
        .expect("create owner");
    let token = generate_device_token();
    let device = db::create_device(pool, owner.id, "GPS Recorder", &token)
        .await
        .expect("create device");
    (device.id, token)
}

async fn create_batch(pool: &PgPool, device_id: Uuid, batch_id: Uuid) {
    sqlx::query(
        "INSERT INTO ingest_batches \
         (device_id, batch_id, schema_name, content_sha256, byte_length, record_count, first_ts_ms, last_ts_ms) \
         VALUES ($1, $2, 'gps/1', decode(repeat('00', 32), 'hex'), 1, 1, 1, 1)",
    )
    .bind(device_id)
    .bind(batch_id)
    .execute(pool)
    .await
    .expect("create batch");
}

async fn insert_point(
    pool: &PgPool,
    device_id: Uuid,
    batch_id: Uuid,
    recorded_at: &str,
    lat: f64,
    lon: f64,
    fix_quality: i64,
) {
    sqlx::query(
        "INSERT INTO gps_points \
         (device_id, batch_id, recorded_at, lat, lon, fix_quality, satellites, geometry) \
         VALUES ($1, $2, $3::timestamptz, $4, $5, $6, 8, ST_SetSRID(ST_MakePoint($5, $4), 4326))",
    )
    .bind(device_id)
    .bind(batch_id)
    .bind(recorded_at)
    .bind(lat)
    .bind(lon)
    .bind(fix_quality)
    .execute(pool)
    .await
    .expect("insert raw GPS");
}

async fn get_daily_view(
    app: axum::Router,
    device_id: Uuid,
    date: &str,
) -> axum::response::Response {
    app.oneshot(
        Request::get(format!("/api/v1/devices/{device_id}/days/{date}"))
            .body(Body::empty())
            .expect("construct Daily View request"),
    )
    .await
    .expect("Daily View response")
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).expect("JSON response")
}

fn assert_distance(body: &Value, expected: f64, tolerance: f64) {
    let actual = body["summary"]["distance_m"]
        .as_f64()
        .expect("distance_m number");
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected distance {expected} ± {tolerance}, got {actual}"
    );
}
