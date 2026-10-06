//! Stale generation/fencing and failed candidates must retain the good snapshot.
mod support;
use lifetrail_server::processing;
use support::{at, ndjson, record, scenarios, upload};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn captured_generation_and_fencing_races_never_publish() {
    for race in ["generation", "fencing"] {
        let published = scenarios::publish(&[
            record(at(8, 0), 10.77, 106.7),
            record(at(8, 1), 10.771, 106.7),
        ])
        .await;
        let original = published.view["processing"]["published_revision"].clone();
        processing::queue_day(
            &published.pool,
            published.device_id,
            "2026-10-05".parse().unwrap(),
        )
        .await
        .unwrap();
        // Block staging after capture without relying on worker timing.
        let mut lock = published.pool.begin().await.unwrap();
        sqlx::query("LOCK activity_manifests IN SHARE MODE")
            .execute(&mut *lock)
            .await
            .unwrap();
        let pool = published.pool.clone();
        let worker = tokio::spawn(async move { processing::process_next(&pool).await });
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE 'INSERT INTO activity_manifests%')").fetch_one(&published.pool).await.unwrap();
                if waiting { break; }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }).await.unwrap();
        if race == "generation" {
            let late = ndjson(&[record(at(8, 0) + 30000, 10.7705, 106.7)]);
            assert_eq!(
                upload(&published.router, &published.token, Uuid::new_v4(), &late)
                    .await
                    .0,
                axum::http::StatusCode::OK,
            );
        } else {
            let mut tx = published.pool.begin().await.unwrap();
            sqlx::query("UPDATE device_processing_control SET fencing_token=fencing_token+1 WHERE device_id=$1").bind(published.device_id).execute(&mut *tx).await.unwrap();
            sqlx::query("UPDATE processing_jobs SET fencing_token=fencing_token+1,state='queued',lease_until=NULL WHERE device_id=$1").bind(published.device_id).execute(&mut *tx).await.unwrap();
            tx.commit().await.unwrap();
        }
        lock.commit().await.unwrap();
        assert!(worker.await.unwrap().unwrap());
        let retained = published.reread("2026-10-05").await;
        assert_eq!(
            retained["processing"]["published_revision"], original,
            "{race}"
        );
        assert_eq!(retained["processing"]["state"], "queued");
        assert!(processing::process_next(&published.pool).await.unwrap());
        assert_ne!(
            published.reread("2026-10-05").await["processing"]["published_revision"],
            original
        );
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn failed_snapshot_staging_keeps_processed_view() {
    let published = scenarios::publish(&[
        record(at(8, 0), 10.77, 106.7),
        record(at(8, 1), 10.771, 106.7),
    ])
    .await;
    processing::queue_day(
        &published.pool,
        published.device_id,
        "2026-10-05".parse().unwrap(),
    )
    .await
    .unwrap();
    // Existing rows remain valid; only a newly staged snapshot is rejected.
    sqlx::query(
        "ALTER TABLE daily_snapshots ADD CONSTRAINT test_reject_candidate CHECK (false) NOT VALID",
    )
    .execute(&published.pool)
    .await
    .unwrap();
    let result = processing::process_next(&published.pool).await;
    sqlx::query("ALTER TABLE daily_snapshots DROP CONSTRAINT test_reject_candidate")
        .execute(&published.pool)
        .await
        .unwrap();
    assert!(result.is_err());
    let retained = published.reread("2026-10-05").await;
    assert_eq!(retained["processing_state"], "processed");
    assert_eq!(retained["processing"]["state"], "failed");
    assert_eq!(
        retained["processing"]["published_revision"],
        published.view["processing"]["published_revision"]
    );
    assert_eq!(retained["route_parts"], published.view["route_parts"]);
}
