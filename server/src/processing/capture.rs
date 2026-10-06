use super::model::{Claim, Day, Input, Target};
use chrono::NaiveDate;
use sqlx::PgPool;

pub(super) async fn capture(pool: &PgPool, claim: Claim) -> Result<Input, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let target: Target=sqlx::query_as("SELECT c.input_generation,c.target_generation,c.target_id,c.fencing_token,u.timezone,u.timezone_generation \
        FROM device_processing_control c JOIN devices d ON d.id=c.device_id JOIN users u ON u.id=d.owner_user_id WHERE c.device_id=$1")
        .bind(claim.device_id).fetch_one(&mut *tx).await?;
    let dates: Vec<NaiveDate>=sqlx::query_scalar("SELECT local_date FROM processing_days WHERE device_id=$1 AND timezone=$2 AND state='queued' ORDER BY local_date")
        .bind(claim.device_id).bind(&target.timezone).fetch_all(&mut *tx).await?;
    let mut days = Vec::new();
    for date in dates {
        let (point_count,usable_count,first,last)=sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE fix_quality>0),min(recorded_at),max(recorded_at) \
            FROM gps_points WHERE device_id=$1 AND (recorded_at AT TIME ZONE $2)::date=$3")
            .bind(claim.device_id).bind(&target.timezone).bind(date).fetch_one(&mut *tx).await?;
        days.push(Day {
            date,
            point_count,
            usable_count,
            first,
            last,
        });
    }
    tx.commit().await?;
    Ok(Input {
        claim,
        target,
        days,
    })
}
