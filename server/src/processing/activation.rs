use super::{
    claim,
    model::{Input, TARGET_COLUMNS, Target},
    staging,
};
use sqlx::PgPool;

pub(super) async fn activate(pool: &PgPool, input: Input) -> Result<(), sqlx::Error> {
    // Staging is intentionally outside an activation transaction.  Renew at
    // each durable boundary so a slow matcher cannot retain authority after a
    // different worker has reclaimed the lease.
    if !claim::renew(pool, input.claim).await? {
        claim::finish(pool, input.claim, "superseded").await?;
        return Ok(());
    }
    let candidates = staging::stage(pool, &input).await?;
    if !claim::renew(pool, input.claim).await? {
        claim::finish(pool, input.claim, "superseded").await?;
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    let current: Target=sqlx::query_as(&format!("SELECT {TARGET_COLUMNS} \
        FROM device_processing_control c JOIN devices d ON d.id=c.device_id JOIN users u ON u.id=d.owner_user_id WHERE c.device_id=$1 FOR UPDATE OF c FOR SHARE OF u"))
        .bind(input.claim.device_id).fetch_one(&mut *tx).await?;
    if current.fencing_token != input.claim.token {
        tx.rollback().await?;
        claim::finish(pool, input.claim, "superseded").await?;
        return Ok(());
    }
    if current.work_generation != input.target.work_generation
        || current.input_generation != input.target.input_generation
        || current.target_generation != input.target.target_generation
        || current.target_id != input.target.target_id
        || current.timezone != input.target.timezone
        || current.timezone_generation != input.target.timezone_generation
    {
        sqlx::query("UPDATE processing_jobs SET state='queued',lease_until=NULL WHERE device_id=$1 AND fencing_token=$2 AND state='running'")
            .bind(input.claim.device_id).bind(input.claim.token)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        claim::finish(pool, input.claim, "requeued").await?;
        return Ok(());
    }
    for (date, id) in &candidates {
        sqlx::query("INSERT INTO daily_publications(device_id,local_date,timezone,snapshot_id) VALUES($1,$2,$3,$4) ON CONFLICT(device_id,local_date,timezone) DO UPDATE SET snapshot_id=EXCLUDED.snapshot_id")
        .bind(input.claim.device_id).bind(date).bind(&current.timezone).bind(id).execute(&mut *tx).await?;
    }
    // This is the sole mutable manifest authority.  It changes in the same
    // short transaction as all affected Daily Snapshot pointers.
    let manifest: uuid::Uuid =
        sqlx::query_scalar("SELECT manifest_id FROM daily_snapshots WHERE id=$1")
            .bind(
                candidates
                    .first()
                    .map(|(_, id)| *id)
                    .ok_or(sqlx::Error::RowNotFound)?,
            )
            .fetch_one(&mut *tx)
            .await?;
    sqlx::query("UPDATE device_processing_control SET active_manifest_id=$2,dirty_from_at=NULL,dirty_until_at=NULL WHERE device_id=$1")
        .bind(input.claim.device_id).bind(manifest).execute(&mut *tx).await?;
    for day in &input.days {
        sqlx::query("INSERT INTO processing_days(device_id,local_date,timezone,state) VALUES($1,$2,$3,'done') ON CONFLICT(device_id,local_date,timezone) DO UPDATE SET state='done'")
            .bind(input.claim.device_id).bind(day.date).bind(&current.timezone).execute(&mut *tx).await?;
    }
    sqlx::query("UPDATE processing_jobs SET state='idle',lease_until=NULL WHERE device_id=$1 AND fencing_token=$2 AND state='running'")
        .bind(input.claim.device_id).bind(input.claim.token)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    claim::finish(pool, input.claim, "activated").await
}
