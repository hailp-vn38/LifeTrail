mod support;
use axum::http::StatusCode;
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db, processing,
};
use serde_json::json;
use support::{read, upload};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn stop_config_change_rejects_captured_candidate_and_retains_previous_publication() {
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
    let body: String = (0..=3).map(|i| format!("{}\n", json!({"ts_ms":1791158400000_i64+i*60000,"lat":10.77,"lon":106.7,"fix_quality":1,"satellites":8}))).collect();
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &body).await.0,
        StatusCode::OK
    );
    assert!(processing::process_next(&pool).await.unwrap());
    let path = format!("/api/v1/devices/{}/days/2026-10-05", device.id);
    let original = read(&router, &path).await;
    assert_eq!(original["summary"]["stop_count"], 1);
    processing::queue_day(&pool, device.id, "2026-10-05".parse().unwrap())
        .await
        .unwrap();
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK activity_manifests IN SHARE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let worker_pool = pool.clone();
    let worker = tokio::spawn(async move { processing::process_next(&worker_pool).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE 'INSERT INTO activity_manifests%')").fetch_one(&pool).await.unwrap();
            if blocked { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    sqlx::query("UPDATE device_processing_control SET stop_min_duration_s=600 WHERE device_id=$1")
        .bind(device.id)
        .execute(&pool)
        .await
        .unwrap();
    lock.commit().await.unwrap();
    assert!(worker.await.unwrap().unwrap());
    let stale = read(&router, &path).await;
    assert_eq!(
        stale["processing"]["published_revision"],
        original["processing"]["published_revision"]
    );
    assert_eq!(stale["processing"]["data_freshness"], "stale");
    assert_eq!(stale["processing"]["state"], "queued");
    assert!(processing::process_next(&pool).await.unwrap());
    let current = read(&router, &path).await;
    assert_eq!(current["processing"]["data_freshness"], "current");
    assert_eq!(current["summary"]["stop_count"], 0);
    assert_eq!(current["summary"]["point_count"], 4);
    assert_eq!(current["provenance"]["source_raw_generation"], 1);
    assert_eq!(current["provenance"]["processing_target_generation"], 2);
}
