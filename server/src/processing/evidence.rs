//! Unresolved intervals remain explicit until movement/gap processing is available.
use super::{
    model::{Observation, Target},
    stops::Stop,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct UnresolvedInterval {
    pub observed_from_at: DateTime<Utc>,
    pub observed_until_at: DateTime<Utc>,
    pub reason: &'static str,
    pub source_record_count: usize,
}

pub(super) fn unresolved(
    points: &[Observation],
    stops: &[Stop],
    target: &Target,
) -> Vec<UnresolvedInterval> {
    let mut unresolved_intervals: Vec<UnresolvedInterval> = Vec::new();
    for pair in points.windows(2) {
        if stops.iter().any(|s| {
            pair[0].recorded_at >= s.observed_from_at && pair[1].recorded_at <= s.observed_until_at
        }) {
            continue;
        }
        let reason = if (pair[1].recorded_at - pair[0].recorded_at).num_seconds()
            > target.observation_gap_s
        {
            "missing_observations"
        } else if !pair[0].usable || !pair[1].usable {
            "unusable_observations"
        } else {
            "unresolved_activity"
        };
        if let Some(last) = unresolved_intervals.last_mut()
            && last.reason == reason
            && last.observed_until_at == pair[0].recorded_at
        {
            last.observed_until_at = pair[1].recorded_at;
            last.source_record_count += 1;
        } else {
            unresolved_intervals.push(UnresolvedInterval {
                observed_from_at: pair[0].recorded_at,
                observed_until_at: pair[1].recorded_at,
                reason,
                source_record_count: 2,
            });
        }
    }
    unresolved_intervals
}
