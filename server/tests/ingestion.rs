use std::env;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt as _;
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db,
};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use sqlx::Row as _;
use tower::ServiceExt as _;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires real PostgreSQL/PostGIS; use docker compose --profile test run --rm server-tests"]
async fn device_ingestion_commits_atomically_and_replays_verified_batches() {
    let database_url = env::var("LT_TEST_DATABASE_URL").expect("LT_TEST_DATABASE_URL must be set");
    let pool = db::connect(&database_url)
        .await
        .expect("connect test database");
    db::migrate(&pool).await.expect("run migrations");
    sqlx::query("TRUNCATE gps_points, ingest_batches, devices, users CASCADE")
        .execute(&pool)
        .await
        .expect("clear isolated test database");

    let owner = db::create_owner(&pool, "LifeTrail Owner", "Asia/Ho_Chi_Minh")
        .await
        .expect("create owner");
    let token = generate_device_token();
    let device = db::create_device(&pool, owner.id, "GPS Recorder", &token)
        .await
        .expect("create device");
    let app = app::router(
        AppState { db: pool.clone() },
        Some("missing-static-dir".into()),
    );
    let batch_id = Uuid::new_v4();
    let body = valid_body();

    let committed = send_batch(app.clone(), &token, batch_id, body.clone()).await;
    assert_eq!(committed.status(), StatusCode::OK);
    assert_eq!(response_json(committed).await["duplicate"], false);
    assert_counts(&pool, 1, 2).await;

    let replay = send_batch(app.clone(), &token, batch_id, body).await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(response_json(replay).await["duplicate"], true);
    assert_counts(&pool, 1, 2).await;

    let conflict = send_batch(app.clone(), &token, batch_id, different_valid_body()).await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(conflict).await["error"]["code"],
        "batch_conflict"
    );
    assert_counts(&pool, 1, 2).await;

    let invalid_cases = [
        ("empty", Vec::new(), None, None),
        (
            "missing_final_lf",
            b"{\"ts_ms\":1,\"lat\":10,\"lon\":106,\"fix_quality\":1,\"satellites\":8}".to_vec(),
            None,
            None,
        ),
        (
            "crlf",
            b"{\"ts_ms\":1,\"lat\":10,\"lon\":106,\"fix_quality\":1,\"satellites\":8}\r\n".to_vec(),
            None,
            None,
        ),
        (
            "blank_line",
            b"{\"ts_ms\":1,\"lat\":10,\"lon\":106,\"fix_quality\":1,\"satellites\":8}\n\n".to_vec(),
            None,
            None,
        ),
        ("malformed_json", b"{not-json}\n".to_vec(), None, None),
        ("array", b"[]\n".to_vec(), None, None),
        ("primitive", b"1\n".to_vec(), None, None),
        (
            "unknown_field",
            b"{\"ts_ms\":1,\"lat\":10,\"lon\":106,\"fix_quality\":1,\"satellites\":8,\"extra\":true}\n".to_vec(),
            None,
            None,
        ),
        (
            "missing_required_field",
            b"{\"ts_ms\":1,\"lat\":10,\"lon\":106,\"fix_quality\":1}\n".to_vec(),
            None,
            None,
        ),
        ("bad_hash", valid_body(), Some("0".repeat(64)), None),
        ("bad_count", valid_body(), None, Some("9".to_owned())),
        ("out_of_order", out_of_order_body(), None, None),
        (
            "bad_range",
            b"{\"ts_ms\":1,\"lat\":91,\"lon\":106,\"fix_quality\":1,\"satellites\":8}\n".to_vec(),
            None,
            None,
        ),
    ];
    for (name, invalid_body, hash, count) in invalid_cases {
        let response = send_batch_with_metadata(
            app.clone(),
            &token,
            Uuid::new_v4(),
            invalid_body,
            hash,
            count,
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{name}"
        );
        let json = response_json(response).await;
        assert_eq!(json["error"]["code"], "invalid_batch", "{name}");
        assert!(
            json["error"]["request_id"]
                .as_str()
                .unwrap()
                .starts_with("req_")
        );
        assert_counts(&pool, 1, 2).await;
    }

    let overlap = send_batch(app.clone(), &token, Uuid::new_v4(), valid_body()).await;
    assert_eq!(overlap.status(), StatusCode::OK);
    assert_eq!(response_json(overlap).await["duplicate"], false);
    assert_counts(&pool, 2, 4).await;

    let unauthorized =
        send_batch(app.clone(), "lt_dev_unknown", Uuid::new_v4(), valid_body()).await;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response_json(unauthorized).await["error"]["code"],
        "invalid_token"
    );

    sqlx::query(
        "CREATE FUNCTION reject_gps_point_insert() RETURNS trigger LANGUAGE plpgsql AS $$ \
         BEGIN RAISE EXCEPTION 'test insert failure'; END; $$;",
    )
    .execute(&pool)
    .await
    .expect("create failing point function");
    sqlx::query(
        "CREATE TRIGGER reject_gps_point_insert BEFORE INSERT ON gps_points \
         FOR EACH ROW EXECUTE FUNCTION reject_gps_point_insert()",
    )
    .execute(&pool)
    .await
    .expect("install failing point trigger");
    let failed_commit = send_batch(app.clone(), &token, Uuid::new_v4(), valid_body()).await;
    assert_eq!(failed_commit.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response_json(failed_commit).await["error"]["code"],
        "internal_error"
    );
    assert_counts(&pool, 2, 4).await;
    sqlx::query("DROP TRIGGER reject_gps_point_insert ON gps_points")
        .execute(&pool)
        .await
        .expect("remove failing point trigger");
    sqlx::query("DROP FUNCTION reject_gps_point_insert()")
        .execute(&pool)
        .await
        .expect("remove failing point function");

    let oversized = send_batch(app, &token, Uuid::new_v4(), vec![b'x'; 1_048_577]).await;
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        response_json(oversized).await["error"]["code"],
        "body_too_large"
    );
    assert_counts(&pool, 2, 4).await;

    let geometry: String = sqlx::query("SELECT ST_AsText(geometry) AS geometry FROM gps_points WHERE device_id = $1 ORDER BY id LIMIT 1")
        .bind(device.id)
        .fetch_one(&pool)
        .await
        .expect("select generated geometry")
        .try_get("geometry")
        .expect("decode geometry");
    assert_eq!(geometry, "POINT(106.692345 10.781234)");
}

async fn send_batch(
    app: axum::Router,
    token: &str,
    batch_id: Uuid,
    body: Vec<u8>,
) -> axum::response::Response {
    send_batch_with_metadata(app, token, batch_id, body, None, None).await
}

async fn send_batch_with_metadata(
    app: axum::Router,
    token: &str,
    batch_id: Uuid,
    body: Vec<u8>,
    hash_override: Option<String>,
    count_override: Option<String>,
) -> axum::response::Response {
    let hash = hash_override.unwrap_or_else(|| format!("{:x}", Sha256::digest(&body)));
    let count = count_override.unwrap_or_else(|| {
        body.iter()
            .filter(|&&byte| byte == b'\n')
            .count()
            .to_string()
    });
    app.oneshot(
        Request::post("/api/v1/device/batches")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/x-ndjson")
            .header("x-lifetrail-batch-id", batch_id.to_string())
            .header("x-lifetrail-schema", "gps/1")
            .header("x-lifetrail-content-sha256", hash)
            .header("x-lifetrail-byte-length", body.len().to_string())
            .header("x-lifetrail-record-count", count)
            .body(Body::from(body))
            .expect("construct ingestion request"),
    )
    .await
    .expect("ingestion response")
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).expect("JSON response")
}

async fn assert_counts(pool: &sqlx::PgPool, expected_batches: i64, expected_points: i64) {
    let batches: i64 = sqlx::query_scalar("SELECT count(*) FROM ingest_batches")
        .fetch_one(pool)
        .await
        .expect("count batches");
    let points: i64 = sqlx::query_scalar("SELECT count(*) FROM gps_points")
        .fetch_one(pool)
        .await
        .expect("count points");
    assert_eq!(batches, expected_batches);
    assert_eq!(points, expected_points);
}

fn valid_body() -> Vec<u8> {
    b"{\"ts_ms\":1791102702000,\"lat\":10.781234,\"lon\":106.692345,\"fix_quality\":1,\"satellites\":8}\n{\"ts_ms\":1791102703000,\"lat\":10.781235,\"lon\":106.692346,\"fix_quality\":1,\"satellites\":8}\n".to_vec()
}

fn different_valid_body() -> Vec<u8> {
    b"{\"ts_ms\":1791102702000,\"lat\":10.781234,\"lon\":106.692345,\"fix_quality\":1,\"satellites\":8}\n".to_vec()
}

fn out_of_order_body() -> Vec<u8> {
    b"{\"ts_ms\":2,\"lat\":10,\"lon\":106,\"fix_quality\":1,\"satellites\":8}\n{\"ts_ms\":1,\"lat\":10,\"lon\":106,\"fix_quality\":1,\"satellites\":8}\n".to_vec()
}
