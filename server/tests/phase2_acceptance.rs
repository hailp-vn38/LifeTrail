#[path = "support/phase2_suite.rs"]
mod suite;
mod support;
use lifetrail_server::processing;
use serde_json::Value;
use std::time::Instant;
use suite::Suite;
use support::{assert_display_route_part, assert_route_part};

fn contract(view: &Value) {
    for part in view["route_parts"].as_array().unwrap() {
        assert_eq!(part["source"], "processed_gps");
        assert!(part.get("matcher_evidence").is_none());
        assert_display_route_part(part, "Phase 2 fixture");
    }
}

fn playback_contract(playback: &Value) {
    for part in playback["route_parts"].as_array().unwrap() {
        assert_eq!(part["source"], "processed_gps");
        assert_route_part(part, "Phase 2 fixture", 0.01);
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn master_late_overlap_replay_and_range_reuse() {
    let suite = Suite::new().await;
    let count = suite.stage("master/stage-01-base").await;
    assert_eq!(suite.generation().await, count as i64);
    assert!(processing::process_next(&suite.pool).await.unwrap());
    let before = suite.day("2026-10-05").await;
    contract(&before);
    assert!(before["summary"]["excluded_point_count"].as_i64().unwrap() >= 1);
    assert!(before["evidence_holes"].as_array().unwrap().len() < 10);
    assert_eq!(before["summary"]["gap_count"], 2);
    let old_trip = before["timeline"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "trip")
        .unwrap()["id"]
        .clone();
    let late = suite.stage("master/stage-02-late-midnight-gap-fill").await;
    let retained = suite.day("2026-10-05").await;
    assert_eq!(
        retained["processing"]["published_revision"],
        before["processing"]["published_revision"]
    );
    assert!(processing::process_next(&suite.pool).await.unwrap());
    let first = suite.day("2026-10-05").await;
    let second = suite.day("2026-10-06").await;
    contract(&first);
    contract(&second);
    playback_contract(&suite.playback("2026-10-05").await);
    playback_contract(&suite.playback("2026-10-06").await);
    assert_eq!(first["summary"]["gap_count"], 1);
    assert_eq!(
        first["provenance"]["manifest_version"],
        second["provenance"]["manifest_version"]
    );
    let crossing = first["timeline"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "stop" && e["continues_after"] == true)
        .unwrap();
    let next = second["timeline"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == crossing["id"])
        .unwrap();
    assert!(crossing["daily_observed_duration_s"].as_i64().unwrap() > 17000);
    assert!(next["daily_observed_duration_s"].as_i64().unwrap() >= 1190);
    assert!(
        first["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == old_trip),
        "unchanged early activity must reuse its revision"
    );
    let overlaps = suite.stage("master/stage-03-overlap-better-quality").await;
    assert!(processing::process_next(&suite.pool).await.unwrap());
    contract(&suite.day("2026-10-05").await);
    assert_eq!(suite.generation().await, (count + late + overlaps) as i64);
    let replay = include_str!(
        "../../tools/lifetrail-phase2-testdata/master/stage-04-replay-and-conflict/replay-identical.ndjson"
    );
    let conflict = include_str!(
        "../../tools/lifetrail-phase2-testdata/master/stage-04-replay-and-conflict/same-id-different-body.ndjson"
    );
    let id = "bc63d63a-d640-43c7-88a7-6a7fcfe94e5a".parse().unwrap();
    let response = support::upload(&suite.router, &suite.token, id, replay).await;
    assert_eq!(response.0, axum::http::StatusCode::OK);
    assert_eq!(response.1["duplicate"], true);
    assert_eq!(
        support::upload(&suite.router, &suite.token, id, conflict)
            .await
            .0,
        axum::http::StatusCode::CONFLICT
    );
    assert_eq!(suite.generation().await, (count + late + overlaps) as i64);
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS; prints scale acceptance measurements"]
async fn scale_30000_processed_gps() {
    let suite = Suite::new().await;
    assert_eq!(suite.stage("scale-30000").await, 100);
    assert_eq!(suite.generation().await, 100);
    let start = Instant::now();
    assert!(processing::process_next(&suite.pool).await.unwrap());
    let processing_ms = start.elapsed().as_millis();
    let start = Instant::now();
    let body: Value =
        sqlx::query_scalar("SELECT body FROM daily_snapshots ORDER BY created_at DESC LIMIT 1")
            .fetch_one(&suite.pool)
            .await
            .unwrap();
    let db_read_ms = start.elapsed().as_millis();
    let start = Instant::now();
    let view = suite.day(body["date"].as_str().unwrap()).await;
    let api_read_ms = start.elapsed().as_millis();
    assert_eq!(view["summary"]["point_count"], 30000);
    assert_eq!(view["processing"]["data_freshness"], "current");
    contract(&view);
    let playback = suite.playback(body["date"].as_str().unwrap()).await;
    playback_contract(&playback);
    let display_vertices: usize = view["route_parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| {
            part["display_geometry"]["coordinates"]
                .as_array()
                .map_or(0, Vec::len)
        })
        .sum();
    let display_bytes = serde_json::to_vec(&view["route_parts"]).unwrap().len();
    let playback_bytes = serde_json::to_vec(&playback["route_parts"]).unwrap().len();
    println!(
        "SCALE display_vertices={display_vertices} display_parts_bytes={display_bytes} playback_parts_bytes={playback_bytes}"
    );
    assert!(
        display_bytes < playback_bytes,
        "the Daily Map payload must be lighter than canonical playback"
    );
    let memory = std::fs::read_to_string("/proc/self/status").unwrap();
    let peak = memory
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .unwrap();
    println!(
        "SCALE processing_ms={processing_ms} db_read_ms={db_read_ms} api_read_ms={api_read_ms} snapshot_bytes={} payload_bytes={} {peak}",
        serde_json::to_vec(&body).unwrap().len(),
        serde_json::to_vec(&view).unwrap().len()
    );
}
