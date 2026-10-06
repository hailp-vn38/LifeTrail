//! Evidence continuity across bounded rejected observations.
//!
//! Raw quality classes remain unchanged. Only short runs bracketed by usable
//! observations with plausible displacement may be omitted from derivation.
//! Long failures and observation edges retain unresolved evidence coverage.
use super::{
    gaps, geo,
    model::{Observation, Target},
};

pub(super) struct Supported {
    pub points: Vec<Observation>,
    pub bridged: Vec<bool>,
}

pub(super) fn supported(points: &[Observation], target: &Target) -> Supported {
    let mut bridged = vec![false; points.len()];
    let mut start = 0;
    while start < points.len() {
        if points[start].classification.is_usable() {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end < points.len() && !points[end].classification.is_usable() {
            end += 1;
        }
        if start > 0 && end < points.len() && can_bridge(&points[start - 1..=end], target) {
            bridged[start..end].fill(true);
        }
        start = end;
    }
    let supported = points
        .iter()
        .zip(&bridged)
        .filter(|(_, bridged)| !**bridged)
        .map(|(point, _)| point.clone())
        .collect();
    Supported {
        points: supported,
        bridged,
    }
}

fn can_bridge(points: &[Observation], target: &Target) -> bool {
    let first = &points[0];
    let last = &points[points.len() - 1];
    let elapsed_ms = (last.recorded_at - first.recorded_at).num_milliseconds();
    elapsed_ms > 0
        && elapsed_ms <= target.policy.short_failure_max_s * 1000
        && !points.windows(2).any(|pair| gaps::absent(pair, target))
        && geo::distance_m([first.lon, first.lat], [last.lon, last.lat])
            <= target.policy.max_implied_speed_mps * elapsed_ms as f64 / 1000.0
}

/// Activity metadata counts original records, including withheld observations.
/// Geometry and usable counts continue to include only usable observations.
pub(super) fn records_between(
    points: &[Observation],
    from: chrono::DateTime<chrono::Utc>,
    until: chrono::DateTime<chrono::Utc>,
) -> &[Observation] {
    let start = points.partition_point(|point| point.recorded_at < from);
    let end = points.partition_point(|point| point.recorded_at <= until);
    &points[start..end]
}
