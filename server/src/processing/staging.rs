//! Persist immutable UTC activity and daily candidates before fenced activation.
use super::{
    derived::{self, REDUCER_VERSION},
    manifest::{self, Slice},
    model::Input,
    reprojection, snapshot,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) struct Staged {
    pub candidates: Vec<(chrono::NaiveDate, Uuid)>,
    pub manifest: Uuid,
    pub projection_only: bool,
}

pub(super) async fn stage(pool: &PgPool, input: &Input) -> Result<Staged, sqlx::Error> {
    if input.projection_only {
        let manifest = input
            .target
            .active_manifest_id
            .ok_or(sqlx::Error::RowNotFound)?;
        let source_generation: i64 = sqlx::query_scalar(
            "SELECT input_generation FROM activity_manifests WHERE id=$1 AND device_id=$2",
        )
        .bind(manifest)
        .bind(input.claim.device_id)
        .fetch_one(pool)
        .await?;
        let mut candidates = Vec::new();
        for day in &input.days {
            let id = Uuid::now_v7();
            let body = reprojection::body(pool, input, day, manifest).await?;
            sqlx::query("INSERT INTO daily_snapshots(id,device_id,manifest_id,local_date,timezone,timezone_generation,source_generation,target_generation,body) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9::jsonb)")
                .bind(id).bind(input.claim.device_id).bind(manifest).bind(day.date).bind(&input.target.timezone)
                .bind(input.target.timezone_generation).bind(source_generation).bind(input.target.target_generation).bind(body.to_string()).execute(pool).await?;
            candidates.push((day.date, id));
        }
        return Ok(Staged {
            candidates,
            manifest,
            projection_only: true,
        });
    }
    let manifest = Uuid::now_v7();
    let revision = Uuid::now_v7();
    let derived = derived::derive(&input.observations, &input.target, revision).await;
    let previous =
        previous_slices(pool, input.claim.device_id, input.target.active_manifest_id).await?;
    let mut entries = json!([]);
    if let (Some(first), Some(last)) = (input.observations.first(), input.observations.last())
        && first.recorded_at < last.recorded_at
    {
        // This activity slice processes the complete captured observation range.
        // Both replacement endpoints are genuine observation edges, never day cuts.
        let until = last.recorded_at + chrono::Duration::milliseconds(1);
        sqlx::query("INSERT INTO activity_revisions(id,device_id,observed_from_at,observed_until_at,supersedes_from_at,supersedes_until_at,input_generation,target_generation,config,body) VALUES($1,$2,$3,$4,$3,$4,$5,$6,$7::jsonb,$8::jsonb)")
            .bind(revision).bind(input.claim.device_id).bind(first.recorded_at).bind(until)
            .bind(input.target.input_generation).bind(input.target.target_generation)
            .bind(config(input).to_string())
            .bind(json!({
                "stops":derived.stops,"trips":derived.trips,"gaps":derived.gaps,
                "route_parts":derived.parts,"evidence_holes":derived.evidence_holes
            }).to_string()).execute(pool).await?;
        let slices = manifest::splice(
            &previous,
            Slice {
                revision,
                from: first.recorded_at,
                until,
                // Dataset coverage is independently verified Raw observation
                // evidence.  It is not a processing or matcher chunk seam.
                from_boundary: "verified_observation_edge",
                until_boundary: "verified_observation_edge",
            },
        );
        entries = serde_json::to_value(slices.into_iter().map(|slice| json!({
            "activity_revision":slice.revision,"from_at":slice.from,"until_at":slice.until,
            "from_boundary":slice.from_boundary,"until_boundary":slice.until_boundary
        })).collect::<Vec<_>>()).expect("manifest entries serialize");
    }
    sqlx::query("INSERT INTO activity_manifests(id,device_id,input_generation,target_id,entries) VALUES($1,$2,$3,$4,$5::jsonb)")
        .bind(manifest).bind(input.claim.device_id).bind(input.target.input_generation).bind(&input.target.target_id).bind(entries.to_string()).execute(pool).await?;
    let mut candidates = Vec::new();
    for day in &input.days {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO daily_snapshots(id,device_id,manifest_id,local_date,timezone,timezone_generation,source_generation,target_generation,body) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9::jsonb)")
            .bind(id).bind(input.claim.device_id).bind(manifest).bind(day.date).bind(&input.target.timezone)
            .bind(input.target.timezone_generation).bind(input.target.input_generation).bind(input.target.target_generation)
            .bind(snapshot::body(input,day,manifest,&derived).to_string()).execute(pool).await?;
        candidates.push((day.date, id));
    }
    // Candidate and evidence artifacts are retained; record their size at the
    // time this attempt created them so capacity planning never needs to infer
    // it from whichever manifest happens to be active later.
    let revision_bytes: Option<i64> = sqlx::query_scalar(
        "SELECT octet_length(body::text) + octet_length(config::text) FROM activity_revisions WHERE id=$1",
    )
    .bind(revision)
    .fetch_optional(pool)
    .await?;
    let snapshot_bytes: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(octet_length(body::text)),0)::bigint FROM daily_snapshots WHERE manifest_id=$1",
    )
    .bind(manifest)
    .fetch_one(pool)
    .await?;
    let manifest_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM activity_manifests WHERE device_id=$1")
            .bind(input.claim.device_id)
            .fetch_one(pool)
            .await?;
    let matcher_evidence_bytes: i64 = derived
        .parts
        .iter()
        .filter_map(|part| part.matcher_evidence.as_ref())
        .map(|evidence| {
            serde_json::to_string(evidence)
                .expect("matcher evidence serializes")
                .len() as i64
        })
        .sum();
    sqlx::query("INSERT INTO processing_storage_measurements(id,device_id,fencing_token,activity_revision_bytes,manifest_count,snapshot_bytes,candidate_bytes,matcher_evidence_bytes) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(Uuid::now_v7()).bind(input.claim.device_id).bind(input.claim.token)
        .bind(revision_bytes).bind(manifest_count).bind(snapshot_bytes)
        .bind(revision_bytes.unwrap_or(0) + snapshot_bytes)
        .bind(matcher_evidence_bytes)
        .execute(pool).await?;
    Ok(Staged {
        candidates,
        manifest,
        projection_only: false,
    })
}

#[derive(serde::Deserialize)]
struct StoredSlice {
    activity_revision: Uuid,
    from_at: chrono::DateTime<chrono::Utc>,
    until_at: chrono::DateTime<chrono::Utc>,
}

async fn previous_slices(
    pool: &PgPool,
    device: Uuid,
    manifest: Option<Uuid>,
) -> Result<Vec<Slice>, sqlx::Error> {
    let Some(manifest) = manifest else {
        return Ok(Vec::new());
    };
    let entries: Value =
        sqlx::query_scalar("SELECT entries FROM activity_manifests WHERE id=$1 AND device_id=$2")
            .bind(manifest)
            .bind(device)
            .fetch_optional(pool)
            .await?
            .unwrap_or_else(|| json!([]));
    let stored: Vec<StoredSlice> =
        serde_json::from_value(entries).map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    Ok(stored
        .into_iter()
        .map(|slice| Slice {
            revision: slice.activity_revision,
            from: slice.from_at,
            until: slice.until_at,
            // Existing safe entries stay authoritative outside the splice. Their
            // original boundary evidence is retained in their immutable manifest.
            from_boundary: "retained_safe_slice",
            until_boundary: "retained_safe_slice",
        })
        .collect())
}

/// Algorithm and configuration identity retained with each Activity Revision.
fn config(input: &Input) -> Value {
    json!({
        "algorithms":["anchored-spatial-dwell-v1","continuous-movement-multimode-v1","windowed-mode-evidence-hysteresis-v1","short-osrm-match-v1","raw-quality-classification-v1","observed-gap-detection-v1"],
        "radius_m":input.target.stop_radius_m,"minimum_duration_s":input.target.stop_min_duration_s,
        "observation_gap_s":input.target.observation_gap_s,
        "max_hdop":input.target.policy.max_hdop,
        "max_implied_speed_mps":input.target.policy.max_implied_speed_mps,
        "jump_distance_floor_m":input.target.policy.jump_distance_floor_m,
        "mode_window_s":input.target.mode_window_s,
        "mode_change_min_duration_s":input.target.mode_change_min_duration_s,
        "mode_enter_confidence":input.target.mode_enter_confidence,
        "mode_exit_confidence":input.target.mode_exit_confidence,
        "mode_unknown_grace_s":input.target.mode_unknown_grace_s,
        "match_min_confidence":input.target.match_min_confidence,
        "match_max_attempts":input.target.match_max_attempts,
        "match_retry_delay_ms":input.target.match_retry_delay_ms,
        "match_total_budget_ms":input.target.match_total_budget_ms,
        "matcher_engine_id":input.target.matcher_engine_id,
        "matcher_dataset_id":input.target.matcher_dataset_id,
        "reducer_version":REDUCER_VERSION
    })
}
