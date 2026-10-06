mod support;
use axum::http::StatusCode;
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db, processing,
};
use serde_json::{Value, json};
use support::{read, upload};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn stationary_fixtures_distinguish_dwell_pauses_missing_and_unusable_observations() {
    let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("TRUNCATE users CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    let owner = db::create_owner(&pool, "Owner", "UTC").await.unwrap();
    let router = app::router(AppState { db: pool.clone() }, None);
    let cases: Vec<Value> = serde_json::from_str(include_str!("fixtures/stationary.json")).unwrap();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let token = generate_device_token();
        let device = db::create_device(&pool, owner.id, name, &token)
            .await
            .unwrap();
        let records = case["records"].as_array().unwrap();
        let body: String = records
            .iter()
            .map(|record| {
                format!(
                    "{}\n",
                    json!({
                        "ts_ms":1791158400000_i64+record[0].as_i64().unwrap()*1000,
                        "lat":10.77+record[1].as_f64().unwrap(), "lon":106.7,
                        "fix_quality":record[2], "satellites":8,
                        "hdop":case["hdop"].as_f64().unwrap_or(1.2), "speed_mps":0
                    })
                )
            })
            .collect();
        assert_eq!(
            upload(&router, &token, Uuid::new_v4(), &body).await.0,
            StatusCode::OK,
            "{name}"
        );
        if case.get("radius_m").is_some() || case.get("minimum_duration_s").is_some() {
            sqlx::query("UPDATE device_processing_control SET stop_radius_m=$2,stop_min_duration_s=$3 WHERE device_id=$1")
                .bind(device.id).bind(case["radius_m"].as_f64().unwrap_or(30.0))
                .bind(case["minimum_duration_s"].as_i64().unwrap_or(180)).execute(&pool).await.unwrap();
        }
        assert!(processing::process_next(&pool).await.unwrap());
        let view = read(
            &router,
            &format!("/api/v1/devices/{}/days/2026-10-05", device.id),
        )
        .await;
        assert_eq!(view["processing_state"], "processed", "{name}");
        let stops = view["timeline"].as_array().unwrap();
        let durations: Vec<Value> = stops
            .iter()
            .map(|s| s["observed_duration_s"].clone())
            .collect();
        assert_eq!(json!(durations), case["durations"], "{name}");
        assert_eq!(view["summary"]["point_count"], records.len(), "{name}");
        assert_eq!(view["summary"]["stop_count"], stops.len(), "{name}");
        assert!(view["route"].is_null(), "{name}: no invented movement");
        if let Some(reason) = case.get("reason") {
            assert!(
                view["evidence_holes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .chain(view["unresolved_intervals"].as_array().unwrap().iter())
                    .any(|h| &h["reason"] == reason),
                "{name}"
            );
        }
        for (index, stop) in stops.iter().enumerate() {
            assert_eq!(
                stop["start_boundary"], case["boundaries"][index][0],
                "{name}"
            );
            assert_eq!(stop["end_boundary"], case["boundaries"][index][1], "{name}");
            if stop["start_boundary"] == "open" || stop["end_boundary"] == "open" {
                assert!(stop["full_duration_s"].is_null(), "{name}");
            } else {
                assert_eq!(
                    stop["full_duration_s"], stop["observed_duration_s"],
                    "{name}"
                );
            }
        }
    }
}
