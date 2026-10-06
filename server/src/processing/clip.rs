//! Daily clipping of Route Parts on anchored progress.
//!
//! A visible part is the portion observed inside the Owner-local day, with a
//! synthetic anchor at a calendar boundary and progress rebased to zero, so the
//! visible distance is exactly the difference in original progress at the clip
//! endpoints and adjacent days conserve the part distance.
use super::route_parts::{ProgressAnchor, RoutePart};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct VisiblePart {
    pub id: String,
    pub kind: &'static str,
    pub trip_id: String,
    pub movement_segment_id: String,
    pub source: &'static str,
    pub observed_from_at: DateTime<Utc>,
    pub observed_until_at: DateTime<Utc>,
    pub distance_m: f64,
    pub visible_distance_m: f64,
    pub quality: &'static str,
    pub source_record_count: usize,
    pub visible_from_at: DateTime<Utc>,
    pub visible_until_at: DateTime<Utc>,
    pub continues_before: bool,
    pub continues_after: bool,
    pub geometry: VisibleGeometry,
    pub vertex_distance_m: Vec<f64>,
    pub progress_anchors: Vec<ProgressAnchor>,
}

#[derive(Serialize)]
pub(super) struct VisibleGeometry {
    #[serde(rename = "type")]
    pub geometry_type: &'static str,
    pub coordinates: Vec<[f64; 2]>,
}

pub(super) fn visible(
    part: &RoutePart,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
) -> Option<VisiblePart> {
    if part.observed_until_at <= from || until <= part.observed_from_at {
        return None;
    }
    let visible_from_at = part.observed_from_at.max(from);
    let visible_until_at = part.observed_until_at.min(until);
    let origin = progress_at(part, visible_from_at);
    let mut coordinates = Vec::new();
    let mut times = Vec::new();
    for (index, at) in part.vertex_times.iter().enumerate() {
        if *at < visible_from_at {
            continue;
        }
        if *at > visible_until_at {
            break;
        }
        if coordinates.is_empty() && visible_from_at < *at {
            let (coordinate, _) = at_progress(part, visible_from_at);
            coordinates.push(coordinate);
            times.push(visible_from_at);
        }
        coordinates.push(part.geometry.coordinates[index]);
        times.push(*at);
    }
    if visible_until_at > *times.last()? {
        let (coordinate, _) = at_progress(part, visible_until_at);
        coordinates.push(coordinate);
        times.push(visible_until_at);
    }
    let vertex_distance_m: Vec<f64> = times
        .iter()
        .map(|at| progress_at(part, *at) - origin)
        .collect();
    let visible_distance_m = progress_at(part, visible_until_at) - origin;
    Some(VisiblePart {
        id: part.id.clone(),
        kind: part.kind,
        trip_id: part.trip_id.clone(),
        movement_segment_id: part.movement_segment_id.clone(),
        source: part.source,
        observed_from_at: part.observed_from_at,
        observed_until_at: part.observed_until_at,
        distance_m: part.distance_m,
        visible_distance_m,
        quality: part.quality,
        source_record_count: part.source_record_count,
        visible_from_at,
        visible_until_at,
        continues_before: part.observed_from_at < from,
        continues_after: part.observed_until_at > until,
        geometry: VisibleGeometry {
            geometry_type: "LineString",
            coordinates,
        },
        progress_anchors: times
            .into_iter()
            .zip(&vertex_distance_m)
            .map(|(at, distance_m)| ProgressAnchor {
                at,
                distance_m: *distance_m,
            })
            .collect(),
        vertex_distance_m,
    })
}

pub(super) fn visible_parts(
    parts: &[RoutePart],
    from: DateTime<Utc>,
    until: DateTime<Utc>,
) -> Vec<VisiblePart> {
    parts
        .iter()
        .filter_map(|part| visible(part, from, until))
        .collect()
}

/// Progress at a historical instant, interpolated between the anchors that
/// bracket it. Every instant used here lies inside the part's observed coverage.
fn progress_at(part: &RoutePart, at: DateTime<Utc>) -> f64 {
    at_progress(part, at).1
}

/// Interpolate the coordinate and progress at an instant inside the Part.
///
/// The interpolation is used only for calendar-boundary clipping, where ADR 0004
/// requires a synthetic anchor. It never introduces an extra published anchor:
/// the resulting instant is clamped to the bracketing observed vertices.
fn at_progress(part: &RoutePart, at: DateTime<Utc>) -> ([f64; 2], f64) {
    let last = part.vertex_times.len() - 1;
    let index = (0..last)
        .find(|index| part.vertex_times[*index + 1] >= at)
        .unwrap_or(last - 1);
    let next = (index + 1).min(last);
    let span = (part.vertex_times[next] - part.vertex_times[index]).num_milliseconds() as f64;
    let ratio = if span > 0.0 {
        ((at - part.vertex_times[index]).num_milliseconds() as f64 / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let from = part.geometry.coordinates[index];
    let to = part.geometry.coordinates[next];
    (
        [
            from[0] + (to[0] - from[0]) * ratio,
            from[1] + (to[1] - from[1]) * ratio,
        ],
        part.vertex_distance_m[index]
            + (part.vertex_distance_m[next] - part.vertex_distance_m[index]) * ratio,
    )
}
