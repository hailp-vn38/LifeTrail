//! Raw-derived Route Parts: geometry plus server-owned historical progress.
use super::{geo, model::Observation, movement::MovementRun};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

/// One historical instant mapped to a progress along the Route Part.
#[derive(Serialize)]
pub(super) struct ProgressAnchor {
    pub at: DateTime<Utc>,
    pub distance_m: f64,
}

#[derive(Serialize)]
pub(super) struct PartGeometry {
    #[serde(rename = "type")]
    pub geometry_type: &'static str,
    pub coordinates: Vec<[f64; 2]>,
}

/// A contiguous drawable portion of the derived Route for one Movement Segment.
///
/// `vertex_distance_m` is aligned one-to-one with `geometry.coordinates`, starts
/// at zero and is non-decreasing, so consumers never recompute the length.
#[derive(Serialize)]
pub(super) struct RoutePart {
    pub id: String,
    pub kind: &'static str,
    pub trip_id: String,
    pub movement_segment_id: String,
    pub source: &'static str,
    pub mode: &'static str,
    pub classification_confidence: f64,
    pub observed_from_at: DateTime<Utc>,
    pub observed_until_at: DateTime<Utc>,
    pub distance_m: f64,
    pub vertex_distance_m: Vec<f64>,
    pub progress_anchors: Vec<ProgressAnchor>,
    pub quality: &'static str,
    pub source_record_count: usize,
    pub geometry: PartGeometry,
    /// Immutable request/result/failure evidence.  It is deliberately carried
    /// into the Activity Revision rather than requiring OSRM on historical read.
    pub matcher_evidence: Option<Value>,
    /// Observation times of each coordinate, used for daily clipping only.
    #[serde(skip)]
    pub(super) vertex_times: Vec<DateTime<Utc>>,
}

/// Build one Route Part from the observations of a movement run.
///
/// Every coordinate and anchor comes from an accepted GPS Record; nothing is
/// duplicated or interpolated here.
pub(super) fn build(
    id: String,
    trip_id: String,
    movement_segment_id: String,
    points: &[Observation],
    run: &MovementRun,
    mode: &'static str,
    classification_confidence: f64,
) -> RoutePart {
    let members = &points[run.start..run.end];
    let coordinates: Vec<[f64; 2]> = members.iter().map(|point| [point.lon, point.lat]).collect();
    let mut vertex_distance_m = Vec::with_capacity(coordinates.len());
    let mut progress = 0.0;
    for (index, coordinate) in coordinates.iter().enumerate() {
        if index > 0 {
            progress += geo::distance_m(coordinates[index - 1], *coordinate);
        }
        vertex_distance_m.push(progress);
    }
    let observed_from_at = members[0].recorded_at;
    let observed_until_at = members[members.len() - 1].recorded_at;
    RoutePart {
        id,
        kind: "route_part",
        trip_id,
        movement_segment_id,
        source: "raw",
        mode,
        classification_confidence,
        observed_from_at,
        observed_until_at,
        distance_m: progress,
        progress_anchors: members
            .iter()
            .zip(&vertex_distance_m)
            .map(|(point, distance)| ProgressAnchor {
                at: point.recorded_at,
                distance_m: *distance,
            })
            .collect(),
        vertex_distance_m,
        quality: "sufficient",
        source_record_count: members.len(),
        geometry: PartGeometry {
            geometry_type: "LineString",
            coordinates,
        },
        matcher_evidence: None,
        vertex_times: members.iter().map(|point| point.recorded_at).collect(),
    }
}
