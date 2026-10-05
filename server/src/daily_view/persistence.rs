use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub(super) struct RoutePoint {
    pub(super) recorded_at: DateTime<Utc>,
    pub(super) lat: f64,
    pub(super) lon: f64,
}

pub(super) async fn owner_timezone(
    pool: &PgPool,
    device_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT users.timezone \
         FROM devices JOIN users ON users.id = devices.owner_user_id \
         WHERE devices.id = $1",
    )
    .bind(device_id)
    .fetch_optional(pool)
    .await
}

pub(super) async fn route_points(
    pool: &PgPool,
    device_id: Uuid,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<RoutePoint>, sqlx::Error> {
    sqlx::query_as(
        "SELECT recorded_at, lat, lon FROM gps_points \
         WHERE device_id = $1 AND fix_quality > 0 AND recorded_at >= $2 AND recorded_at < $3 \
         ORDER BY recorded_at ASC, id ASC",
    )
    .bind(device_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
}
