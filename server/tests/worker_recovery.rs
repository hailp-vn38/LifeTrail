mod support;

use axum::http::StatusCode;
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db, processing,
};
use support::{at, ndjson, read, record, upload};
use uuid::Uuid;

/// Lease expiry is a controlled database transition, not a wall-clock sleep:
/// reclaiming it must issue a new fencing token and leave one active snapshot.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn expired_worker_is_reclaimed_with_a_new_fencing_attempt() {
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
    let body = ndjson(&[
        record(at(9, 0), 10.77, 106.70),
        record(at(9, 1), 10.771, 106.701),
    ]);
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &body).await.0,
        StatusCode::OK
    );
    assert!(processing::process_next(&pool).await.unwrap());

    processing::queue_day(&pool, device.id, "2026-10-05".parse().unwrap())
        .await
        .unwrap();
    // Model a process that died after claiming token 41.  No elapsed time is
    // required to exercise reclaim; PostgreSQL's authoritative lease decides.
    sqlx::query("UPDATE device_processing_control SET fencing_token=41 WHERE device_id=$1")
        .bind(device.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE processing_jobs SET state='running',fencing_token=41,lease_until=now()-interval '1 second' WHERE device_id=$1")
        .bind(device.id)
        .execute(&pool)
        .await
        .unwrap();

    assert!(processing::process_next(&pool).await.unwrap());
    let status = read(
        &router,
        &format!("/api/v1/devices/{}/days/2026-10-05/status", device.id),
    )
    .await;
    assert_eq!(status["state"], "idle");
    assert_eq!(status["data_freshness"], "current");
    let (token, attempts): (i64, i64) = sqlx::query_as(
        "SELECT fencing_token,attempt_count FROM processing_jobs WHERE device_id=$1",
    )
    .bind(device.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(token, 42);
    assert!(attempts >= 2);
    let outcome: String = sqlx::query_scalar(
        "SELECT outcome FROM processing_attempts WHERE device_id=$1 AND fencing_token=42",
    )
    .bind(device.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(outcome, "activated");
    let measurements: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM processing_storage_measurements WHERE device_id=$1 AND fencing_token=42",
    )
    .bind(device.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(measurements, 1);
}
