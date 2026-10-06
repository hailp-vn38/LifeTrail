//! Persist immutable UTC activity and daily candidates before fenced activation.
use super::{evidence, model::Input, snapshot, stops};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn stage(
    pool: &PgPool,
    input: &Input,
) -> Result<Vec<(chrono::NaiveDate, Uuid)>, sqlx::Error> {
    let manifest = Uuid::now_v7();
    let revision = Uuid::now_v7();
    let stops = stops::detect(&input.observations, &input.target, revision);
    let unresolved_intervals = evidence::unresolved(&input.observations, &stops, &input.target);
    let mut entries = json!([]);
    if let (Some(first), Some(last)) = (input.observations.first(), input.observations.last())
        && first.recorded_at < last.recorded_at
    {
        // This first activity slice processes the complete captured observation range.
        // Both replacement endpoints are genuine observation edges, never day cuts.
        let until = last.recorded_at + chrono::Duration::milliseconds(1);
        sqlx::query("INSERT INTO activity_revisions(id,device_id,observed_from_at,observed_until_at,input_generation,target_generation,config,body) VALUES($1,$2,$3,$4,$5,$6,$7::jsonb,$8::jsonb)")
            .bind(revision).bind(input.claim.device_id).bind(first.recorded_at).bind(until)
            .bind(input.target.input_generation).bind(input.target.target_generation)
            .bind(json!({"algorithm":"anchored-spatial-dwell-v1","radius_m":input.target.stop_radius_m,"minimum_duration_s":input.target.stop_min_duration_s,"observation_gap_s":input.target.observation_gap_s,"max_hdop":5}).to_string())
            .bind(json!({"stops":stops,"unresolved_intervals":unresolved_intervals}).to_string()).execute(pool).await?;
        entries = json!([{"activity_revision":revision,"from_at":first.recorded_at,"until_at":until,"from_boundary":"observation_edge","until_boundary":"observation_edge"}]);
    }
    sqlx::query("INSERT INTO activity_manifests(id,device_id,input_generation,target_id,entries) VALUES($1,$2,$3,$4,$5::jsonb)")
        .bind(manifest).bind(input.claim.device_id).bind(input.target.input_generation).bind(&input.target.target_id).bind(entries.to_string()).execute(pool).await?;
    let mut candidates = Vec::new();
    for day in &input.days {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO daily_snapshots(id,device_id,manifest_id,local_date,timezone,timezone_generation,source_generation,target_generation,body) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9::jsonb)")
            .bind(id).bind(input.claim.device_id).bind(manifest).bind(day.date).bind(&input.target.timezone)
            .bind(input.target.timezone_generation).bind(input.target.input_generation).bind(input.target.target_generation)
            .bind(snapshot::body(input,day,manifest,&stops,&unresolved_intervals).to_string()).execute(pool).await?;
        candidates.push((day.date, id));
    }
    Ok(candidates)
}
