mod persistence;
mod projection;

use axum::{
    Json,
    extract::{Extension, Path, State, rejection::PathRejection},
};
use chrono::{NaiveDate, TimeZone as _};
use chrono_tz::Tz;
use uuid::Uuid;

use crate::{
    app::{AppState, RequestId},
    error::ApiError,
};

pub(crate) async fn get(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Path((device_id, date)) =
        path.map_err(|_| ApiError::invalid_request(request_id.0.clone()))?;
    let date = parse_date(&date, &request_id.0)?;
    if let Some(snapshot) = crate::processing::daily_snapshot(&state.db, device_id, date)
        .await
        .map_err(|_| ApiError::internal(request_id.0.clone()))?
    {
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

struct LocalDayBounds {
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
}

fn parse_date(value: &str, request_id: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| ApiError::invalid_request(request_id.to_owned()))
}

fn local_day_bounds(
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
