use super::{
    model::{Claim, Day, Input, Observation, TARGET_COLUMNS, Target},
    quality::{self, Policy},
};
use chrono::NaiveDate;
use sqlx::PgPool;

/// Capture one consistent input range: the processing target, every Raw
/// observation with its acquisition metadata, and the daily projection bounds.
///
/// Quality classification is derived here from the captured policy and the
/// captured Raw metadata together, so a published classification is always
/// reproducible from the provenance recorded with the Activity Revision.
pub(super) async fn capture(pool: &PgPool, claim: Claim) -> Result<Input, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let target: Target=sqlx::query_as(&format!("SELECT {TARGET_COLUMNS} \
        FROM device_processing_control c JOIN devices d ON d.id=c.device_id JOIN users u ON u.id=d.owner_user_id WHERE c.device_id=$1"))
        .bind(claim.device_id).fetch_one(&mut *tx).await?;
    let dates: Vec<NaiveDate>=sqlx::query_scalar("SELECT local_date FROM processing_days WHERE device_id=$1 AND timezone=$2 UNION SELECT (recorded_at AT TIME ZONE $2)::date FROM gps_points WHERE device_id=$1 ORDER BY local_date")
        .bind(claim.device_id).bind(&target.timezone).fetch_all(&mut *tx).await?;
    let mut observations: Vec<Observation>=sqlx::query_as("SELECT id,recorded_at,lat,lon,fix_quality,hdop,satellites,speed_mps FROM gps_points WHERE device_id=$1 ORDER BY recorded_at,id")
        .bind(claim.device_id).fetch_all(&mut *tx).await?;
    quality::classify(&mut observations, &Policy::from_target(&target));
    let mut days = Vec::new();
    for date in dates {
        let (from, until) = sqlx::query_as(
            "SELECT $1::date::timestamp AT TIME ZONE $2, ($1::date+1)::timestamp AT TIME ZONE $2",
        )
        .bind(date)
        .bind(&target.timezone)
        .fetch_one(&mut *tx)
        .await?;
        let points: Vec<_> = observations
            .iter()
            .filter(|p| p.recorded_at >= from && p.recorded_at < until)
            .collect();
        let point_count = points.len() as i64;
        let usable_count = points.iter().filter(|p| p.usable()).count() as i64;
        let first = points.first().map(|p| p.recorded_at);
        let last = points.last().map(|p| p.recorded_at);
        days.push(Day {
            date,
            from,
            until,
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
        observations,
    })
}
