//! Windowed transport-mode evidence and its persistence state machine.
//!
//! Classification is deliberately independent of matching.  A route match may
//! corroborate geometry, but it never selects the mode that selected a profile.
use super::{
    geo,
    model::{Observation, Target},
    movement::MovementRun,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    Unknown,
    Walk,
    Bike,
    Car,
}

impl Mode {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Walk => "walk",
            Self::Bike => "bike",
            Self::Car => "car",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Segment {
    pub start: usize,
    pub end: usize,
    pub mode: Mode,
    pub confidence: f64,
}

#[derive(Clone, Copy)]
struct Evidence {
    mode: Mode,
    confidence: f64,
}

/// Split a continuous movement run only after a candidate survives the target's
/// configured duration.  The returned ranges remain contiguous and retain a
/// sustained UNKNOWN as an explicit segment.
pub(super) fn segments(points: &[Observation], run: &MovementRun, target: &Target) -> Vec<Segment> {
    let evidence: Vec<_> = (run.start..run.end)
        .map(|index| evidence(points, run.start, index, target))
        .collect();
    let mut labels = vec![Mode::Unknown; evidence.len()];
    let mut current = Mode::Unknown;
    let mut candidate: Option<(Mode, usize)> = None;
    for (offset, item) in evidence.iter().enumerate() {
        let acceptable = item.mode != current && item.confidence >= target.mode_enter_confidence;
        // Current evidence below exit threshold starts an UNKNOWN candidate;
        // its grace prevents a one-window interruption from splitting a mode.
        let desired = if item.mode == current && item.confidence >= target.mode_exit_confidence {
            current
        } else if acceptable {
            item.mode
        } else {
            Mode::Unknown
        };
        if desired == current {
            candidate = None;
            labels[offset] = current;
            continue;
        }
        let (mode, began) = match candidate {
            Some((mode, began)) if mode == desired => (mode, began),
            _ => (desired, offset),
        };
        candidate = Some((mode, began));
        let age = (points[run.start + offset].recorded_at - points[run.start + began].recorded_at)
            .num_seconds();
        let required = if mode == Mode::Unknown {
            target.mode_unknown_grace_s
        } else {
            target.mode_change_min_duration_s
        };
        if age >= required {
            current = mode;
            for label in &mut labels[began..=offset] {
                *label = current;
            }
            candidate = None;
        } else {
            labels[offset] = current;
        }
    }
    let mut result = Vec::new();
    let mut start = 0;
    for end in 1..=labels.len() {
        if end == labels.len() || labels[end] != labels[start] {
            let confidence = evidence[start..end]
                .iter()
                .map(|e| e.confidence)
                .sum::<f64>()
                / (end - start) as f64;
            result.push(Segment {
                start: run.start + start,
                end: run.start + end,
                mode: labels[start],
                confidence,
            });
            start = end;
        }
    }
    result
}

fn evidence(points: &[Observation], first: usize, index: usize, target: &Target) -> Evidence {
    let until = points[index].recorded_at;
    let mut start = index;
    while start > first
        && (until - points[start - 1].recorded_at).num_seconds() <= target.mode_window_s
    {
        start -= 1;
    }
    let elapsed = (until - points[start].recorded_at).num_milliseconds() as f64 / 1_000.0;
    if elapsed <= 0.0 {
        return Evidence {
            mode: Mode::Unknown,
            confidence: 0.0,
        };
    }
    let distance = points[start..=index]
        .windows(2)
        .map(|pair| geo::distance_m([pair[0].lon, pair[0].lat], [pair[1].lon, pair[1].lat]))
        .sum::<f64>();
    classify_speed(distance / elapsed)
}

fn classify_speed(speed: f64) -> Evidence {
    // Broad overlapping ranges leave boundary speeds uncertain rather than
    // claiming precision we do not have from sparse consumer GPS.
    let (mode, center, spread) = if speed < 2.8 {
        (Mode::Walk, 1.35, 1.45)
    } else if speed < 8.5 {
        (Mode::Bike, 5.0, 3.2)
    } else {
        (Mode::Car, 15.0, 12.0)
    };
    let confidence = (1.0 - (speed - center).abs() / spread).clamp(0.0, 0.98);
    if confidence < 0.5 {
        Evidence {
            mode: Mode::Unknown,
            confidence,
        }
    } else {
        Evidence { mode, confidence }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::quality::{Policy, QualityClass};
    use chrono::{Duration, TimeZone, Utc};

    fn target() -> Target {
        Target {
            input_generation: 0,
            work_generation: 0,
            target_generation: 0,
            target_id: "test".into(),
            timezone: "UTC".into(),
            timezone_generation: 0,
            fencing_token: 0,
            active_manifest_id: None,
            stop_radius_m: 30.0,
            stop_min_duration_s: 180,
            observation_gap_s: 300,
            mode_window_s: 60,
            mode_change_min_duration_s: 120,
            mode_enter_confidence: 0.70,
            mode_exit_confidence: 0.55,
            mode_unknown_grace_s: 90,
            policy: Policy {
                max_hdop: 5.0,
                max_implied_speed_mps: 70.0,
                jump_distance_floor_m: 100.0,
            },
        }
    }

    fn points(steps: &[f64]) -> Vec<Observation> {
        let base = Utc.with_ymd_and_hms(2026, 10, 5, 8, 0, 0).unwrap();
        let mut lat = 10.0;
        steps
            .iter()
            .enumerate()
            .map(|(index, step)| {
                lat += step;
                Observation {
                    id: index as i64,
                    recorded_at: base + Duration::seconds(index as i64 * 60),
                    lat,
                    lon: 106.0,
                    fix_quality: 1,
                    hdop: Some(1.0),
                    satellites: 8,
                    speed_mps: None,
                    classification: QualityClass::Usable,
                }
            })
            .collect()
    }

    #[test]
    fn speed_bands_keep_supported_modes_distinct() {
        assert_eq!(classify_speed(1.3).mode, Mode::Walk);
        assert_eq!(classify_speed(5.0).mode, Mode::Bike);
        assert_eq!(classify_speed(15.0).mode, Mode::Car);
    }

    #[test]
    fn sustained_transfer_splits_a_trip_but_brief_unknown_does_not() {
        // ~1.3m/s walk, then a two-minute zero-speed interruption, then
        // sustained ~15m/s car. The interruption is shorter than UNKNOWN
        // grace; the transfer survives the enter-duration rule.
        let points = points(&[
            0.0, 0.0007, 0.0007, 0.0007, 0.0, 0.0, 0.0007, 0.0081, 0.0081, 0.0081, 0.0081,
        ]);
        let run = MovementRun {
            start: 0,
            end: points.len(),
        };
        let result = segments(&points, &run, &target());
        let modes: Vec<_> = result.iter().map(|segment| segment.mode).collect();
        assert!(modes.contains(&Mode::Walk));
        assert!(modes.contains(&Mode::Car));
        assert!(
            !modes
                .windows(2)
                .any(|pair| pair == [Mode::Walk, Mode::Unknown])
        );
    }
}
