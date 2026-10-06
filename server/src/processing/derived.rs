//! Compose every derived activity for one captured observation range.
use super::{
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
pub(super) const REDUCER_VERSION: i64 = 4;

pub(super) struct Derived {
    pub stops: Vec<Stop>,
    pub trips: Vec<Trip>,
    pub gaps: Vec<GpsGap>,
    pub parts: Vec<RoutePart>,
    pub evidence_holes: Vec<EvidenceHole>,
}

pub(super) async fn derive(points: &[Observation], target: &Target, revision: Uuid) -> Derived {
    let stops = stops::detect(points, target, revision);
    let activity = trips::derive(points, &stops, target, revision);
    // Gaps come from the Raw series before quality filtering, holes only from
    // observations that exist, so the two never describe the same interval.
    let gaps = gaps::detect(points, target, revision);
    let evidence_holes = holes::holes(points, &stops, &activity.trips, target);
    let mut derived = Derived {
        stops,
        trips: activity.trips,
        gaps,
        parts: activity.parts,
        evidence_holes,
    };
    super::matcher::apply(&mut derived.parts, points, target).await;
    // The timeline owns segment/trip aggregates, while Route Parts own the
    // geometry. Reconcile only the published Part facts after matching so both
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
            if parts.iter().all(|part| part.source == "osrm_match") && !parts.is_empty() {
                segment.source = "osrm_match";
            }
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
