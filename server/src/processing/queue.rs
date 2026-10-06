use chrono::NaiveDate;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub(crate) async fn lock_device(
    tx: &mut Transaction<'_, Postgres>,
    device: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO device_processing_control (device_id) VALUES ($1) ON CONFLICT DO NOTHING",
    )
    .bind(device)
    .execute(&mut **tx)
    .await?;
    sqlx::query("SELECT device_id FROM device_processing_control WHERE device_id=$1 FOR UPDATE")
        .bind(device)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub(crate) async fn schedule_batch(
    tx: &mut Transaction<'_, Postgres>,
    device: Uuid,
    batch: Uuid,
) -> Result<(), sqlx::Error> {
    // Bounds coalesce every late Batch in one durable Device-owned range.  We
    // only mark interpretation stale; the last complete Daily Snapshot remains
    // the read source until activation succeeds.
    sqlx::query("UPDATE device_processing_control c SET input_generation=c.input_generation+1, dirty_from_at=CASE WHEN c.dirty_from_at IS NULL THEN b.first_at ELSE LEAST(c.dirty_from_at,b.first_at) END, dirty_until_at=CASE WHEN c.dirty_until_at IS NULL THEN b.until_at ELSE GREATEST(c.dirty_until_at,b.until_at) END FROM (SELECT min(recorded_at) AS first_at, max(recorded_at)+interval '1 millisecond' AS until_at FROM gps_points WHERE device_id=$1 AND batch_id=$2) b WHERE c.device_id=$1")
        .bind(device).bind(batch).execute(&mut **tx).await?;
    sqlx::query(
        "INSERT INTO processing_days (device_id, local_date, timezone, state) \
        SELECT DISTINCT $1, (g.recorded_at AT TIME ZONE u.timezone)::date, u.timezone, 'queued' \
        FROM gps_points g JOIN devices d ON d.id=g.device_id JOIN users u ON u.id=d.owner_user_id \
        WHERE g.device_id=$1 AND g.batch_id=$2 \
        ON CONFLICT(device_id,local_date,timezone) DO UPDATE SET state='queued'",
    )
    .bind(device)
    .bind(batch)
    .execute(&mut **tx)
    .await?;
    enqueue(tx, device).await
}
pub async fn queue_day(pool: &PgPool, device: Uuid, date: NaiveDate) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_device(&mut tx, device).await?;
    sqlx::query("INSERT INTO processing_days(device_id,local_date,timezone,state) \
        SELECT d.id,$2,u.timezone,'queued' FROM devices d JOIN users u ON u.id=d.owner_user_id WHERE d.id=$1 \
        ON CONFLICT(device_id,local_date,timezone) DO UPDATE SET state='queued'")
        .bind(device).bind(date).execute(&mut *tx).await?;
    enqueue(&mut tx, device).await?;
    tx.commit().await
}
async fn enqueue(tx: &mut Transaction<'_, Postgres>, device: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE device_processing_control SET work_generation=work_generation+1 WHERE device_id=$1",
    )
    .bind(device)
    .execute(&mut **tx)
    .await?;
    sqlx::query("INSERT INTO processing_jobs(device_id,state) VALUES($1,'queued') \
        ON CONFLICT(device_id) DO UPDATE SET state=CASE WHEN processing_jobs.state='running' THEN 'running' ELSE 'queued' END, failure_message=NULL")
        .bind(device).execute(&mut **tx).await?;
    Ok(())
}
