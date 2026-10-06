mod support;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db, processing,
};
use serde_json::json;
use support::{read, upload};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn sparse_upload_publishes_truthful_snapshot_and_refreshes_unresolved_late_input() {
    let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("TRUNCATE users CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    let owner = db::create_owner(&pool, "Owner", "Asia/Ho_Chi_Minh")
        .await
        .unwrap();
    let token = generate_device_token();
    let device = db::create_device(&pool, owner.id, "Device", &token)
        .await
        .unwrap();
    let router = app::router(AppState { db: pool.clone() }, None);
    let batch = Uuid::new_v4();
    let record =
        json!({"ts_ms":1791158400000_i64,"lat":10.77,"lon":106.7,"fix_quality":0,"satellites":0});
    let body = format!("{record}\n");
    assert_eq!(
        upload(&router, &token, batch, &body).await.0,
        StatusCode::OK
    );
    let path = format!("/api/v1/devices/{}/days/2026-10-05", device.id);
    let queued = read(&router, &format!("{path}/status")).await;
    assert_eq!(queued["input_generation"], 1);
    assert_eq!(queued["state"], "queued");
    assert_eq!(queued["data_freshness"], "unavailable");
    assert_eq!(
        upload(&router, &token, batch, &body).await.1["duplicate"],
        true
    );
    assert_eq!(
        read(&router, &format!("{path}/status")).await["input_generation"],
        1
    );
    assert_eq!(
        upload(&router, "invalid-token", Uuid::new_v4(), &body)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), "{}\n").await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let conflicting = format!(
        "{}\n",
        json!({"ts_ms":1791158400000_i64,"lat":11.0,"lon":106.7,"fix_quality":0,"satellites":0})
    );
    assert_eq!(
        upload(&router, &token, batch, &conflicting).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        read(&router, &format!("{path}/status")).await["input_generation"],
        1
    );
    assert!(processing::process_next(&pool).await.unwrap());
    let published = read(&router, &path).await;
    assert_eq!(published["processing_state"], "processed");
    assert_eq!(published["processing"]["state"], "idle");
    assert_eq!(published["processing"]["data_freshness"], "current");
    assert_eq!(published["evidence_state"], "insufficient");
    assert_eq!(published["summary"]["point_count"], 1);
    assert_eq!(published["summary"]["usable_point_count"], 0);
    assert_eq!(published["summary"]["first_fix_at"], "2026-10-05T00:00:00Z");
    assert_eq!(published["route_parts"], json!([]));
    assert_eq!(published["timeline"], json!([]));
    assert!(published["route"].is_null());
    assert!(published["provenance"]["manifest_version"].is_string());
    let revision = published["processing"]["published_revision"].clone();
    let raw_count: i64 = sqlx::query_scalar("SELECT count(*) FROM gps_points")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(raw_count, 1);
    let late = format!(
        "{}\n",
        json!({"ts_ms":1791158460000_i64,"lat":10.78,"lon":106.71,"fix_quality":1,"satellites":8})
    );
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &late).await.0,
        StatusCode::OK
    );
    assert!(processing::process_next(&pool).await.unwrap());
    let stale = read(&router, &path).await;
    assert_ne!(stale["processing"]["published_revision"], revision);
    assert_eq!(stale["processing"]["data_freshness"], "current");
    assert_eq!(stale["summary"]["point_count"], 2);
    assert!(stale["processing"]["deferred_reason"].is_null());
    assert_eq!(stale["timeline"], json!([]));
    assert_eq!(
        stale["evidence_holes"][0]["reason"],
        "unusable_observations"
    );
    processing::queue_day(&pool, device.id, "2026-10-06".parse().unwrap())
        .await
        .unwrap();
    assert!(processing::process_next(&pool).await.unwrap());
    let empty = read(&router, &path.replace("2026-10-05", "2026-10-06")).await;
    assert_eq!(empty["processing_state"], "processed");
    assert_eq!(empty["summary"]["point_count"], 0);
    assert!(empty["summary"]["first_fix_at"].is_null());
    // A new target identity makes an existing publication stale, even without a generation change.
    sqlx::query("UPDATE device_processing_control SET target_id='sparse-v2' WHERE device_id=$1")
        .bind(device.id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        read(&router, &path.replace("2026-10-05", "2026-10-06")).await["processing"]["data_freshness"],
        "stale"
    );
    // Freeze candidate staging so operator requests arrive after capture, before activation.
    processing::queue_day(&pool, device.id, "2026-10-07".parse().unwrap())
        .await
        .unwrap();
    let mut staging_lock = pool.begin().await.unwrap();
    sqlx::query("LOCK activity_manifests IN SHARE MODE")
        .execute(&mut *staging_lock)
        .await
        .unwrap();
    let worker_pool = pool.clone();
    let worker = tokio::spawn(async move { processing::process_next(&worker_pool).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE 'INSERT INTO activity_manifests%')")
                .fetch_one(&pool).await.unwrap();
            if blocked { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    processing::queue_day(&pool, device.id, "2026-10-08".parse().unwrap())
        .await
        .unwrap();
    processing::queue_day(&pool, device.id, "2026-10-07".parse().unwrap())
        .await
        .unwrap();
    staging_lock.commit().await.unwrap();
    assert!(worker.await.unwrap().unwrap());
    assert_eq!(
        read(&router, &path.replace("2026-10-05", "2026-10-08")).await["processing"]["state"],
        "queued"
    );
    assert!(processing::process_next(&pool).await.unwrap());
    for day in ["2026-10-07", "2026-10-08"] {
        let result = read(&router, &path.replace("2026-10-05", day)).await;
        assert_eq!(result["processing_state"], "processed");
        assert_eq!(result["processing"]["data_freshness"], "current");
    }
    assert!(
        sqlx::query("UPDATE daily_snapshots SET body='{}'")
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM activity_manifests")
            .execute(&pool)
            .await
            .is_err()
    );
    sqlx::query("UPDATE users SET timezone='UTC' WHERE id=$1")
        .bind(owner.id)
        .execute(&pool)
        .await
        .unwrap();
    let changed_zone = read(&router, &path).await;
    assert_eq!(changed_zone["timezone"], "UTC");
    assert_eq!(changed_zone["processing_state"], "raw");
    assert_eq!(changed_zone["processing"]["data_freshness"], "unavailable");
    assert_eq!(changed_zone["processing"]["timezone_generation"], 1);
    let response = router
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/devices/{}/days/2026-10-05/status",
                    Uuid::new_v4()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
