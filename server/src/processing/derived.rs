//! Compose every derived activity for one captured observation range.
use super::{
    continuity,
    gaps::{self, GpsGap},
    holes::{self, EvidenceHole},
    model::{Observation, Target},
    route_parts::RoutePart,
    stops::{self, Stop},
    trips::{self, Trip},
};
use uuid::Uuid;

/// Version of the activity reducer that produced a revision and its Daily
/// Snapshots. Bumped whenever derivation semantics change, and recorded with
/// every Activity Revision and snapshot as provenance.
pub(super) const REDUCER_VERSION: i64 = 6;

pub(super) struct Derived {
    pub stops: Vec<Stop>,
    pub trips: Vec<Trip>,
    pub gaps: Vec<GpsGap>,
    pub parts: Vec<RoutePart>,
    pub evidence_holes: Vec<EvidenceHole>,
}

pub(super) fn derive(points: &[Observation], target: &Target, revision: Uuid) -> Derived {
    let supported = continuity::supported(points, target);
    let mut stops = stops::detect(&supported.points, target, revision);
    let mut activity = trips::derive(&supported.points, &stops, target, revision);
    for stop in &mut stops {
        let raw =
            continuity::records_between(points, stop.observed_from_at, stop.observed_until_at);
        stop.source_record_count = raw.len();
        stop.source_record_ids = raw.iter().map(|point| point.id).collect();
    }
    for trip in &mut activity.trips {
        let raw =
            continuity::records_between(points, trip.observed_from_at, trip.observed_until_at);
        trip.source_record_count = raw.len();
        trip.source_record_ids = raw.iter().map(|point| point.id).collect();
        for segment in &mut trip.movement_segments {
            segment.source_record_count = continuity::records_between(
                points,
                segment.observed_from_at,
                segment.observed_until_at,
            )
            .len();
        }
    }
    // Gaps come from the Raw series before quality filtering, holes only from
    // observations that exist, so the two never describe the same interval.
    let gaps = gaps::detect(points, target, revision);
    let evidence_holes = holes::holes(points, &supported.bridged, &stops, &activity.trips, target);
    let mut derived = Derived {
        stops,
        trips: activity.trips,
        gaps,
        parts: activity.parts,
        evidence_holes,
    };
    // The timeline owns segment/trip aggregates, while Route Parts own the
    // geometry. Reconcile only the published Part facts after processing so both
    // contracts report the same server-owned distance and source.
    for trip in &mut derived.trips {
        for segment in &mut trip.movement_segments {
            let parts: Vec<_> = derived
                .parts
                .iter()
                .filter(|part| part.movement_segment_id == segment.id)
                .collect();
            segment.route_part_ids = parts.iter().map(|part| part.id.clone()).collect();
            segment.distance_m = parts.iter().map(|part| part.distance_m).sum();
        }
        trip.distance_m = derived
            .parts
            .iter()
            .filter(|part| part.trip_id == trip.id)
            .map(|part| part.distance_m)
            .sum();
    }
    derived
}
