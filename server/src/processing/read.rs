use super::model::ProcessingStatus;
use chrono::NaiveDate;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct Publication {
    timezone: String,
    timezone_generation: i64,
    input_generation: i64,
    target_generation: i64,
    target_id: String,
    snapshot_target_id: Option<String>,
    job_state: Option<String>,
    day_state: Option<String>,
    failure_message: Option<String>,
    snapshot_id: Option<Uuid>,
    snapshot_manifest_id: Option<Uuid>,
    snapshot_projection_schema: Option<i32>,
    source_generation: Option<i64>,
    snapshot_target: Option<i64>,
    snapshot_timezone: Option<i64>,
    body: Option<String>,
}
async fn publication(
    pool: &PgPool,
    device: Uuid,
    date: NaiveDate,
) -> Result<Option<Publication>, sqlx::Error> {
    sqlx::query_as("SELECT u.timezone,u.timezone_generation,COALESCE(c.input_generation,0) AS input_generation, \
        COALESCE(c.target_generation,1) AS target_generation,COALESCE(c.target_id,'processed-gps-v1') AS target_id,m.target_id AS snapshot_target_id,j.state AS job_state,p.state AS day_state,j.failure_message, \
        s.id AS snapshot_id,s.manifest_id AS snapshot_manifest_id,(s.body->'provenance'->>'projection_schema_version')::int AS snapshot_projection_schema,s.source_generation,s.target_generation AS snapshot_target,s.timezone_generation AS snapshot_timezone,s.body::text AS body \
        FROM devices d JOIN users u ON u.id=d.owner_user_id \
        LEFT JOIN device_processing_control c ON c.device_id=d.id LEFT JOIN processing_jobs j ON j.device_id=d.id \
        LEFT JOIN processing_days p ON p.device_id=d.id AND p.local_date=$2 AND p.timezone=u.timezone \
        LEFT JOIN daily_publications a ON a.device_id=d.id AND a.local_date=$2 AND a.timezone=u.timezone \
        LEFT JOIN daily_snapshots s ON s.id=a.snapshot_id LEFT JOIN activity_manifests m ON m.id=s.manifest_id WHERE d.id=$1")
        .bind(device).bind(date).fetch_optional(pool).await
}
impl Publication {
    fn status(&self) -> ProcessingStatus {
        let usable_snapshot = self.snapshot_timezone == Some(self.timezone_generation);
        let published_revision = if usable_snapshot {
            self.snapshot_id
        } else {
            None
        };
        let data_freshness = if published_revision.is_none() {
            "unavailable"
        } else if self.source_generation == Some(self.input_generation)
            && self.snapshot_target == Some(self.target_generation)
            && self.snapshot_target_id.as_deref() == Some(self.target_id.as_str())
            && self.day_state.as_deref() == Some("done")
        {
            "current"
        } else if self.snapshot_target == Some(self.target_generation)
            && self.snapshot_target_id.as_deref() == Some(self.target_id.as_str())
        {
            "stale_source"
        } else {
            "stale"
        };
        let state = if self.day_state.as_deref() == Some("queued")
            || matches!(
                self.job_state.as_deref(),
                Some("queued" | "running" | "failed")
            ) {
            self.job_state.clone().unwrap_or_else(|| "queued".into())
        } else {
            "idle".into()
        };
        ProcessingStatus {
            state,
            data_freshness,
            published_revision,
            input_generation: self.input_generation,
            timezone: self.timezone.clone(),
            timezone_generation: self.timezone_generation,
            deferred_reason: (self.day_state.as_deref() == Some("deferred"))
                .then_some("activity_processing_not_available"),
            failure_message: self.failure_message.clone(),
            display_projection_stale: published_revision.is_some()
                && self.snapshot_projection_schema
                    != Some(super::reprojection::DAILY_PROJECTION_SCHEMA_VERSION),
        }
    }
}
pub async fn status(
    pool: &PgPool,
    device: Uuid,
    date: NaiveDate,
) -> Result<Option<ProcessingStatus>, sqlx::Error> {
    Ok(publication(pool, device, date)
        .await?
        .map(|row| row.status()))
}
pub async fn daily_snapshot(
    pool: &PgPool,
    device: Uuid,
    date: NaiveDate,
) -> Result<Option<Value>, sqlx::Error> {
    let Some(row) = publication(pool, device, date).await? else {
        return Ok(None);
    };
    let state = row.status();
    if state.published_revision.is_none() {
        return Ok(None);
    }
    let mut body: Value =
        serde_json::from_str(row.body.as_deref().ok_or(sqlx::Error::RowNotFound)?)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
    body["processing"] =
        serde_json::to_value(state).map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
    Ok(Some(body))
}

/// The published manifest backing a day's Playback resource.
pub struct PlaybackSource {
    pub manifest_id: Uuid,
    pub manifest_version: Uuid,
    pub projection_schema_version: i64,
    pub timezone: String,
}

/// Whether a day can serve canonical playback.
pub enum PlaybackResolution {
    /// Canonical geometry is available for the day's published manifest.
    Ready(PlaybackSource),
    /// The day has no usable publication.
    Missing,
    /// The publication is not schema v2, or the pinned manifest is gone: the
    /// client must drop its cache and reload.
    Gone,
}

/// Resolve the manifest that a day's Playback response should read from.
///
/// An unpinned request serves the current publication. A pinned request serves
/// exactly the pinned manifest while it still exists, so late data cannot mix
/// Route geometry from one manifest with a Timeline from another. A pinned
/// manifest that no longer exists is `Gone` (410).
pub async fn playback_source(
    pool: &PgPool,
    device: Uuid,
    date: NaiveDate,
    pin: Option<Uuid>,
) -> Result<PlaybackResolution, sqlx::Error> {
    let Some(row) = publication(pool, device, date).await? else {
        return Ok(PlaybackResolution::Missing);
    };
    if row.status().published_revision.is_none() {
        return Ok(PlaybackResolution::Missing);
    }
    let Some(current) = row.snapshot_manifest_id else {
        return Ok(PlaybackResolution::Missing);
    };
    let manifest = match pin {
        None => current,
        Some(pin) => {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM activity_manifests WHERE id=$1 AND device_id=$2)",
            )
            .bind(pin)
            .bind(device)
            .fetch_one(pool)
            .await?;
            if !exists {
                return Ok(PlaybackResolution::Gone);
            }
            pin
        }
    };
    Ok(PlaybackResolution::Ready(PlaybackSource {
        manifest_id: manifest,
        manifest_version: manifest,
        projection_schema_version: super::reprojection::DAILY_PROJECTION_SCHEMA_VERSION as i64,
        timezone: row.timezone,
    }))
}
