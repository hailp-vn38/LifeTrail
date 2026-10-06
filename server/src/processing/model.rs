use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(super) struct Claim {
    pub device_id: Uuid,
    pub token: i64,
}
#[derive(sqlx::FromRow)]
pub(super) struct Target {
    pub input_generation: i64,
    pub work_generation: i64,
    pub target_generation: i64,
    pub target_id: String,
    pub timezone: String,
    pub timezone_generation: i64,
    pub fencing_token: i64,
}
pub(super) struct Input {
    pub claim: Claim,
    pub target: Target,
    pub days: Vec<Day>,
}
pub(super) struct Day {
    pub date: NaiveDate,
    pub point_count: i64,
    pub usable_count: i64,
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
