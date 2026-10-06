use super::model::Claim;
use sqlx::PgPool;

const LEASE: &str = "30 seconds";

pub(super) async fn claim(pool: &PgPool) -> Result<Option<Claim>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let device: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT c.device_id FROM device_processing_control c \
        JOIN processing_jobs j ON j.device_id=c.device_id \
        WHERE j.state IN ('queued','failed') OR (j.state='running' AND j.lease_until < now()) \
        ORDER BY c.device_id FOR UPDATE OF c SKIP LOCKED LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(device_id) = device else {
        return Ok(None);
    };
    let token=sqlx::query_scalar("UPDATE device_processing_control SET fencing_token=fencing_token+1 WHERE device_id=$1 RETURNING fencing_token")
        .bind(device_id).fetch_one(&mut *tx).await?;
    let attempt: i64 = sqlx::query_scalar(&format!("UPDATE processing_jobs SET state='running', fencing_token=$2, lease_until=now()+interval '{LEASE}', failure_message=NULL, attempt_count=attempt_count+1, claimed_at=now(), claimed_work_generation=c.work_generation, claimed_input_generation=c.input_generation, claimed_target_generation=c.target_generation, claimed_target_id=c.target_id FROM device_processing_control c WHERE processing_jobs.device_id=$1 AND c.device_id=$1 RETURNING processing_jobs.attempt_count"))
        .bind(device_id).bind(token).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO processing_attempts(device_id,fencing_token,attempt,work_generation,input_generation,target_generation,target_id) SELECT c.device_id,$2,$3,c.work_generation,c.input_generation,c.target_generation,c.target_id FROM device_processing_control c WHERE c.device_id=$1")
        .bind(device_id).bind(token).bind(attempt).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Some(Claim {
        device_id,
        token,
        attempt,
    }))
}

/// Renew only the lease held by this fencing token.  A reclaimed worker cannot
/// extend a newer worker's authority.
pub(super) async fn renew(pool: &PgPool, claim: Claim) -> Result<bool, sqlx::Error> {
    let renewed = sqlx::query("UPDATE processing_jobs SET lease_until=now()+interval '30 seconds' WHERE device_id=$1 AND fencing_token=$2 AND state='running' AND lease_until >= now()")
        .bind(claim.device_id).bind(claim.token).execute(pool).await?.rows_affected() == 1;
    if renewed {
        sqlx::query("UPDATE processing_attempts SET renewed_at=now() WHERE device_id=$1 AND fencing_token=$2 AND finished_at IS NULL")
            .bind(claim.device_id).bind(claim.token).execute(pool).await?;
    }
    Ok(renewed)
}

pub(super) async fn finish(pool: &PgPool, claim: Claim, outcome: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE processing_attempts SET finished_at=now(),outcome=$3 WHERE device_id=$1 AND fencing_token=$2 AND finished_at IS NULL")
        .bind(claim.device_id).bind(claim.token).bind(outcome).execute(pool).await?;
    Ok(())
}
pub(super) async fn fail(pool: &PgPool, claim: Claim, message: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE processing_jobs SET state='failed',failure_message=$3 WHERE device_id=$1 AND fencing_token=$2 AND state='running'")
        .bind(claim.device_id).bind(claim.token).bind(message).execute(pool).await?;
    finish(pool, claim, "failed").await?;
    Ok(())
}
