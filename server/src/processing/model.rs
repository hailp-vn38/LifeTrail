use super::quality::{self, QualityClass};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(super) struct Claim {
    pub device_id: Uuid,
    pub token: i64,
}
/// Processing target columns, shared by capture and activation so the identity
/// checked at activation is always the identity that was captured.
pub(super) const TARGET_COLUMNS: &str = "c.input_generation,c.work_generation,c.target_generation,c.target_id,c.fencing_token,\
c.stop_radius_m,c.stop_min_duration_s,c.observation_gap_s,\
c.max_hdop,c.max_implied_speed_mps,c.jump_distance_floor_m,\
c.mode_window_s,c.mode_change_min_duration_s,c.mode_enter_confidence,c.mode_exit_confidence,c.mode_unknown_grace_s,\
c.match_min_confidence,c.match_max_attempts,c.match_retry_delay_ms,c.match_total_budget_ms,c.matcher_engine_id,c.matcher_dataset_id,\
u.timezone,u.timezone_generation";
/// The identity of one Device's processing configuration.
///
/// The target owns the quality policy: the same columns that select it also
/// carry the configured thresholds, so no caller copies policy fields into a
/// second configuration value that could drift from the captured one.
#[derive(sqlx::FromRow)]
pub(super) struct Target {
    pub input_generation: i64,
    pub work_generation: i64,
    pub target_generation: i64,
    pub target_id: String,
    pub timezone: String,
    pub timezone_generation: i64,
    pub fencing_token: i64,
    pub stop_radius_m: f64,
    pub stop_min_duration_s: i64,
    pub observation_gap_s: i64,
    pub mode_window_s: i64,
    pub mode_change_min_duration_s: i64,
    pub mode_enter_confidence: f64,
    pub mode_exit_confidence: f64,
    pub mode_unknown_grace_s: i64,
    pub match_min_confidence: f64,
    pub match_max_attempts: i64,
    pub match_retry_delay_ms: i64,
    pub match_total_budget_ms: i64,
    pub matcher_engine_id: String,
    pub matcher_dataset_id: String,
    #[sqlx(flatten)]
    pub policy: quality::Policy,
}
pub(super) struct Input {
    pub claim: Claim,
    pub target: Target,
    pub days: Vec<Day>,
    pub observations: Vec<Observation>,
}
/// One Owner-local day of captured Raw GPS, counted by classification.
///
/// Every GPS Record falls in exactly one class, so the three counts always
/// partition `point_count`.
pub(super) struct Day {
    pub date: NaiveDate,
    pub from: DateTime<Utc>,
    pub until: DateTime<Utc>,
    pub point_count: i64,
    pub usable_count: i64,
    pub low_quality_count: i64,
    pub excluded_count: i64,
    pub first: Option<DateTime<Utc>>,
    pub last: Option<DateTime<Utc>>,
}
#[derive(Serialize)]
pub struct ProcessingStatus {
    pub state: String,
    pub data_freshness: &'static str,
    pub published_revision: Option<Uuid>,
    pub input_generation: i64,
    pub timezone: String,
    pub timezone_generation: i64,
    pub deferred_reason: Option<&'static str>,
    pub failure_message: Option<String>,
}

/// One captured GPS Record with the Raw acquisition metadata that classifies it.
///
/// `classification` is computed by `quality::classify` from the captured policy,
/// never read from storage, so a published classification is reproducible from
/// the Raw metadata plus the recorded policy.
#[derive(sqlx::FromRow)]
pub(super) struct Observation {
    pub id: i64,
    pub recorded_at: DateTime<Utc>,
    pub lat: f64,
    pub lon: f64,
    pub fix_quality: i64,
    pub hdop: Option<f64>,
    pub satellites: i64,
    pub speed_mps: Option<f64>,
    #[sqlx(skip)]
    pub classification: QualityClass,
}
