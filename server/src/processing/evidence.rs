//! Unresolved intervals remain explicit until every interval is resolved.
use super::{
    model::{Observation, Target},
    stops::Stop,
    trips::Trip,
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
    trips: &[Trip],
    target: &Target,
) -> Vec<UnresolvedInterval> {
    let mut unresolved_intervals: Vec<UnresolvedInterval> = Vec::new();
    for pair in points.windows(2) {
        // An actual absence of Raw observations and unusable Raw records are
        // disclosed even when derived activity covers both sides, because a
        // derived Trip must not imply continuity across them.
        let absent =
            (pair[1].recorded_at - pair[0].recorded_at).num_seconds() > target.observation_gap_s;
        let unusable = !pair[0].usable || !pair[1].usable;
        if !absent && !unusable && resolved(&pair[0], &pair[1], stops, trips) {
            continue;
        }
        let reason = if absent {
            "missing_observations"
        } else if unusable {
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

/// An interval is resolved when both of its observations belong to a derived
/// activity. A Stop and an adjacent Trip each resolve their own side, so the
/// transition between them is activity rather than missing evidence.
fn resolved(from: &Observation, until: &Observation, stops: &[Stop], trips: &[Trip]) -> bool {
    within(from, stops, trips) && within(until, stops, trips)
}

fn within(point: &Observation, stops: &[Stop], trips: &[Trip]) -> bool {
    stops.iter().any(|stop| {
        point.recorded_at >= stop.observed_from_at && point.recorded_at <= stop.observed_until_at
    }) || trips.iter().any(|trip| {
        point.recorded_at >= trip.observed_from_at && point.recorded_at <= trip.observed_until_at
    })
}
