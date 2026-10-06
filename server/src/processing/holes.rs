//! Evidence Holes: intervals that contain Raw observations which cannot support
//! reliable activity or geometry.
//!
//! A hole is always bounded by observations that exist, so it is distinct from
//! a GPS Gap. A hole never asserts one Trip or an inferred Stop across it, and
//! the activity beside it keeps open actual boundaries.
use super::{
    gaps,
    model::{Observation, Target},
    quality,
    stops::Stop,
    trips::Trip,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Projected interval of existing observations with unresolved coverage.
#[derive(Serialize)]
pub(super) struct EvidenceHole {
    pub observed_from_at: DateTime<Utc>,
    pub observed_until_at: DateTime<Utc>,
    pub reason: &'static str,
    pub source_record_count: usize,
}

pub(super) fn holes(
    points: &[Observation],
    bridged: &[bool],
    stops: &[Stop],
    trips: &[Trip],
    target: &Target,
) -> Vec<EvidenceHole> {
    let mut holes: Vec<EvidenceHole> = Vec::new();
    for (index, pair) in points.windows(2).enumerate() {
        // An actual absence of Raw observations is a GPS Gap, published as its
        // own Timeline event. It is never evidence coverage: a hole always
        // contains records that exist but cannot support activity.
        if gaps::absent(pair, target) {
            continue;
        }
        // Short failures are resolved only by the independently checked
        // neighboring evidence AND an activity covering both boundaries.
        if (bridged[index] || bridged[index + 1]) && resolved(&pair[0], &pair[1], stops, trips) {
            continue;
        }
        // All other unreliable records retain explicit unresolved coverage.
        // Reliable observations that no Stop or Trip explains are ambiguous
        // activity, never missing observations.
        let reason = if let Some(reason) = unreliable(pair) {
            reason
        } else if resolved(&pair[0], &pair[1], stops, trips) {
            continue;
        } else {
            "ambiguous_activity"
        };
        extend(&mut holes, pair, reason);
    }
    holes
}

/// Why this pair of existing observations cannot support reliable activity.
///
/// `quality` owns the class-to-reason decision, including the stronger-claim
/// rule, so coverage metadata never re-derives the classification vocabulary.
fn unreliable(pair: &[Observation]) -> Option<&'static str> {
    quality::hole_reason(pair.iter().map(|point| point.classification))
}

fn extend(holes: &mut Vec<EvidenceHole>, pair: &[Observation], reason: &'static str) {
    let (from, until) = (pair[0].recorded_at, pair[1].recorded_at);
    if let Some(last) = holes.last_mut()
        && last.reason == reason
        && last.observed_until_at == from
    {
        last.observed_until_at = until;
        last.source_record_count += 1;
    } else {
        holes.push(EvidenceHole {
            observed_from_at: from,
            observed_until_at: until,
            reason,
            source_record_count: 2,
        });
    }
}

/// An interval is resolved when both of its observations belong to a derived
/// activity. A Stop and an adjacent Trip each resolve their own side, so the
/// transition between them is activity rather than a hole.
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
