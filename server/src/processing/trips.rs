//! Trips: maximal chains of observed movement between supported boundaries.
//!
//! A Trip remains continuous across mode transitions; classifier output only
//! partitions its ordered Movement Segments.
use super::{
    classification,
    model::{Observation, Target},
    movement,
    route_parts::{self, RoutePart},
    stops::Stop,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub(super) struct MovementSegment {
    pub id: String,
    pub mode: &'static str,
    pub classification_confidence: f64,
    pub source: &'static str,
    pub observed_from_at: DateTime<Utc>,
    pub observed_until_at: DateTime<Utc>,
    pub observed_duration_s: i64,
    pub distance_m: f64,
    pub quality: &'static str,
    pub route_part_ids: Vec<String>,
    pub source_record_count: usize,
}

#[derive(Serialize)]
pub(super) struct Trip {
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
    pub distance_m: f64,
    pub quality: &'static str,
    pub movement_segment_count: usize,
    pub movement_segments: Vec<MovementSegment>,
    pub source_record_count: usize,
    pub usable_record_count: usize,
    pub source_record_ids: Vec<i64>,
}

pub(super) struct Activity {
    pub trips: Vec<Trip>,
    pub parts: Vec<RoutePart>,
}

pub(super) fn derive(
    points: &[Observation],
    stops: &[Stop],
    target: &Target,
    revision: Uuid,
) -> Activity {
    let mut trips = Vec::new();
    let mut parts = Vec::new();
    for run in movement::runs(points, stops, target) {
        if !movement::drawable(points, &run) {
            continue;
        }
        let trip_id = format!("{revision}:trip:{}", trips.len());
        let members = &points[run.start..run.end];
        let observed_from_at = members[0].recorded_at;
        let observed_until_at = members[members.len() - 1].recorded_at;
        // Trip time includes short pauses, so it is the observed span of the run.
        let observed_duration_s = (observed_until_at - observed_from_at).num_seconds();
        let start_confirmed = confirms(stops, |stop| {
            stop.end_index == run.start && stop.end_confirmed
        });
        let end_confirmed = confirms(stops, |stop| {
            stop.start_index == run.end && stop.start_confirmed
        });
        let full_duration_s = (start_confirmed && end_confirmed).then_some(observed_duration_s);
        let mut movement_segments = Vec::new();
        let mut trip_parts = Vec::new();
        for (sequence, classified) in classification::segments(points, &run, target)
            .into_iter()
            .enumerate()
        {
            let segment_id = format!("{trip_id}:segment:{sequence}");
            let segment_run = movement::MovementRun {
                start: classified.start,
                end: classified.end,
            };
            let part = movement::drawable(points, &segment_run).then(|| {
                route_parts::build(
                    format!("{segment_id}:part:0"),
                    trip_id.clone(),
                    segment_id.clone(),
                    points,
                    &segment_run,
                    classified.mode.name(),
                    classified.confidence,
                )
            });
            let segment_members = &points[classified.start..classified.end];
            let route_part_ids = part.iter().map(|part| part.id.clone()).collect();
            let distance_m = part.as_ref().map_or(0.0, |part| part.distance_m);
            movement_segments.push(MovementSegment {
                id: segment_id,
                mode: classified.mode.name(),
                classification_confidence: classified.confidence,
                source: "raw",
                observed_from_at: segment_members[0].recorded_at,
                observed_until_at: segment_members[segment_members.len() - 1].recorded_at,
                observed_duration_s: (segment_members[segment_members.len() - 1].recorded_at
                    - segment_members[0].recorded_at)
                    .num_seconds(),
                distance_m,
                quality: "sufficient",
                route_part_ids,
                source_record_count: segment_members.len(),
            });
            if let Some(part) = part {
                trip_parts.push(part);
            }
        }
        let distance_m: f64 = trip_parts.iter().map(|part| part.distance_m).sum();
        trips.push(Trip {
            id: trip_id,
            kind: "trip",
            activity_revision: revision,
            observed_from_at,
            observed_until_at,
            observed_duration_s,
            actual_start_at: start_confirmed.then_some(observed_from_at),
            actual_end_at: end_confirmed.then_some(observed_until_at),
            start_boundary: if start_confirmed { "confirmed" } else { "open" },
            end_boundary: if end_confirmed { "confirmed" } else { "open" },
            full_duration_s,
            distance_m,
            quality: "sufficient",
            movement_segment_count: movement_segments.len(),
            movement_segments,
            source_record_count: members.len(),
            usable_record_count: members.len(),
            source_record_ids: members.iter().map(|point| point.id).collect(),
        });
        parts.extend(trip_parts);
    }
    Activity { trips, parts }
}

/// A Trip boundary is confirmed only when an adjoining Stop confirms it. Dataset
/// edges, actual observation absences and Evidence Holes leave it open.
fn confirms(stops: &[Stop], predicate: impl Fn(&Stop) -> bool) -> bool {
    stops.iter().any(predicate)
}
