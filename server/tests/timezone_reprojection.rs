mod support;

use axum::http::StatusCode;
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db, processing,
};
use serde_json::json;
use support::{ndjson, read, record, upload};
use uuid::Uuid;

/// The public Daily View/status seam proves timezone work reuses published
/// activity even if a newer Batch is waiting.  The direct persisted checks are
/// provenance assertions, not a private-worker test seam.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn timezone_reprojects_published_manifest_while_newer_raw_input_is_pending() {
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
    let first = ndjson(&[
        record(support::at(23, 50), 10.77, 106.70),
        record(support::at(23, 55), 10.771, 106.701),
    ]);
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &first).await.0,
        StatusCode::OK
    );
    assert!(processing::process_next(&pool).await.unwrap());
    let old = read(
        &router,
        &format!("/api/v1/devices/{}/days/2026-10-05", device.id),
    )
    .await;
    let old_manifest = old["provenance"]["manifest_version"].clone();
    let old_generation = old["provenance"]["source_raw_generation"].clone();

    let late = format!(
        "{}\n",
        json!({"ts_ms": support::at(23, 56), "lat": 10.772, "lon": 106.702, "fix_quality": 1, "satellites": 8})
    );
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &late).await.0,
        StatusCode::OK
    );
    sqlx::query("UPDATE users SET timezone='Asia/Ho_Chi_Minh' WHERE id=$1")
        .bind(owner.id)
        .execute(&pool)
        .await
        .unwrap();

    let path = format!("/api/v1/devices/{}/days/2026-10-06", device.id);
    assert_eq!(
        read(&router, &format!("{path}/status")).await["state"],
        "queued"
    );
    assert!(processing::process_next(&pool).await.unwrap());
    let projected = read(&router, &path).await;
    assert_eq!(projected["processing_state"], "processed");
    assert_eq!(projected["summary"]["point_count"], 3);
    assert_eq!(
        projected["summary"]["usable_point_count"], 2,
        "quality counts come from the pinned activity source, not a new quality pass"
    );
    assert_eq!(
        projected["provenance"]["reducer_version"],
        old["provenance"]["reducer_version"]
    );
    assert_eq!(projected["timezone"], "Asia/Ho_Chi_Minh");
    assert_eq!(projected["provenance"]["manifest_version"], old_manifest);
    assert_eq!(
        projected["provenance"]["source_raw_generation"],
        old_generation
    );
    assert_eq!(projected["processing"]["data_freshness"], "stale_source");
    assert_eq!(projected["processing"]["input_generation"], 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM activity_revisions WHERE device_id=$1")
            .bind(device.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1,
        "projection-only work must not create or reprocess activity"
    );
}
