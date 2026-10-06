use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;

pub async fn read(router: &axum::Router, path: &str) -> Value {
    let response = router
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
pub async fn upload(
    router: &axum::Router,
    token: &str,
    batch: Uuid,
    body: &str,
) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/device/batches")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/x-ndjson")
                .header("X-LifeTrail-Batch-Id", batch.to_string())
                .header("X-LifeTrail-Schema", "gps/1")
                .header(
                    "X-LifeTrail-Content-SHA256",
                    format!("{:x}", Sha256::digest(body.as_bytes())),
                )
                .header("X-LifeTrail-Byte-Length", body.len().to_string())
                .header("X-LifeTrail-Record-Count", body.lines().count().to_string())
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    (
        status,
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap(),
    )
}
