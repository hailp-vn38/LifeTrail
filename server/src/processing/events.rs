//! Chronological daily projection of Trips, Stops and GPS Gaps.
use super::{gaps::GpsGap, model::Day, stops::Stop, trips::Trip};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

struct Projected {
    visible_from: DateTime<Utc>,
    visible_until: DateTime<Utc>,
    id: String,
    value: Value,
}

/// Trip, Stop and GPS Gap items in observed chronological order for one
/// Owner-local day.
///
/// Actual, observed and visible intervals stay distinct, and continuation flags
/// only report observed coverage outside the day.
pub(super) fn timeline(stops: &[Stop], trips: &[Trip], gaps: &[GpsGap], day: &Day) -> Vec<Value> {
    let mut projected: Vec<Projected> = stops
        .iter()
        .map(|stop| project(stop, stop.observed_from_at, stop.observed_until_at, day))
        .chain(
            trips
                .iter()
                .map(|trip| project(trip, trip.observed_from_at, trip.observed_until_at, day)),
        )
        .chain(
            gaps.iter()
                .map(|gap| project(gap, gap.observed_from_at, gap.observed_until_at, day)),
        )
        .flatten()
        .collect();
    projected.sort_by(|left, right| {
        (&left.visible_from, &left.visible_until, &left.id).cmp(&(
            &right.visible_from,
            &right.visible_until,
            &right.id,
        ))
    });
    projected.into_iter().map(|item| item.value).collect()
}

fn project<T: serde::Serialize>(
    activity: &T,
    observed_from: DateTime<Utc>,
    observed_until: DateTime<Utc>,
    day: &Day,
) -> Option<Projected> {
    let visible_from = observed_from.max(day.from);
    let visible_until = observed_until.min(day.until);
    if visible_from >= visible_until {
        return None;
    }
    let mut value = serde_json::to_value(activity).expect("activity serializes");
    let id = value["id"].as_str().expect("activity has an id").to_owned();
    value["visible_from_at"] = json!(visible_from);
    value["visible_until_at"] = json!(visible_until);
    value["daily_observed_duration_s"] = json!((visible_until - visible_from).num_seconds());
    value["continues_before"] = json!(observed_from < day.from);
    value["continues_after"] = json!(observed_until > day.until);
    Some(Projected {
        visible_from,
        visible_until,
        id,
        value,
    })
}
