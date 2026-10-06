//! Safe range replacement: unchanged Raw gaps bound independent activity chains.
//! Quality/classification still sees the complete captured history; only derived
//! output inside the replacement range is persisted in the new revision.
use super::{derived::Derived, model::Input};
use chrono::{DateTime, Utc};
use serde_json::Value;

pub(super) fn restrict(
    derived: &mut Derived,
    input: &Input,
    previous: &[Value],
) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let first = input.observations.first()?.recorded_at;
    let last = input.observations.last()?.recorded_at + chrono::Duration::milliseconds(1);
    if first >= last {
        return None;
    }
    let mut from = first;
    let mut until = last;
    if let (Some(dirty_from), Some(dirty_until)) =
        (input.target.dirty_from_at, input.target.dirty_until_at)
        && !previous.is_empty()
        && previous.iter().all(|revision| {
            revision["target_generation"].as_i64() == Some(input.target.target_generation)
        })
    {
        for gap in &derived.gaps {
            // Both interpretations must agree on the entire absence. A filled
            // gap is never a safe cut, nor is a technical geometry seam.
            let unchanged = previous.iter().any(|revision| {
                revision["body"]["gaps"].as_array().is_some_and(|gaps| {
                    gaps.iter().any(|old| {
                        belongs(old, revision)
                            && old["observed_from_at"] == serde_json::json!(gap.observed_from_at)
                            && old["observed_until_at"] == serde_json::json!(gap.observed_until_at)
                    })
                })
            });
            let boundary = gap.observed_until_at;
            let crosses =
                |from: DateTime<Utc>, until: DateTime<Utc>| from < boundary && until > boundary;
            let safe_new = !derived
                .stops
                .iter()
                .any(|v| crosses(v.observed_from_at, v.observed_until_at))
                && !derived
                    .trips
                    .iter()
                    .any(|v| crosses(v.observed_from_at, v.observed_until_at))
                && !derived
                    .parts
                    .iter()
                    .any(|v| crosses(v.observed_from_at, v.observed_until_at))
                && !derived
                    .evidence_holes
                    .iter()
                    .any(|v| crosses(v.observed_from_at, v.observed_until_at));
            let safe_old = previous.iter().all(|revision| {
                ["stops", "trips", "route_parts", "evidence_holes"]
                    .iter()
                    .all(|key| {
                        revision["body"][key]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter(|v| belongs(v, revision))
                            .all(|v| {
                                match (
                                    instant(&v["observed_from_at"]),
                                    instant(&v["observed_until_at"]),
                                ) {
                                    (Some(from), Some(until)) => !crosses(from, until),
                                    _ => false,
                                }
                            })
                    })
            });
            if !unchanged || !safe_new || !safe_old {
                continue;
            }
            if boundary <= dirty_from {
                from = from.max(boundary);
            }
            if gap.observed_from_at >= dirty_until {
                until = until.min(boundary);
            }
        }
    }
    derived
        .stops
        .retain(|v| v.observed_from_at >= from && v.observed_from_at < until);
    derived
        .trips
        .retain(|v| v.observed_from_at >= from && v.observed_from_at < until);
    derived
        .gaps
        .retain(|v| v.observed_from_at >= from && v.observed_from_at < until);
    derived
        .parts
        .retain(|v| v.observed_from_at >= from && v.observed_from_at < until);
    derived
        .evidence_holes
        .retain(|v| v.observed_from_at >= from && v.observed_from_at < until);
    Some((from, until))
}

fn instant(value: &Value) -> Option<DateTime<Utc>> {
    value.as_str()?.parse().ok()
}
fn belongs(value: &Value, revision: &Value) -> bool {
    match (
        instant(&value["observed_from_at"]),
        instant(&revision["from_at"]),
        instant(&revision["until_at"]),
    ) {
        (Some(at), Some(from), Some(until)) => at >= from && at < until,
        _ => false,
    }
}
