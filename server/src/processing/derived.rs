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
pub(super) const REDUCER_VERSION: i64 = 2;

pub(super) struct Derived {
    pub stops: Vec<Stop>,
    pub trips: Vec<Trip>,
    pub gaps: Vec<GpsGap>,
    pub parts: Vec<RoutePart>,
    pub evidence_holes: Vec<EvidenceHole>,
}

pub(super) fn derive(points: &[Observation], target: &Target, revision: Uuid) -> Derived {
    let stops = stops::detect(points, target, revision);
    let activity = trips::derive(points, &stops, target, revision);
    // Gaps come from the Raw series before quality filtering, holes only from
    // observations that exist, so the two never describe the same interval.
    let gaps = gaps::detect(points, target, revision);
    let evidence_holes = holes::holes(points, &stops, &activity.trips, target);
    Derived {
        stops,
        trips: activity.trips,
        gaps,
        parts: activity.parts,
        evidence_holes,
    }
}
