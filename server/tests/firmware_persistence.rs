//! Real firmware decisions through ingestion, processing and published Daily Views.
mod support;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db, processing,
};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command};
use support::{read, upload};
use uuid::Uuid;

fn epoch(value: &Value) -> i64 {
    value
        .as_str()
        .unwrap()
        .parse::<DateTime<Utc>>()
        .unwrap()
        .timestamp()
}

fn activities<'a>(view: &'a Value, kind: &str) -> Vec<&'a Value> {
    view["timeline"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["kind"] == kind)
        .collect()
}

async fn publish(
    pool: &sqlx::PgPool,
    router: &axum::Router,
    owner: Uuid,
    name: &str,
    records: &[Value],
) -> Value {
    let token = generate_device_token();
    let device = db::create_device(pool, owner, name, &token).await.unwrap();
    for chunk in records.chunks(500) {
        let body: String = chunk.iter().map(|record| format!("{record}\n")).collect();
        assert_eq!(
            upload(router, &token, Uuid::new_v4(), &body).await.0,
            StatusCode::OK
        );
    }
    let thresholds: (i64, i64, i64, f64) = sqlx::query_as(
        "SELECT observation_gap_s,short_failure_max_s,stop_min_duration_s,stop_radius_m FROM device_processing_control WHERE device_id=$1")
        .bind(device.id).fetch_one(pool).await.unwrap();
    assert_eq!(thresholds, (300, 10, 180, 30.0));
    assert!(processing::process_next(pool).await.unwrap());
    let path = format!("/api/v1/devices/{}/days/2026-10-05", device.id);
    let view = read(router, &path).await;
    assert_eq!(view["processing_state"], "processed");
    assert_eq!(view["summary"]["point_count"], records.len());
    let raw = read(router, &format!("{path}?view=raw")).await;
    assert_eq!(raw["processing_state"], "raw");
    for part in view["route_parts"].as_array().unwrap() {
        assert_eq!(part["source"], "processed_gps");
        support::assert_route_part(part, name, 0.01);
    }
    view
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL/PostGIS, Python and CMake"]
async fn device_persistence_preserves_phase2_semantics_and_geometry() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let fixtures = std::env::temp_dir().join(format!("lifetrail-fw-{}", Uuid::new_v4()));
    let generated = Command::new("python3")
        .arg(root.join("tools/gps_persistence_acceptance.py"))
        .arg("--output")
        .arg(&fixtures)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    // Suite is only run against the explicitly provided disposable test database.
    sqlx::query("TRUNCATE users CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    let owner = db::create_owner(&pool, "Owner", "UTC").await.unwrap();
    let router = app::router(AppState { db: pool.clone() }, None);
    let mut report = Vec::new();
    for name in [
        "stationary",
        "lost_heartbeat",
        "walk",
        "drive",
        "turn",
        "commute",
        "short_stop",
        "no_fix",
    ] {
        let fixture: Value = serde_json::from_str(
            &std::fs::read_to_string(fixtures.join(format!("{name}.json"))).unwrap(),
        )
        .unwrap();
        let original = fixture["baseline"].as_array().unwrap();
        let filtered = fixture["filtered"].as_array().unwrap();
        assert!(filtered.len() < original.len());
        if name == "stationary" {
            assert!(filtered.len() * 20 < original.len());
        }
        let baseline = publish(
            &pool,
            &router,
            owner.id,
            &format!("{name}-baseline"),
            original,
        )
        .await;
        let sparse = publish(
            &pool,
            &router,
            owner.id,
            &format!("{name}-filtered"),
            filtered,
        )
        .await;
        for kind in ["stop", "trip", "gap"] {
            assert_eq!(
                activities(&baseline, kind).len(),
                activities(&sparse, kind).len(),
                "{name}: {kind} continuity"
            );
        }
        // short_failure_max_s concerns rejected observations, not valid sparse intervals.
        assert!(
            sparse["evidence_holes"].as_array().unwrap().is_empty(),
            "{name}: {}",
            sparse["evidence_holes"]
        );
        if name == "no_fix" {
            assert_eq!(activities(&sparse, "gap").len(), 1);
        } else {
            assert!(activities(&sparse, "gap").is_empty());
        }
        let mut stop_error = 0;
        for (before, after) in activities(&baseline, "stop")
            .iter()
            .zip(activities(&sparse, "stop"))
        {
            for boundary in ["observed_from_at", "observed_until_at"] {
                let difference = (epoch(&before[boundary]) - epoch(&after[boundary])).abs();
                assert!(difference <= 30, "{name}: {boundary} error {difference}s");
                stop_error = stop_error.max(difference);
            }
        }
        for (before, after) in activities(&baseline, "trip")
            .iter()
            .zip(activities(&sparse, "trip"))
        {
            let difference =
                (epoch(&before["observed_from_at"]) - epoch(&after["observed_from_at"])).abs();
            assert!(difference < 5, "{name}: trip start error {difference}s");
        }
        let cross_track = support_geometry::maximum_error(&baseline, &sparse);
        assert!(
            cross_track < if name == "turn" { 30.0 } else { 15.0 },
            "{name}: geometry error {cross_track}m"
        );
        report.push(json!({"name":name,"raw_navigation_epochs":original.len(),
            "persisted_gps_records":filtered.len(),"persist_ratio":filtered.len() as f64 / original.len() as f64,
            "max_crosstrack_error_vs_baseline_m":cross_track,"stop_boundary_error_s":stop_error}));
    }
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    std::fs::write(
        fixtures.join("report.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("Acceptance artifacts: {}", fixtures.display());
}

#[path = "support/persistence_geometry.rs"]
mod support_geometry;
