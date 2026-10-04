use serde::Serialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use crate::auth::digest_token;

pub const MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Owner {
    pub id: Uuid,
    pub display_name: String,
    pub timezone: String,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Device {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateOwnerError {
    #[error("an Owner already exists for this deployment")]
    OwnerAlreadyExists,
    #[error("timezone must be a valid IANA timezone")]
    InvalidTimezone,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

pub async fn create_owner(
    pool: &PgPool,
    display_name: &str,
    timezone: &str,
) -> Result<Owner, CreateOwnerError> {
    if timezone.parse::<chrono_tz::Tz>().is_err() {
        return Err(CreateOwnerError::InvalidTimezone);
    }
    sqlx::query_as::<_, Owner>(
        "INSERT INTO users (id, display_name, timezone) VALUES ($1, $2, $3) \
         RETURNING id, display_name, timezone",
    )
    .bind(Uuid::now_v7())
    .bind(display_name)
    .bind(timezone)
    .fetch_one(pool)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .and_then(|database_error| database_error.code())
            .as_deref()
            == Some("23505")
        {
            CreateOwnerError::OwnerAlreadyExists
        } else {
            CreateOwnerError::Database(error)
        }
    })
}

pub async fn create_device(
    pool: &PgPool,
    owner_user_id: Uuid,
    name: &str,
    token: &str,
) -> Result<Device, sqlx::Error> {
    sqlx::query_as::<_, Device>(
        "INSERT INTO devices (id, owner_user_id, name, token_digest) VALUES ($1, $2, $3, $4) \
         RETURNING id, owner_user_id, name",
    )
    .bind(Uuid::now_v7())
    .bind(owner_user_id)
    .bind(name)
    .bind(digest_token(token))
    .fetch_one(pool)
    .await
}

pub async fn list_devices(pool: &PgPool) -> Result<Vec<Device>, sqlx::Error> {
    sqlx::query_as::<_, Device>(
        "SELECT id, owner_user_id, name FROM devices ORDER BY created_at ASC, id ASC",
    )
    .fetch_all(pool)
    .await
}

pub async fn find_device(pool: &PgPool, id: Uuid) -> Result<Option<Device>, sqlx::Error> {
    sqlx::query_as::<_, Device>("SELECT id, owner_user_id, name FROM devices WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn resolve_device_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<Device>, sqlx::Error> {
    sqlx::query_as::<_, Device>(
        "SELECT id, owner_user_id, name FROM devices WHERE token_digest = $1",
    )
    .bind(digest_token(token))
    .fetch_optional(pool)
    .await
}
