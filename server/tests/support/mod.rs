use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
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

/// One accepted gps/1 record with usable acquisition metadata.
#[allow(dead_code)]
pub fn record(ts_ms: i64, lat: f64, lon: f64) -> Value {
    json!({"ts_ms":ts_ms,"lat":lat,"lon":lon,"fix_quality":1,"satellites":8,"hdop":1.2})
}

/// Exact gps/1 Batch framing for a sequence of records.
#[allow(dead_code)]
pub fn ndjson(records: &[Value]) -> String {
    records.iter().map(|record| format!("{record}\n")).collect()
}

/// Invariants every published Route Part must satisfy: aligned progress with a
/// zero origin, non-decreasing bounded progress, strictly increasing anchors and
/// a final progress equal to the part length within tolerance.
#[allow(dead_code)]
pub fn assert_route_part(part: &Value, context: &str, tolerance: f64) {
    let coordinates = part["geometry"]["coordinates"]
        .as_array()
        .expect("coordinates");
    assert_eq!(
        part["geometry"]["type"], "LineString",
        "{context}: valid LineString"
    );
    assert!(
        coordinates.len() >= 2,
        "{context}: at least two coordinates"
    );
    let progress = part["vertex_distance_m"]
        .as_array()
        .expect("vertex_distance_m");
    assert_eq!(
        progress.len(),
        coordinates.len(),
        "{context}: vertex_distance_m aligns 1:1 with coordinates"
    );
    assert_eq!(
        progress[0].as_f64().unwrap(),
        0.0,
        "{context}: progress starts at zero"
    );
    let distance = part["distance_m"].as_f64().unwrap();
    let visible = part["visible_distance_m"].as_f64().unwrap();
    assert!(
        visible > 0.0 && visible <= distance + tolerance,
        "{context}: visible distance is positive and bounded by the part distance"
    );
    for pair in progress.windows(2) {
        assert!(
            pair[1].as_f64().unwrap() >= pair[0].as_f64().unwrap(),
            "{context}: progress is non-decreasing"
        );
    }
    assert!(
        (progress.last().unwrap().as_f64().unwrap() - visible).abs() <= tolerance,
        "{context}: final progress equals the published visible part distance"
    );
    let anchors = part["progress_anchors"].as_array().expect("anchors");
    assert!(anchors.len() >= 2, "{context}: start and end anchors");
    // Anchor times compare as instants: records may share a second, so an optional
    // fractional part makes lexical order differ from chronological order.
    let instant = |anchor: &Value| {
        chrono::DateTime::parse_from_rfc3339(anchor["at"].as_str().expect("anchor time"))
            .expect("anchor time is RFC 3339")
    };
    let mut previous: Option<chrono::DateTime<chrono::FixedOffset>> = None;
    let mut previous_distance = 0.0;
    for anchor in anchors {
        let at = anchor["at"].as_str().expect("anchor time").to_owned();
        if let Some(previous) = &previous {
            assert!(
                instant(anchor) > *previous,
                "{context}: anchor times increase strictly ({at} after {previous})"
            );
        }
        let anchor_distance = anchor["distance_m"].as_f64().unwrap();
        assert!(
            anchor_distance >= previous_distance - tolerance,
            "{context}: anchor progress is non-decreasing"
        );
        assert!(
            anchor_distance <= distance + tolerance,
            "{context}: anchor progress is bounded by the part distance"
        );
        previous = Some(instant(anchor));
        previous_distance = anchor_distance;
    }
    assert_eq!(
        anchors[0]["at"].as_str().unwrap(),
        part["visible_from_at"].as_str().unwrap(),
        "{context}: first anchor is the visible start"
    );
    assert_eq!(
        anchors[anchors.len() - 1]["at"].as_str().unwrap(),
        part["visible_until_at"].as_str().unwrap(),
        "{context}: last anchor is the visible end"
    );
    assert_eq!(
        anchors[0]["distance_m"].as_f64().unwrap(),
        0.0,
        "{context}: visible progress is rebased to zero"
    );
}
