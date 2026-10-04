use std::{env, process::Command};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt as _;
use lifetrail_server::{
    app::{self, AppState},
    auth::digest_token,
    db,
};
use serde_json::Value;
use sqlx::Row as _;
use tower::ServiceExt as _;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires real PostgreSQL/PostGIS; use docker compose --profile test run --rm server-tests"]
async fn provisioning_migrates_postgis_lists_devices_and_authenticates_bearer_tokens() {
    let database_url = env::var("LT_TEST_DATABASE_URL").expect("LT_TEST_DATABASE_URL must be set");
    let pool = db::connect(&database_url)
        .await
        .expect("connect test database");
    db::migrate(&pool).await.expect("run migrations");
    sqlx::query("TRUNCATE devices, users CASCADE")
        .execute(&pool)
        .await
        .expect("clear isolated test database");

    let postgis_version: String = sqlx::query_scalar("SELECT postgis_version()")
        .fetch_one(&pool)
        .await
        .expect("PostGIS extension is available");
    assert!(!postgis_version.is_empty());

    let owner = run_cli(
        &database_url,
        [
            "owner",
            "create",
            "--display-name",
            "LifeTrail Owner",
            "--timezone",
            "Asia/Ho_Chi_Minh",
        ],
    );
    let owner_id = owner["id"].as_str().expect("Owner id");
    let created_device = run_cli(
        &database_url,
        [
            "device",
            "create",
            "--owner-id",
            owner_id,
            "--name",
            "GPS Recorder",
        ],
    );
    let device_id: Uuid = created_device["id"]
        .as_str()
        .expect("Device id")
        .parse()
        .expect("Device UUID");
    let token = created_device["token"].as_str().expect("plaintext token");

    let stored_digest: Vec<u8> = sqlx::query("SELECT token_digest FROM devices WHERE id = $1")
        .bind(device_id)
        .fetch_one(&pool)
        .await
        .expect("select token digest")
        .try_get("token_digest")
        .expect("decode digest");
    assert_eq!(stored_digest, digest_token(token));
    assert_ne!(stored_digest, token.as_bytes());

    let app = app::router(AppState { db: pool.clone() }, "missing-static-dir".into());
    let devices_response = app
        .clone()
        .oneshot(Request::get("/api/v1/devices").body(Body::empty()).unwrap())
        .await
        .expect("device list response");
    assert_eq!(devices_response.status(), StatusCode::OK);
    let devices_body = devices_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let devices_json: Value = serde_json::from_slice(&devices_body).unwrap();
    assert!(
        devices_json["devices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == device_id.to_string())
    );

    let valid_response = app
        .clone()
        .oneshot(
            Request::get("/api/v1/device")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("valid bearer response");
    assert_eq!(valid_response.status(), StatusCode::OK);

    let method_not_allowed = app
        .clone()
        .oneshot(
            Request::post("/api/v1/devices")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("method not allowed response");
    assert_eq!(method_not_allowed.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert!(method_not_allowed.headers().contains_key("x-request-id"));
    let method_not_allowed_body = method_not_allowed
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let method_not_allowed_json: Value = serde_json::from_slice(&method_not_allowed_body).unwrap();
    assert_eq!(
        method_not_allowed_json["error"]["code"],
        "method_not_allowed"
    );

    assert_cli_fails(
        &database_url,
        [
            "owner",
            "create",
            "--display-name",
            "Second Owner",
            "--timezone",
            "Asia/Ho_Chi_Minh",
        ],
        "an Owner already exists for this deployment",
    );
    assert_cli_fails(
        &database_url,
        [
            "owner",
            "create",
            "--display-name",
            "Bad Timezone",
            "--timezone",
            "not-a-timezone",
        ],
        "timezone must be a valid IANA timezone",
    );

    let unknown_response = app
        .oneshot(
            Request::get("/api/v1/device")
                .header("authorization", "Bearer lt_dev_unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("unknown bearer response");
    assert_eq!(unknown_response.status(), StatusCode::UNAUTHORIZED);
    let unknown_body = unknown_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let unknown_json: Value = serde_json::from_slice(&unknown_body).unwrap();
    assert_eq!(unknown_json["error"]["code"], "invalid_token");
    assert!(
        unknown_json["error"]["request_id"]
            .as_str()
            .unwrap()
            .starts_with("req_")
    );
}

fn run_cli<const N: usize>(database_url: &str, arguments: [&str; N]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_lifetrail-server"))
        .env("LT_DATABASE_URL", database_url)
        .args(arguments)
        .output()
        .expect("run provisioning CLI");
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("CLI emits JSON")
}

fn assert_cli_fails<const N: usize>(database_url: &str, arguments: [&str; N], expected: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_lifetrail-server"))
        .env("LT_DATABASE_URL", database_url)
        .args(arguments)
        .output()
        .expect("run provisioning CLI");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "CLI error did not contain {expected:?}: {stderr}"
    );
}
