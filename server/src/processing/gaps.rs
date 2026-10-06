//! GPS Gaps: temporal absence of Raw observations between known observations.
//!
//! Detection reads the complete Raw series before quality filtering, so a poor
//! record never invents an absence and unusable records never create a Gap. A
//! Gap precedes no first observation and follows no last one, ends Trip
//! continuity, contributes only its own duration and asserts no movement, Stop,
//! matcher call or straight Route connector.
use super::model::{Observation, Target};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// One interval without Raw GPS observations between two observed boundaries.
#[derive(Serialize)]
pub(super) struct GpsGap {
    pub id: String,
    pub kind: &'static str,
    pub activity_revision: Uuid,
    pub observed_from_at: DateTime<Utc>,
    pub observed_until_at: DateTime<Utc>,
    pub observed_duration_s: i64,
}

pub(super) fn detect(points: &[Observation], target: &Target, revision: Uuid) -> Vec<GpsGap> {
    points
        .windows(2)
        .filter(|pair| absent(pair, target))
        .enumerate()
        .map(|(index, pair)| {
            let from = pair[0].recorded_at;
            let until = pair[1].recorded_at;
            GpsGap {
                id: format!("{revision}:gap:{index}"),
                kind: "gap",
                activity_revision: revision,
                observed_from_at: from,
                observed_until_at: until,
                observed_duration_s: (until - from).num_seconds(),
            }
        })
        .collect()
}

/// The one predicate that defines a GPS Gap: an interval without Raw
/// observations between two known observed boundaries.
///
/// Compare at millisecond precision, like `movement`. GPS Records carry
/// millisecond timestamps and two accepted records may share a second, so a
/// sub-second spacing is an immediate re-observation rather than an absence.
/// `holes` asks this same predicate so a Gap and an Evidence Hole can never be
/// derived from two disagreeing notions of absence.
pub(super) fn absent(pair: &[Observation], target: &Target) -> bool {
    (pair[1].recorded_at - pair[0].recorded_at).num_milliseconds()
        > target.observation_gap_s * 1_000
}
