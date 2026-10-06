use sqlx::PgPool;
use uuid::Uuid;

use super::validation::ValidatedBatch;

pub(super) enum Persisted {
    New,
    Replay,
    Conflict,
}

pub(super) async fn persist(
    pool: &PgPool,
    device_id: Uuid,
    batch: &ValidatedBatch,
) -> Result<Persisted, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    crate::processing::lock_device(&mut transaction, device_id).await?;
    let inserted = sqlx::query_scalar::<_, i32>(
        "INSERT INTO ingest_batches \
         (device_id, batch_id, schema_name, content_sha256, byte_length, record_count, first_ts_ms, last_ts_ms) \
         VALUES ($1, $2, 'gps/1', $3, $4, $5, $6, $7) \
         ON CONFLICT (device_id, batch_id) DO NOTHING \
         RETURNING 1",
    )
    .bind(device_id)
    .bind(batch.batch_id)
    .bind(&batch.hash)
    .bind(batch.byte_length as i64)
    .bind(batch.record_count() as i64)
    .bind(batch.records.first().expect("validated records").ts_ms)
    .bind(batch.records.last().expect("validated records").ts_ms)
    .fetch_optional(&mut *transaction)
    .await?;

    if inserted.is_none() {
        let existing_hash: Vec<u8> = sqlx::query_scalar(
            "SELECT content_sha256 FROM ingest_batches WHERE device_id = $1 AND batch_id = $2",
        )
        .bind(device_id)
        .bind(batch.batch_id)
        .fetch_one(&mut *transaction)
        .await?;
        transaction.commit().await?;
        return Ok(if existing_hash == batch.hash {
            Persisted::Replay
        } else {
            Persisted::Conflict
        });
    }

    for record in &batch.records {
        sqlx::query(
            "INSERT INTO gps_points \
             (device_id, batch_id, recorded_at, lat, lon, alt_m, speed_mps, course_deg, fix_quality, satellites, hdop, geometry) \
             VALUES ($1, $2, to_timestamp($3::double precision / 1000), $4, $5, $6, $7, $8, $9, $10, $11, \
                     ST_SetSRID(ST_MakePoint($5, $4), 4326))",
        )
        .bind(device_id)
        .bind(batch.batch_id)
        .bind(record.ts_ms)
        .bind(record.lat)
        .bind(record.lon)
        .bind(record.alt_m)
        .bind(record.speed_mps)
        .bind(record.course_deg)
        .bind(record.fix_quality)
        .bind(record.satellites)
        .bind(record.hdop)
        .execute(&mut *transaction)
        .await?;
    }
    crate::processing::schedule_batch(&mut transaction, device_id, batch.batch_id).await?;
    transaction.commit().await?;
    Ok(Persisted::New)
}
