mod persistence;
mod validation;

use axum::{
    Json,
    body::Body,
    extract::{Extension, State},
    http::HeaderMap,
};
use futures_util::StreamExt as _;
use serde::Serialize;

use crate::{
    app::{AppState, RequestId, bearer_token},
    db,
    error::ApiError,
};

const MAX_BATCH_BYTES: usize = 1_048_576;

#[derive(Serialize)]
pub(crate) struct BatchAcknowledgement {
    batch_id: uuid::Uuid,
    status: &'static str,
    record_count: u64,
    duplicate: bool,
}

pub(crate) async fn ingest_batch(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    headers: HeaderMap,
    body: Body,
) -> Result<Json<BatchAcknowledgement>, ApiError> {
    let token =
        bearer_token(&headers).ok_or_else(|| ApiError::invalid_token(request_id.0.clone()))?;
    let device = db::resolve_device_token(&state.db, token)
        .await
        .map_err(|_| ApiError::internal(request_id.0.clone()))?
        .ok_or_else(|| ApiError::invalid_token(request_id.0.clone()))?;
    let metadata = validation::parse_metadata(&headers)
        .map_err(|error| ApiError::invalid_batch(request_id.0.clone(), error.details()))?;
    let bytes = collect_body(body, &request_id.0).await?;
    let batch = validation::validate(metadata, bytes)
        .map_err(|error| ApiError::invalid_batch(request_id.0.clone(), error.details()))?;

    match persistence::persist(&state.db, device.id, &batch)
        .await
        .map_err(|_| ApiError::internal(request_id.0.clone()))?
    {
        persistence::Persisted::New => Ok(Json(BatchAcknowledgement {
            batch_id: batch.batch_id,
            status: "committed",
            record_count: batch.record_count(),
            duplicate: false,
        })),
        persistence::Persisted::Replay => Ok(Json(BatchAcknowledgement {
            batch_id: batch.batch_id,
            status: "committed",
            record_count: batch.record_count(),
            duplicate: true,
        })),
        persistence::Persisted::Conflict => Err(ApiError::batch_conflict(request_id.0)),
    }
}

async fn collect_body(body: Body, request_id: &str) -> Result<Vec<u8>, ApiError> {
    let mut bytes = Vec::new();
    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ApiError::invalid_batch(request_id.to_owned(), None))?;
        if bytes.len().saturating_add(chunk.len()) > MAX_BATCH_BYTES {
            return Err(ApiError::body_too_large(request_id.to_owned()));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
