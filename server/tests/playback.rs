//! Playback resource and display-schema drift seams (Phase 2 map performance).
//!
//! The Daily Snapshot is the display projection; the Playback resource is the
//! canonical source of geometry and progress. These tests pin the lazy-load
//! contract (404 for no publication, 410 for a stale manifest pin) and the
//! idempotent projection-only requeue seam.
mod support;

use axum::http::StatusCode;
use lifetrail_server::processing;
use serde_json::json;
use support::{at, record, scenarios};

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn playback_serves_canonical_geometry_while_daily_serves_display() {
    let published = scenarios::publish(&moving_day()).await;
    let playback = published.playback("2026-10-05").await;
    assert_eq!(playback["date"], "2026-10-05");
    assert_eq!(playback["projection_schema_version"], 2);
    let parts = playback["route_parts"].as_array().unwrap();
    assert!(!parts.is_empty());
    for part in parts {
        // Canonical resource carries the full geometry contract.
        assert!(part["geometry"]["coordinates"].is_array());
        assert!(part["vertex_distance_m"].is_array());
        assert!(part["progress_anchors"].is_array());
        assert!(part.get("display_geometry").is_none());
        support::assert_route_part(part, "playback part", 0.01);
    }
    // The Daily Snapshot carries the display shape instead.
    for part in published.view["route_parts"].as_array().unwrap() {
        assert!(part.get("geometry").is_none());
        support::assert_display_route_part(part, "daily part");
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn playback_is_404_without_a_publication_and_410_for_a_stale_pin() {
    let published = scenarios::publish(&moving_day()).await;
    let app = published.router.clone();
    // A day with no publication is not found, matching the Daily View.
    let missing = get(
        app.clone(),
        &format!(
            "/api/v1/devices/{}/days/2026-10-04/playback",
            published.device_id
        ),
    )
    .await;
    assert_eq!(missing, StatusCode::NOT_FOUND);
    // A pin that no longer exists forces the client to drop its cache.
    let stale_pin = uuid::Uuid::new_v4();
    let gone = get(
        app,
        &format!(
            "/api/v1/devices/{}/days/2026-10-05/playback?manifest_version={stale_pin}",
            published.device_id
        ),
    )
    .await;
    assert_eq!(gone, StatusCode::GONE);
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn schema_drift_is_flagged_and_requeued_once() {
    let published = scenarios::publish(&moving_day()).await;
    let pool = published.pool.clone();
    let device = published.device_id;
    // Simulate an old-schema publication by rewriting the published provenance.
    // daily_snapshots is immutable by trigger; lift it only inside this test.
    rewrite_schema_to_v1(&pool, device).await;

    // The view stays usable and is flagged stale.
    let view = published.reread("2026-10-05").await;
    assert_eq!(view["processing"]["display_projection_stale"], json!(true));
    assert_eq!(view["processing_state"], "processed");

    // The lazy requeue is idempotent: a second stale read adds no new request.
    let _ = published.reread("2026-10-05").await;
    let queued: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM projection_requeues WHERE device_id=$1 AND local_date='2026-10-05'",
    )
    .bind(device)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        queued, 1,
        "a repeated stale read must not duplicate the request"
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn backfill_enqueues_only_stale_publications() {
    let published = scenarios::publish(&moving_day()).await;
    let pool = published.pool.clone();
    // The fresh v2 publication is already current: nothing to backfill.
    let accepted = processing::backfill_projection_schema(&pool).await.unwrap();
    assert_eq!(accepted, 0);
    // An old-schema publication becomes eligible, exactly once.
    rewrite_schema_to_v1(&pool, published.device_id).await;
    assert_eq!(
        processing::backfill_projection_schema(&pool).await.unwrap(),
        1
    );
    assert_eq!(
        processing::backfill_projection_schema(&pool).await.unwrap(),
        0
    );
}

/// Force the immutable published snapshot to look like it predates schema v2.
/// The immutability trigger is disabled only for this statement.
async fn rewrite_schema_to_v1(pool: &sqlx::PgPool, device: uuid::Uuid) {
    sqlx::query("ALTER TABLE daily_snapshots DISABLE TRIGGER immutable_daily_snapshot")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE daily_snapshots SET body=jsonb_set(body,'{provenance,projection_schema_version}','1') \
         WHERE device_id=$1",
    )
    .bind(device)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("ALTER TABLE daily_snapshots ENABLE TRIGGER immutable_daily_snapshot")
        .execute(pool)
        .await
        .unwrap();
}

fn moving_day() -> Vec<serde_json::Value> {
    // Two observed legs with a Stop in between, all on the Owner-local day.
    vec![
        record(at(8, 0), 10.0, 106.0),
        record(at(8, 1), 10.001, 106.0),
        record(at(8, 2), 10.002, 106.0),
        record(at(9, 0), 10.003, 106.0),
        record(at(9, 1), 10.004, 106.0),
        record(at(9, 2), 10.005, 106.0),
    ]
}

async fn get(app: axum::Router, path: &str) -> StatusCode {
    use axum::body::Body;
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;
    let response = app
        .oneshot(axum::http::Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let _ = response.into_body().collect().await;
    status
}
