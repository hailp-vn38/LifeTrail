use super::{
    model::{Input, Target},
    snapshot,
};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn activate(pool: &PgPool, input: Input) -> Result<(), sqlx::Error> {
    // Stage immutable candidate rows before the short pointer-swap transaction.
    let manifest = Uuid::now_v7();
    let sparse: Vec<_> = input
        .days
        .iter()
        .filter(|day| day.point_count <= 1)
        .collect();
    let mut candidates = Vec::new();
    if !sparse.is_empty() {
        sqlx::query("INSERT INTO activity_manifests(id,device_id,input_generation,target_id,entries) VALUES($1,$2,$3,$4,'[]')")
            .bind(manifest).bind(input.claim.device_id).bind(input.target.input_generation).bind(&input.target.target_id).execute(pool).await?;
        for day in sparse {
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO daily_snapshots(id,device_id,manifest_id,local_date,timezone,timezone_generation,source_generation,target_generation,body) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9::jsonb)")
                .bind(id).bind(input.claim.device_id).bind(manifest).bind(day.date).bind(&input.target.timezone)
                .bind(input.target.timezone_generation).bind(input.target.input_generation).bind(input.target.target_generation)
                .bind(snapshot::body(&input,day,manifest).to_string()).execute(pool).await?;
            candidates.push((day.date, id));
        }
    }
    let mut tx = pool.begin().await?;
    let current: Target=sqlx::query_as("SELECT c.input_generation,c.work_generation,c.target_generation,c.target_id,c.fencing_token,u.timezone,u.timezone_generation \
        FROM device_processing_control c JOIN devices d ON d.id=c.device_id JOIN users u ON u.id=d.owner_user_id WHERE c.device_id=$1 FOR UPDATE OF c FOR SHARE OF u")
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
    for day in &input.days {
        sqlx::query("UPDATE processing_days SET state=$4 WHERE device_id=$1 AND local_date=$2 AND timezone=$3")
            .bind(input.claim.device_id).bind(day.date).bind(&current.timezone).bind(if day.point_count<=1 {"done"} else {"deferred"}).execute(&mut *tx).await?;
    }
    sqlx::query("UPDATE processing_jobs SET state='idle',lease_until=NULL WHERE device_id=$1")
        .bind(input.claim.device_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}
