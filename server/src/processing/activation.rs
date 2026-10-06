use super::{
    model::{Input, TARGET_COLUMNS, Target},
    staging,
};
use sqlx::PgPool;

pub(super) async fn activate(pool: &PgPool, input: Input) -> Result<(), sqlx::Error> {
    let staged = staging::stage(pool, &input).await?;
    let candidates = &staged.candidates;
    let mut tx = pool.begin().await?;
    let current: Target=sqlx::query_as(&format!("SELECT {TARGET_COLUMNS} \
        FROM device_processing_control c JOIN devices d ON d.id=c.device_id JOIN users u ON u.id=d.owner_user_id WHERE c.device_id=$1 FOR UPDATE OF c FOR SHARE OF u"))
        .bind(input.claim.device_id).fetch_one(&mut *tx).await?;
    if current.fencing_token != input.claim.token {
        return Ok(());
    }
    if current.work_generation != input.target.work_generation
        || current.input_generation != input.target.input_generation
        || current.target_generation != input.target.target_generation
        || current.target_id != input.target.target_id
        || current.timezone != input.target.timezone
        || current.timezone_generation != input.target.timezone_generation
        || (staged.projection_only && current.active_manifest_id != Some(staged.manifest))
    {
        sqlx::query("UPDATE processing_jobs SET state='queued' WHERE device_id=$1")
            .bind(input.claim.device_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(());
    }
    for (date, id) in candidates {
        sqlx::query("INSERT INTO daily_publications(device_id,local_date,timezone,snapshot_id) VALUES($1,$2,$3,$4) ON CONFLICT(device_id,local_date,timezone) DO UPDATE SET snapshot_id=EXCLUDED.snapshot_id")
        .bind(input.claim.device_id).bind(date).bind(&current.timezone).bind(id).execute(&mut *tx).await?;
    }
    if !staged.projection_only {
        // This is the sole mutable manifest authority. It changes alongside
        // activity publication, never during projection-only activation.
        sqlx::query("UPDATE device_processing_control SET active_manifest_id=$2,dirty_from_at=NULL,dirty_until_at=NULL WHERE device_id=$1")
            .bind(input.claim.device_id).bind(staged.manifest).execute(&mut *tx).await?;
    }
    for day in &input.days {
        sqlx::query("INSERT INTO processing_days(device_id,local_date,timezone,state) VALUES($1,$2,$3,'done') ON CONFLICT(device_id,local_date,timezone) DO UPDATE SET state='done'")
            .bind(input.claim.device_id).bind(day.date).bind(&current.timezone).execute(&mut *tx).await?;
    }
    sqlx::query("UPDATE processing_jobs SET state='idle',lease_until=NULL WHERE device_id=$1")
        .bind(input.claim.device_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}
