//! Real fixture Batches through ingestion and the durable processing worker.
use crate::support::{read, upload};
use axum::http::StatusCode;
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db,
};
use serde_json::Value;
use sqlx::PgPool;
use std::path::PathBuf;
use uuid::Uuid;

pub struct Suite {
    pub pool: PgPool,
    pub router: axum::Router,
    pub device: Uuid,
    pub token: String,
}
impl Suite {
    pub async fn new() -> Self {
        let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        db::migrate(&pool).await.unwrap();
        sqlx::query("TRUNCATE users CASCADE")
            .execute(&pool)
            .await
            .unwrap();
        let owner = db::create_owner(&pool, "Owner", "Asia/Ho_Chi_Minh")
            .await
            .unwrap();
        let token = generate_device_token();
        let device = db::create_device(&pool, owner.id, "Device", &token)
            .await
            .unwrap();
        let router = app::router(AppState { db: pool.clone() }, None);
        Self {
            pool,
            router,
            device: device.id,
            token,
        }
    }
    pub async fn stage(&self, relative: &str) -> usize {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tools/lifetrail-phase2-testdata")
            .join(relative)
            .join("batches");
        let mut paths: Vec<_> = std::fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "ndjson"))
            .collect();
        paths.sort();
        for path in &paths {
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let id = stem.split_once('_').unwrap().1.parse::<Uuid>().unwrap();
            let body = std::fs::read_to_string(path).unwrap();
            assert_eq!(
                upload(&self.router, &self.token, id, &body).await.0,
                StatusCode::OK
            );
        }
        paths.len()
    }
    pub async fn day(&self, date: &str) -> Value {
        read(
            &self.router,
            &format!("/api/v1/devices/{}/days/{date}", self.device),
        )
        .await
    }
    pub async fn generation(&self) -> i64 {
        sqlx::query_scalar(
            "SELECT input_generation FROM device_processing_control WHERE device_id=$1",
        )
        .bind(self.device)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }
}
