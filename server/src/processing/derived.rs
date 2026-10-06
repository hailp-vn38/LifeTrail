//! Compose every derived activity for one captured observation range.
use super::{
    evidence::{self, UnresolvedInterval},
    model::{Observation, Target},
    route_parts::RoutePart,
    stops::{self, Stop},
    trips::{self, Trip},
};
use uuid::Uuid;

pub(super) struct Derived {
    pub stops: Vec<Stop>,
    pub trips: Vec<Trip>,
    pub parts: Vec<RoutePart>,
    pub unresolved_intervals: Vec<UnresolvedInterval>,
}

pub(super) fn derive(points: &[Observation], target: &Target, revision: Uuid) -> Derived {
    let stops = stops::detect(points, target, revision);
    let activity = trips::derive(points, &stops, target, revision);
    let unresolved_intervals = evidence::unresolved(points, &stops, &activity.trips, target);
    Derived {
        stops,
        trips: activity.trips,
        parts: activity.parts,
        unresolved_intervals,
    }
}
