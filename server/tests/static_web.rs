use std::{fs, path::PathBuf};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt as _;
use lifetrail_server::app::{self, AppState};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt as _;
use uuid::Uuid;

#[tokio::test]
async fn built_web_and_canonical_daily_url_are_served_from_the_server_origin() {
    let static_dir = temporary_static_dir();
    fs::write(static_dir.join("index.html"), "<main>LifeTrail Web</main>")
        .expect("write built web index");
    fs::write(static_dir.join("assets/app.js"), "console.log('LifeTrail')")
        .expect("write built web asset");
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://lifetrail:lifetrail@127.0.0.1/lifetrail")
        .expect("create lazy pool");
    let app = app::router(AppState { db: pool }, static_dir.clone());

    assert_html(app.clone(), "/").await;
    assert_asset(app.clone()).await;
    assert_html(
        app,
        "/devices/f273162b-31a4-42db-a0a0-32f342e72a27/day/2026-10-04",
    )
    .await;

    fs::remove_dir_all(static_dir).expect("remove temporary static directory");
}

async fn assert_html(app: axum::Router, uri: &str) {
    let response = app
        .oneshot(
            Request::get(uri)
                .body(Body::empty())
                .expect("construct request"),
        )
        .await
        .expect("static response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    assert_eq!(body.as_ref(), b"<main>LifeTrail Web</main>");
}

async fn assert_asset(app: axum::Router) {
    let response = app
        .oneshot(
            Request::get("/assets/app.js")
                .body(Body::empty())
                .expect("construct asset request"),
        )
        .await
        .expect("asset response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("read asset")
        .to_bytes();
    assert_eq!(body.as_ref(), b"console.log('LifeTrail')");
}

fn temporary_static_dir() -> PathBuf {
    let directory = std::env::temp_dir().join(format!("lifetrail-static-web-{}", Uuid::new_v4()));
    fs::create_dir_all(directory.join("assets")).expect("create temporary static directory");
    directory
}
