use super::model::Claim;
use sqlx::PgPool;

pub(super) async fn claim(pool: &PgPool) -> Result<Option<Claim>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let device: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT c.device_id FROM device_processing_control c \
        JOIN processing_jobs j ON j.device_id=c.device_id \
        WHERE j.state='queued' OR (j.state='running' AND j.lease_until < now()) \
        ORDER BY c.device_id FOR UPDATE OF c SKIP LOCKED LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(device_id) = device else {
        return Ok(None);
    };
    let token=sqlx::query_scalar("UPDATE device_processing_control SET fencing_token=fencing_token+1 WHERE device_id=$1 RETURNING fencing_token")
        .bind(device_id).fetch_one(&mut *tx).await?;
    sqlx::query("UPDATE processing_jobs SET state='running', fencing_token=$2, lease_until=now()+interval '30 seconds', failure_message=NULL WHERE device_id=$1")
        .bind(device_id).bind(token).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Some(Claim { device_id, token }))
}
pub(super) async fn fail(pool: &PgPool, claim: Claim, message: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE processing_jobs SET state='failed',failure_message=$3 WHERE device_id=$1 AND fencing_token=$2 AND state='running'")
        .bind(claim.device_id).bind(claim.token).bind(message).execute(pool).await?;
    Ok(())
}
