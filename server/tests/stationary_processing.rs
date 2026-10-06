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
async fn stationary_upload_publishes_open_stop_across_midnight_without_changing_raw() {
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
    // 23:50–00:20 local; keep each observation interval below the absence threshold.
    let start = chrono::DateTime::parse_from_rfc3339("2026-10-05T16:50:00Z")
        .unwrap()
        .timestamp_millis();
    let body: String = (0..=30).map(|i| format!("{}\n", json!({"ts_ms":start+i*60000,"lat":10.77+if i%2==0 {0.00002} else {-0.00002},"lon":106.7,"fix_quality":1,"satellites":8,"hdop":1.2,"speed_mps":2.0}))).collect();
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &body).await.0,
        StatusCode::OK
    );
    let raw: Vec<String> =
        sqlx::query_scalar("SELECT row_to_json(g)::text FROM gps_points g ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(processing::process_next(&pool).await.unwrap());
    let path = format!("/api/v1/devices/{}/days/", device.id);
    let first = read(&router, &format!("{path}2026-10-05")).await;
    let second = read(&router, &format!("{path}2026-10-06")).await;
    assert_eq!(first["processing_state"], "processed");
    assert_eq!(first["summary"]["stop_count"], 1);
    assert_eq!(first["summary"]["stop_duration_s"], 600);
    assert_eq!(second["summary"]["stop_duration_s"], 1200);
    let stop = &first["timeline"][0];
    assert_eq!(stop["kind"], "stop");
    assert_eq!(stop["id"], second["timeline"][0]["id"]);
    assert_eq!(
        stop["activity_revision"],
        second["timeline"][0]["activity_revision"]
    );
    assert_eq!(stop["observed_duration_s"], 1800);
    assert_eq!(stop["observed_from_at"], "2026-10-05T16:50:00Z");
    assert_eq!(stop["observed_until_at"], "2026-10-05T17:20:00Z");
    assert_eq!(stop["start_boundary"], "open");
    assert_eq!(stop["end_boundary"], "open");
    assert!(stop["actual_start_at"].is_null());
    assert!(stop["actual_end_at"].is_null());
    assert!(stop["full_duration_s"].is_null());
    assert_eq!(stop["source_record_count"], 31);
    assert!(stop["radius_m"].as_f64().unwrap() > 0.0);
    assert_eq!(first["evidence_state"], "sufficient");
    let after: Vec<String> =
        sqlx::query_scalar("SELECT row_to_json(g)::text FROM gps_points g ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(raw, after);
    assert!(
        sqlx::query("UPDATE activity_revisions SET body='{}'")
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM activity_revisions")
            .execute(&pool)
            .await
            .is_err()
    );
    // Reprocessing retains immutable source artifacts and gives a new local event identity.
    processing::queue_day(&pool, device.id, "2026-10-05".parse().unwrap())
        .await
        .unwrap();
    assert!(processing::process_next(&pool).await.unwrap());
    let replacement = read(&router, &format!("{path}2026-10-05")).await;
    assert_ne!(replacement["timeline"][0]["id"], stop["id"]);
    let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM activity_revisions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(retained, 2);
    // Later reliable observations close both boundaries in a new revision.
    let later: String = [start - 60000, start + 1860000]
        .into_iter()
        .map(|ts| {
            format!(
                "{}\n",
                json!({"ts_ms":ts,"lat":10.78,"lon":106.7,"fix_quality":1,"satellites":8})
            )
        })
        .collect();
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &later).await.0,
        StatusCode::OK
    );
    assert!(processing::process_next(&pool).await.unwrap());
    let closed = read(&router, &format!("{path}2026-10-05")).await;
    assert_eq!(closed["timeline"][0]["start_boundary"], "confirmed");
    assert_eq!(closed["timeline"][0]["end_boundary"], "confirmed");
    assert_eq!(
        closed["timeline"][0]["actual_start_at"],
        "2026-10-05T16:50:00Z"
    );
    assert_eq!(
        closed["timeline"][0]["actual_end_at"],
        "2026-10-05T17:20:00Z"
    );
    assert_eq!(closed["timeline"][0]["full_duration_s"], 1800);
    let stored: String = sqlx::query_scalar("SELECT body::text FROM daily_snapshots WHERE id=$1")
        .bind(
            first["processing"]["published_revision"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap(),
        )
        .fetch_one(&pool)
        .await
        .unwrap();
    let stored: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(stored["timeline"][0], *stop);
    // A Stop ending exactly at midnight has no positive overlap in the next day.
    let token2 = generate_device_token();
    let device2 = db::create_device(&pool, owner.id, "Midnight", &token2)
        .await
        .unwrap();
    let edge = chrono::DateTime::parse_from_rfc3339("2026-10-05T16:57:00Z")
        .unwrap()
        .timestamp_millis();
    let body: String = (0..=3)
        .map(|i| {
            format!(
                "{}\n",
                json!({"ts_ms":edge+i*60000,"lat":10.77,"lon":106.7,"fix_quality":1,"satellites":8})
            )
        })
        .collect();
    assert_eq!(
        upload(&router, &token2, Uuid::new_v4(), &body).await.0,
        StatusCode::OK
    );
    assert!(processing::process_next(&pool).await.unwrap());
    let next = read(
        &router,
        &format!("/api/v1/devices/{}/days/2026-10-06", device2.id),
    )
    .await;
    assert_eq!(next["summary"]["point_count"], 1);
    assert_eq!(next["summary"]["stop_count"], 0);
    assert_eq!(next["summary"]["stop_duration_s"], 0);
    assert_eq!(next["timeline"], json!([]));
}
