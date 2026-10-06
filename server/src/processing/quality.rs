//! Deterministic quality classification of captured Raw observations.
//!
//! Classification reads immutable Raw acquisition metadata plus the centrally
//! configurable policy captured with the processing target. It never updates,
//! deletes or rewrites Raw GPS: an `Excluded` observation is withheld from
//! derived geometry only, so the Owner can still inspect what was recorded.
use super::{geo, model::Observation};
use chrono::{DateTime, Utc};

/// An impossible position cannot support geometry.
const INSUFFICIENT_GEOMETRY: &str = "insufficient_geometry";
/// Unreliable acquisition metadata cannot support activity.
const INSUFFICIENT_QUALITY: &str = "insufficient_quality";

/// How well one Raw observation supports derived activity and geometry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum QualityClass {
    /// Reliable enough to support a Stop, Trip or Route Part.
    Usable,
    /// A Raw observation that exists but cannot support reliable activity.
    LowQuality,
    /// An impossible observation, withheld from derived geometry.
    Excluded,
}

impl QualityClass {
    pub(super) fn is_usable(self) -> bool {
        matches!(self, QualityClass::Usable)
    }

    /// Why one classification cannot support reliable activity, if it cannot.
    ///
    /// This is the single owner of the class-to-reason vocabulary, so coverage
    /// metadata never re-derives the distinction itself.
    fn hole_reason(self) -> Option<&'static str> {
        match self {
            QualityClass::Usable => None,
            QualityClass::LowQuality => Some(INSUFFICIENT_QUALITY),
            QualityClass::Excluded => Some(INSUFFICIENT_GEOMETRY),
        }
    }
}

impl Default for QualityClass {
    /// A classification that was never computed must never contribute geometry.
    fn default() -> Self {
        QualityClass::Excluded
    }
}

/// Centrally configurable quality and implied-speed policy.
///
/// The processing `Target` owns this policy through `#[sqlx(flatten)]`, so the
/// configured thresholds and the criteria that read them cannot drift apart.
#[derive(sqlx::FromRow)]
pub(super) struct Policy {
    pub(super) max_hdop: f64,
    pub(super) max_implied_speed_mps: f64,
    pub(super) jump_distance_floor_m: f64,
}

/// Why an interval containing these classes cannot support reliable activity.
///
/// The stronger claim wins: an impossible position cannot support geometry even
/// when a neighbouring record is merely of poor quality.
pub(super) fn hole_reason(classes: impl IntoIterator<Item = QualityClass>) -> Option<&'static str> {
    let mut reason = None;
    for class in classes {
        if class == QualityClass::Excluded {
            return class.hole_reason();
        }
        reason = reason.or(class.hole_reason());
    }
    reason
}

/// Classify every captured observation, in Raw order.
///
/// Displacement is measured against the previous `Usable` observation rather
/// than the previous row: an observation that was already excluded or
/// low-quality contributes no trusted position, so it cannot push the next
/// reliable record into an impossible jump of its own.
pub(super) fn classify(points: &mut [Observation], policy: &Policy) {
    let mut trusted: Option<(DateTime<Utc>, [f64; 2])> = None;
    for point in points.iter_mut() {
        point.classification = classify_one(point, trusted, policy);
        if point.classification.is_usable() {
            trusted = Some((point.recorded_at, [point.lon, point.lat]));
        }
    }
}

fn classify_one(
    point: &Observation,
    trusted: Option<(DateTime<Utc>, [f64; 2])>,
    policy: &Policy,
) -> QualityClass {
    // Acquisition metadata that cannot support a position at all.
    if point.fix_quality <= 0
        || point.satellites <= 0
        || point.hdop.is_some_and(|hdop| hdop > policy.max_hdop)
    {
        return QualityClass::LowQuality;
    }
    // A receiver reporting travel faster than the policy allows is impossible,
    // however far its position moved.
    if point
        .speed_mps
        .is_some_and(|speed| speed > policy.max_implied_speed_mps)
    {
        return QualityClass::Excluded;
    }
    if trusted.is_some_and(|(at, position)| jumped(point, at, position, policy)) {
        return QualityClass::Excluded;
    }
    QualityClass::Usable
}

/// True when the displacement from the previous trusted position is larger than
/// the distance the elapsed time allows.
///
/// A short interval between two observations is an immediate re-observation
/// rather than a journey, so very short intervals keep an absolute floor: the
/// two records may share a second and carry genuinely different positions.
fn jumped(
    point: &Observation,
    previous_at: DateTime<Utc>,
    previous_position: [f64; 2],
    policy: &Policy,
) -> bool {
    let elapsed_s = (point.recorded_at - previous_at).num_milliseconds() as f64 / 1_000.0;
    let allowed_m = (policy.max_implied_speed_mps * elapsed_s).max(policy.jump_distance_floor_m);
    geo::distance_m(previous_position, [point.lon, point.lat]) > allowed_m
}
