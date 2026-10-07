mod persistence;
mod projection;

use axum::{
    Json,
    extract::{
        Extension, Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
};
use chrono::{NaiveDate, TimeZone as _};
use chrono_tz::Tz;
use uuid::Uuid;

use crate::{
    app::{AppState, RequestId},
    error::ApiError,
};

#[derive(serde::Deserialize, Default)]
pub(crate) struct ViewOptions {
    view: Option<ViewMode>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum ViewMode {
    Raw,
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    options: Result<Query<ViewOptions>, QueryRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Path((device_id, date)) =
        path.map_err(|_| ApiError::invalid_request(request_id.0.clone()))?;
    let date = parse_date(&date, &request_id.0)?;
    let Query(options) = options.map_err(|_| ApiError::invalid_request(request_id.0.clone()))?;
    let started = std::time::Instant::now();
    if options.view.is_none()
        && let Some(snapshot) = crate::processing::daily_snapshot(&state.db, device_id, date)
            .await
            .map_err(|_| ApiError::internal(request_id.0.clone()))?
    {
        // Schema drift: keep serving the full-resolution snowman (it stays
        // internally consistent) and lazily enqueue a projection-only requeue.
        if snapshot["processing"]["display_projection_stale"] == serde_json::Value::Bool(true) {
            crate::processing::queue_projection(&state.db, device_id, date)
                .await
                .map_err(|_| ApiError::internal(request_id.0.clone()))?;
        }
        tracing::debug!(
            %device_id,
            %date,
            daily_view_json_bytes = serde_json::to_string(&snapshot).map_or(0, |s| s.len()),
            latency_ms = started.elapsed().as_millis() as u64,
            "daily view served from snapshot"
        );
        return Ok(Json(snapshot));
    }
    let timezone = persistence::owner_timezone(&state.db, device_id)
        .await
        .map_err(|_| ApiError::internal(request_id.0.clone()))?
        .ok_or_else(|| ApiError::not_found(request_id.0.clone()))?;
    let timezone = timezone
        .parse::<Tz>()
        .map_err(|_| ApiError::internal(request_id.0.clone()))?;
    let bounds = local_day_bounds(date, timezone, &request_id.0)?;
    let points = persistence::route_points(&state.db, device_id, bounds.start, bounds.end)
        .await
        .map_err(|_| ApiError::internal(request_id.0.clone()))?;

    let mut raw = serde_json::to_value(projection::DailyView::from_points(
        device_id,
        date,
        timezone.name(),
        points,
    ))
    .map_err(|_| ApiError::internal(request_id.0.clone()))?;
    raw["processing"] = serde_json::to_value(
        crate::processing::status(&state.db, device_id, date)
            .await
            .map_err(|_| ApiError::internal(request_id.0.clone()))?,
    )
    .map_err(|_| ApiError::internal(request_id.0.clone()))?;
    Ok(Json(raw))
}

pub(crate) struct LocalDayBounds {
    pub(crate) start: chrono::DateTime<chrono::Utc>,
    pub(crate) end: chrono::DateTime<chrono::Utc>,
}

pub(crate) fn parse_date(value: &str, request_id: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| ApiError::invalid_request(request_id.to_owned()))
}

pub(crate) fn local_day_bounds(
    date: NaiveDate,
    timezone: Tz,
    request_id: &str,
) -> Result<LocalDayBounds, ApiError> {
    let start = local_midnight_utc(date, timezone, request_id)?;
    let next_date = date
        .succ_opt()
        .ok_or_else(|| ApiError::invalid_request(request_id.to_owned()))?;
    let end = local_midnight_utc(next_date, timezone, request_id)?;

    Ok(LocalDayBounds { start, end })
}

fn local_midnight_utc(
    date: NaiveDate,
    timezone: Tz,
    request_id: &str,
) -> Result<chrono::DateTime<chrono::Utc>, ApiError> {
    let midnight = timezone
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).expect("midnight is valid"))
        .single()
        .ok_or_else(|| ApiError::invalid_request(request_id.to_owned()))?;
    Ok(midnight.to_utc())
}

pub(crate) async fn status(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Path((device, date)) = path.map_err(|_| ApiError::invalid_request(request_id.0.clone()))?;
    let date = parse_date(&date, &request_id.0)?;
    let value = crate::processing::status(&state.db, device, date)
        .await
        .map_err(|_| ApiError::internal(request_id.0.clone()))?
        .ok_or_else(|| ApiError::not_found(request_id.0.clone()))?;
    Ok(Json(
        serde_json::to_value(value).map_err(|_| ApiError::internal(request_id.0))?,
    ))
}

#[derive(serde::Deserialize, Default)]
pub(crate) struct PlaybackOptions {
    manifest_version: Option<Uuid>,
}

/// Canonical full-resolution playback geometry for one Owner-local day.
///
/// Served lazily when the client engages playback. Canonical proof of having no
/// publication (raw / deleted / not processed) stays 404, matching the Daily
/// view; schema drift and stale manifest pins are 410.
pub(crate) async fn playback(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    options: Result<Query<PlaybackOptions>, QueryRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Path((device_id, date)) =
        path.map_err(|_| ApiError::invalid_request(request_id.0.clone()))?;
    let date = parse_date(&date, &request_id.0)?;
    let Query(options) = options.map_err(|_| ApiError::invalid_request(request_id.0.clone()))?;
    let started = std::time::Instant::now();
    match crate::processing::playback_source(&state.db, device_id, date, options.manifest_version)
        .await
        .map_err(|_| ApiError::internal(request_id.0.clone()))?
    {
        crate::processing::PlaybackResolution::Missing => Err(ApiError::not_found(request_id.0)),
        crate::processing::PlaybackResolution::Gone => Err(ApiError::gone(request_id.0)),
        crate::processing::PlaybackResolution::Ready(source) => {
            let timezone = persistence::owner_timezone(&state.db, device_id)
                .await
                .map_err(|_| ApiError::internal(request_id.0.clone()))?
                .ok_or_else(|| ApiError::not_found(request_id.0.clone()))?;
            let timezone = timezone
                .parse::<Tz>()
                .map_err(|_| ApiError::internal(request_id.0.clone()))?;
            let bounds = local_day_bounds(date, timezone, &request_id.0)?;
            let view = crate::processing::playback_view(
                &state.db,
                device_id,
                date,
                timezone.name(),
                bounds.start,
                bounds.end,
                &source,
            )
            .await
            .map_err(|_| ApiError::internal(request_id.0.clone()))?;
            tracing::debug!(
                %device_id,
                %date,
                manifest_version = %source.manifest_version,
                playback_json_bytes = serde_json::to_string(&view).map_or(0, |s| s.len()),
                latency_ms = started.elapsed().as_millis() as u64,
                "playback served"
            );
            Ok(Json(view))
        }
    }
}
