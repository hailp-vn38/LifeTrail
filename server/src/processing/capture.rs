use super::{
    model::{Claim, Day, Input, Observation, TARGET_COLUMNS, Target},
    quality::{self, QualityClass},
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
    // A snapshot in this timezone/generation means an operator request is an
    // activity refresh.  Its absence after a timezone change means projection
    // work may reuse the active immutable manifest, including while Raw is
    // dirty for a newer generation.
    let projection_only = target.active_manifest_id.is_some()
        && !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM daily_publications p JOIN daily_snapshots s ON s.id=p.snapshot_id WHERE p.device_id=$1 AND p.timezone=$2 AND s.timezone_generation=$3)")
            .bind(claim.device_id).bind(&target.timezone).bind(target.timezone_generation)
            .fetch_one(&mut *tx).await?;
    let dates: Vec<NaiveDate>=sqlx::query_scalar("SELECT local_date FROM processing_days WHERE device_id=$1 AND timezone=$2 UNION SELECT (recorded_at AT TIME ZONE $2)::date FROM gps_points WHERE device_id=$1 ORDER BY local_date")
        .bind(claim.device_id).bind(&target.timezone).fetch_all(&mut *tx).await?;
    let mut observations: Vec<Observation>=sqlx::query_as("SELECT id,recorded_at,lat,lon,fix_quality,hdop,satellites,speed_mps FROM gps_points WHERE device_id=$1 ORDER BY recorded_at,id")
        .bind(claim.device_id).fetch_all(&mut *tx).await?;
    if !projection_only {
        quality::classify(&mut observations, &target.policy);
    }
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
        // Each class is counted from the classifier's own variants. Deriving the
        // poorer classes by subtraction would publish a low-quality record as an
        // impossible one and collapse the three-way distinction.
        let count = |class: QualityClass| {
            points.iter().filter(|p| p.classification == class).count() as i64
        };
        let point_count = points.len() as i64;
        let first = points.first().map(|p| p.recorded_at);
        let last = points.last().map(|p| p.recorded_at);
        days.push(Day {
            date,
            from,
            until,
            point_count,
            // Reprojection does not re-run quality classification. The raw
            // count remains truthful; class-specific counts are intentionally
            // unavailable rather than recalculated under a new projection.
            usable_count: (!projection_only)
                .then(|| count(QualityClass::Usable))
                .unwrap_or(0),
            low_quality_count: (!projection_only)
                .then(|| count(QualityClass::LowQuality))
                .unwrap_or(0),
            excluded_count: (!projection_only)
                .then(|| count(QualityClass::Excluded))
                .unwrap_or(0),
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
        projection_only,
    })
}
