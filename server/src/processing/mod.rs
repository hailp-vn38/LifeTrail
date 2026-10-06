//! Durable activity derivation and fenced daily publication.
mod activation;
mod capture;
mod claim;
mod classification;
mod clip;
mod derived;
mod events;
mod gaps;
mod geo;
mod holes;
mod manifest;
mod model;
mod movement;
mod quality;
mod queue;
mod read;
mod reprojection;
mod route_parts;
mod snapshot;
mod staging;
mod stops;
mod trips;

pub use queue::queue_day;
pub(crate) use queue::{lock_device, schedule_batch};
pub use read::{daily_snapshot, status};
use sqlx::PgPool;

/// Process one durable claim. Also usable by the operator and integration seam.
pub async fn process_next(pool: &PgPool) -> Result<bool, sqlx::Error> {
    let Some(claim) = claim::claim(pool).await? else {
        return Ok(false);
    };
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
