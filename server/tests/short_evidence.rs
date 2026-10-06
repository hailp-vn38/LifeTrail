//! Public ingestion-to-publication regressions for short rejected observations.
mod support;
use serde_json::json;
use support::{at, record, scenarios};

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn alternating_hdop_keeps_one_trip_without_micro_holes() {
    let records: Vec<_> = (0..178)
        .map(|i| {
            let mut point = record(at(8, 0) + i * 1000, 10.77 + i as f64 * 0.000012, 106.7);
            point["hdop"] = json!([3.5, 4.5, 5.5, 6.5][i as usize % 4]);
            point
        })
        .collect();
    let view = scenarios::publish(&records).await.view;
    assert_eq!(
        view["summary"]["trip_count"], 1,
        "short quality failures must not fragment a Trip"
    );
    assert_eq!(
        view["evidence_holes"],
        json!([]),
        "neighboring observations support the short intervals"
    );
    assert_eq!(view["summary"]["low_quality_point_count"], 88);
    assert_eq!(view["summary"]["usable_point_count"], 90);
    let trip = &view["timeline"][0];
    assert_eq!(trip["source_record_count"], 178);
    assert_eq!(trip["usable_record_count"], 90);
    let coordinates = view["route_parts"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|part| part["geometry"]["coordinates"].as_array().unwrap());
    for coordinate in coordinates {
        assert!(
            records
                .iter()
                .any(|point| point["hdop"].as_f64().unwrap() <= 5.0
                    && (coordinate[0].as_f64().unwrap() - point["lon"].as_f64().unwrap()).abs()
                        < 1e-10
                    && (coordinate[1].as_f64().unwrap() - point["lat"].as_f64().unwrap()).abs()
                        < 1e-10)
        );
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn isolated_one_second_jump_is_excluded_without_breaking_activity() {
    let mut records: Vec<_> = (0..20)
        .map(|i| record(at(8, 0) + i * 1000, 10.77 + i as f64 * 0.00002, 106.7))
        .collect();
    records[10]["lat"] = json!(11.0);
    let view = scenarios::publish(&records).await.view;
    assert_eq!(view["summary"]["trip_count"], 1);
    assert_eq!(view["summary"]["excluded_point_count"], 1);
    assert_eq!(view["evidence_holes"], json!([]));
    for part in view["route_parts"].as_array().unwrap() {
        assert!(
            !part["geometry"]["coordinates"]
                .as_array()
                .unwrap()
                .contains(&json!([106.7, 11.0]))
        );
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn master_1835_fixture_keeps_supported_movement() {
    let mut records: Vec<serde_json::Value> = include_str!("../../tools/lifetrail-phase2-testdata/master/stage-01-base/batches/144_d688a4be-6eb9-4295-8bdb-d0e4f5184817.ndjson")
        .lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    // Include the next actual usable observation to bound the final failure.
    records.push(serde_json::from_str(include_str!("../../tools/lifetrail-phase2-testdata/master/stage-01-base/batches/145_eb470e40-1b33-4da9-b262-7246e7049adc.ndjson").lines().next().unwrap()).unwrap());
    let view = scenarios::publish(&records).await.view;
    assert_eq!(view["summary"]["trip_count"], 1);
    assert_eq!(view["evidence_holes"], json!([]));
    assert_eq!(view["summary"]["low_quality_point_count"], 150);
    assert_eq!(view["summary"]["excluded_point_count"], 0);
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn long_failures_and_unbounded_edges_remain_holes() {
    for rejected in [0..2, 5..16, 18..20] {
        let mut records: Vec<_> = (0..20)
            .map(|i| record(at(8, 0) + i * 1000, 10.77 + i as f64 * 0.00002, 106.7))
            .collect();
        for i in rejected.clone() {
            records[i]["hdop"] = json!(8.0);
        }
        let view = scenarios::publish(&records).await.view;
        assert_eq!(
            view["evidence_holes"].as_array().unwrap().len(),
            1,
            "{rejected:?}"
        );
        assert_eq!(view["evidence_holes"][0]["reason"], "insufficient_quality");
        assert_eq!(view["summary"]["gap_count"], 0);
        assert_eq!(
            view["summary"]["trip_count"],
            if rejected.start == 5 { 2 } else { 1 }
        );
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn short_failures_do_not_hide_a_stationary_dwell() {
    let records: Vec<_> = (0..242)
        .map(|i| {
            let mut point = record(at(8, 0) + i * 1000, 10.77, 106.7);
            point["hdop"] = json!([3.5, 4.5, 5.5, 6.5][i as usize % 4]);
            point
        })
        .collect();
    let view = scenarios::publish(&records).await.view;
    assert_eq!(view["summary"]["stop_count"], 1);
    assert_eq!(view["summary"]["trip_count"], 0);
    assert_eq!(view["evidence_holes"], json!([]));
    assert_eq!(view["timeline"][0]["source_record_count"], 242);
    assert_eq!(view["timeline"][0]["usable_record_count"], 122);
    assert_eq!(view["timeline"][0]["observed_duration_s"], 241);
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn disabling_continuity_retires_snapshot_and_preserves_policy_provenance() {
    use lifetrail_server::processing;
    let mut records: Vec<_> = (0..20)
        .map(|i| record(at(8, 0) + i * 1000, 10.77 + i as f64 * 0.00002, 106.7))
        .collect();
    records[10]["hdop"] = json!(8.0);
    let published = scenarios::publish(&records).await;
    assert_eq!(published.view["summary"]["trip_count"], 1);
    sqlx::query("UPDATE device_processing_control SET short_failure_max_s=0 WHERE device_id=$1")
        .bind(published.device_id)
        .execute(&published.pool)
        .await
        .unwrap();
    let retained = published.reread("2026-10-05").await;
    assert_eq!(retained["processing"]["data_freshness"], "stale");
    assert_eq!(retained["summary"]["trip_count"], 1);
    processing::queue_day(
        &published.pool,
        published.device_id,
        "2026-10-05".parse().unwrap(),
    )
    .await
    .unwrap();
    processing::process_next(&published.pool).await.unwrap();
    let rebuilt = published.reread("2026-10-05").await;
    assert_eq!(rebuilt["summary"]["trip_count"], 2);
    assert_eq!(rebuilt["evidence_holes"].as_array().unwrap().len(), 1);
    let config: serde_json::Value =
        sqlx::query_scalar("SELECT config FROM activity_revisions WHERE id=$1::uuid")
            .bind(
                rebuilt["timeline"][0]["activity_revision"]
                    .as_str()
                    .unwrap(),
            )
            .fetch_one(&published.pool)
            .await
            .unwrap();
    assert_eq!(config["short_failure_max_s"], 0);
}
