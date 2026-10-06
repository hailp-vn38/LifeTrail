//! Spatial dwell on continuous UTC observations, independent of calendar projection.
use super::{
    geo,
    model::{Observation, Target},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub(super) struct Stop {
    pub id: String,
    pub kind: &'static str,
    pub activity_revision: Uuid,
    pub observed_from_at: DateTime<Utc>,
    pub observed_until_at: DateTime<Utc>,
    pub observed_duration_s: i64,
    pub actual_start_at: Option<DateTime<Utc>>,
    pub actual_end_at: Option<DateTime<Utc>>,
    pub start_boundary: &'static str,
    pub end_boundary: &'static str,
    pub full_duration_s: Option<i64>,
    pub center: [f64; 2],
    pub radius_m: f64,
    pub quality: &'static str,
    pub source_record_count: usize,
    pub usable_record_count: usize,
    pub source_record_ids: Vec<i64>,
    /// Observation range this Stop covers; used to bound adjacent Trips.
    #[serde(skip)]
    pub(super) start_index: usize,
    #[serde(skip)]
    pub(super) end_index: usize,
    #[serde(skip)]
    pub(super) start_confirmed: bool,
    #[serde(skip)]
    pub(super) end_confirmed: bool,
}

pub(super) fn detect(points: &[Observation], target: &Target, revision: Uuid) -> Vec<Stop> {
    let mut stops = Vec::new();
    let mut start = 0;
    while start < points.len() {
        if !points[start].usable() {
            start += 1;
            continue;
        }
        // The first reliable observation anchors the dwell disk. Every member must
        // stay inside it; a chain of nearby points cannot drift into a false Stop.
        let center = [points[start].lon, points[start].lat];
        let mut end = start + 1;
        let mut radius_m: f64 = 0.0;
        while end < points.len() {
            let point = &points[end];
            let distance = geo::distance_m(center, [point.lon, point.lat]);
            if !point.usable()
                || (point.recorded_at - points[end - 1].recorded_at).num_seconds()
                    > target.observation_gap_s
                || distance > target.stop_radius_m
            {
                break;
            }
            radius_m = radius_m.max(distance);
            end += 1;
        }
        let first = points[start].recorded_at;
        let last = points[end - 1].recorded_at;
        let duration = (last - first).num_seconds();
        let confirmed_before =
            start > 0 && transition(&points[start - 1], &points[start], center, target);
        let confirmed_after =
            end < points.len() && transition(&points[end], &points[end - 1], center, target);
        if duration >= target.stop_min_duration_s {
            stops.push(Stop {
                id: format!("{revision}:stop:{}", stops.len()),
                kind: "stop",
                activity_revision: revision,
                observed_from_at: first,
                observed_until_at: last,
                observed_duration_s: duration,
                actual_start_at: confirmed_before.then_some(first),
                actual_end_at: confirmed_after.then_some(last),
                start_boundary: if confirmed_before {
                    "confirmed"
                } else {
                    "open"
                },
                end_boundary: if confirmed_after { "confirmed" } else { "open" },
                full_duration_s: (confirmed_before && confirmed_after).then_some(duration),
                center,
                radius_m,
                quality: "sufficient",
                source_record_count: end - start,
                usable_record_count: end - start,
                source_record_ids: points[start..end].iter().map(|p| p.id).collect(),
                start_index: start,
                end_index: end,
                start_confirmed: confirmed_before,
                end_confirmed: confirmed_after,
            });
            start = end;
        } else {
            // Retry at the next observation: a short approach must not hide a
            // qualifying dwell anchored slightly later in the same vicinity.
            start += 1;
        }
    }
    stops
}

fn transition(
    outside: &Observation,
    inside: &Observation,
    center: [f64; 2],
    target: &Target,
) -> bool {
    outside.usable()
        && (outside.recorded_at - inside.recorded_at)
            .num_seconds()
            .abs()
            <= target.observation_gap_s
        && outside.recorded_at != inside.recorded_at
        && geo::distance_m(center, [outside.lon, outside.lat]) > target.stop_radius_m
}
