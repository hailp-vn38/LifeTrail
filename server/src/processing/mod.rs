//! Durable activity derivation and fenced daily publication.
mod activation;
mod capture;
mod claim;
mod classification;
mod continuity;
mod derived;
mod display_geometry;
mod gaps;
mod geo;
mod holes;
mod manifest;
mod model;
mod movement;
mod progress_projection;
mod quality;
mod queue;
mod read;
mod replacement;
mod reprojection;
mod route_parts;
mod staging;
mod stops;
mod trips;

pub use queue::{backfill_projection_schema, queue_day, queue_projection};
pub(crate) use queue::{lock_device, schedule_batch};
pub use read::{PlaybackResolution, daily_snapshot, playback_source, status};

/// Build the canonical Playback response for one day from its published manifest.
pub async fn playback_view(
    pool: &PgPool,
    device: Uuid,
    date: chrono::NaiveDate,
    timezone: &str,
    from: chrono::DateTime<chrono::Utc>,
    until: chrono::DateTime<chrono::Utc>,
    source: &read::PlaybackSource,
) -> Result<serde_json::Value, sqlx::Error> {
    let day = model::Day::for_bounds(date, from, until);
    let parts = reprojection::route_parts_for_day(pool, device, source.manifest_id, &day).await?;
    Ok(serde_json::json!({
        "device_id": device,
        "date": date,
        "timezone": timezone,
        "manifest_version": source.manifest_version,
        "projection_schema_version": source.projection_schema_version,
        "route_parts": parts,
        "provenance": {
            "manifest_version": source.manifest_version,
            "projection_schema_version": source.projection_schema_version,
        },
    }))
}
use sqlx::PgPool;
use uuid::Uuid;

/// Process one durable claim. Also usable by the operator and integration seam.
pub async fn process_next(pool: &PgPool) -> Result<bool, sqlx::Error> {
    let Some(claim) = claim::claim(pool).await? else {
        return Ok(false);
    };
    tracing::info!(device_id = %claim.device_id, attempt = claim.attempt, fencing_token = claim.token, "processing attempt claimed");
    let result = async {
        let input = capture::capture(pool, claim).await?;
        activation::activate(pool, input).await
    }
    .await;
    if let Err(error) = &result {
        claim::fail(pool, claim, &error.to_string()).await?;
    }
    result.map(|()| true)
}

pub async fn run(pool: PgPool) {
    loop {
        match process_next(&pool).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(error) => tracing::error!(%error, "processing job failed"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
