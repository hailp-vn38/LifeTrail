use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Extension, Path, State, rejection::PathRejection},
    http::{HeaderMap, HeaderValue, Request, header},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use serde::Serialize;
use sqlx::PgPool;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use uuid::Uuid;

use crate::{
    db::{self, Device},
    error::ApiError,
};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

#[derive(Clone)]
struct RequestId(String);

pub fn router(state: AppState, static_dir: PathBuf) -> Router {
    let api = Router::new()
        .route("/v1/devices", get(list_devices))
        .route("/v1/devices/{device_id}", get(get_device))
        .route("/v1/device", get(authenticated_device))
        .fallback(api_not_found)
        .method_not_allowed_fallback(api_method_not_allowed)
        .with_state(state.clone());

    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
        .nest("/api", api)
        .fallback_service(
            ServeDir::new(&static_dir)
                .append_index_html_on_directories(true)
                .not_found_service(ServeFile::new(static_dir.join("index.html"))),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
            let request_id = request
                .extensions()
                .get::<RequestId>()
                .map(|value| value.0.as_str())
                .unwrap_or("missing");
            tracing::info_span!("http_request", method = %request.method(), uri = %request.uri(), request_id)
        }))
        .layer(middleware::from_fn(assign_request_id))
}

async fn assign_request_id(mut request: Request<axum::body::Body>, next: Next) -> Response {
    let request_id = format!("req_{}", Uuid::now_v7());
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));
    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}

async fn live() -> Json<StatusResponse> {
    Json(StatusResponse { status: "ok" })
}

async fn ready(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<StatusResponse>, ApiError> {
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .map_err(|_| ApiError::internal(request_id.0.clone()))?;
    Ok(Json(StatusResponse { status: "ok" }))
}

async fn list_devices(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
) -> Result<Json<DevicesResponse>, ApiError> {
    let devices = db::list_devices(&state.db)
        .await
        .map_err(|_| ApiError::internal(request_id.0))?;
    Ok(Json(DevicesResponse { devices }))
}

async fn get_device(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    device_id: Result<Path<Uuid>, PathRejection>,
) -> Result<Json<Device>, ApiError> {
    let Path(device_id) = device_id.map_err(|_| ApiError::invalid_request(request_id.0.clone()))?;
    let device = db::find_device(&state.db, device_id)
        .await
        .map_err(|_| ApiError::internal(request_id.0.clone()))?
        .ok_or_else(|| ApiError::not_found(request_id.0))?;
    Ok(Json(device))
}

async fn authenticated_device(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
) -> Result<Json<Device>, ApiError> {
    let token =
        bearer_token(&headers).ok_or_else(|| ApiError::invalid_token(request_id.0.clone()))?;
    let device = db::resolve_device_token(&state.db, token)
        .await
        .map_err(|_| ApiError::internal(request_id.0.clone()))?
        .ok_or_else(|| ApiError::invalid_token(request_id.0))?;
    Ok(Json(device))
}

async fn api_not_found(Extension(request_id): Extension<RequestId>) -> ApiError {
    ApiError::not_found(request_id.0)
}

async fn api_method_not_allowed(Extension(request_id): Extension<RequestId>) -> ApiError {
    ApiError::method_not_allowed(request_id.0)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

#[derive(Serialize)]
struct StatusResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct DevicesResponse {
    devices: Vec<Device>,
}
