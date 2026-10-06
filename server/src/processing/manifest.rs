//! The explicit, ordered authority for Activity Revision coverage.
//!
//! A manifest entry is a half-open slice.  Selection is therefore structural,
//! never based on a revision UUID or creation time.
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Slice {
    pub revision: Uuid,
    pub from: DateTime<Utc>,
    pub until: DateTime<Utc>,
    pub from_boundary: &'static str,
    pub until_boundary: &'static str,
}

/// Replace `[from, until)` while retaining every non-overlapping old source.
/// Callers must establish semantic safety of the replacement endpoints before
/// calling this function; technical matcher/chunk seams are not boundaries.
pub(super) fn splice(existing: &[Slice], replacement: Slice) -> Vec<Slice> {
    assert!(replacement.from < replacement.until);
    let mut output = Vec::with_capacity(existing.len() + 1);
    for entry in existing {
        assert!(entry.from < entry.until);
        if entry.from < replacement.from {
            let until = entry.until.min(replacement.from);
            if entry.from < until {
                output.push(Slice {
                    revision: entry.revision,
                    from: entry.from,
                    until,
                    from_boundary: entry.from_boundary,
                    until_boundary: "superseded_range",
                });
            }
        }
        if entry.until > replacement.until {
            let from = entry.from.max(replacement.until);
            if from < entry.until {
                output.push(Slice {
                    revision: entry.revision,
                    from,
                    until: entry.until,
                    from_boundary: "superseded_range",
                    until_boundary: entry.until_boundary,
                });
            }
        }
    }
    output.push(replacement);
    output.sort_by_key(|slice| slice.from);
    assert!(output.windows(2).all(|pair| pair[0].until <= pair[1].from));
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone as _;

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 5, hour, 0, 0).unwrap()
    }
    fn slice(revision: Uuid, from: u32, until: u32) -> Slice {
        Slice {
            revision,
            from: at(from),
            until: at(until),
            from_boundary: "confirmed_stop",
            until_boundary: "confirmed_stop",
        }
    }

    #[test]
    fn splicing_an_interior_safe_range_retains_old_sources_on_both_sides() {
        let original = Uuid::new_v4();
        let replacement = Uuid::new_v4();
        let slices = splice(&[slice(original, 0, 23)], slice(replacement, 8, 16));
        assert_eq!(
            slices
                .iter()
                .map(|s| (s.revision, s.from, s.until))
                .collect::<Vec<_>>(),
            vec![
                (original, at(0), at(8)),
                (replacement, at(8), at(16)),
                (original, at(16), at(23))
            ]
        );
    }

    #[test]
    fn overlapping_replacements_remain_ordered_and_non_overlapping() {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let third = Uuid::new_v4();
        let once = splice(&[slice(first, 0, 23)], slice(second, 8, 16));
        let twice = splice(&once, slice(third, 12, 20));
        assert_eq!(
            twice
                .iter()
                .map(|s| (s.revision, s.from, s.until))
                .collect::<Vec<_>>(),
            vec![
                (first, at(0), at(8)),
                (second, at(8), at(12)),
                (third, at(12), at(20)),
                (first, at(20), at(23)),
            ]
        );
    }
}
