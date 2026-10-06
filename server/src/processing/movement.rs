//! Continuous movement runs between Stops on the observed UTC history.
//!
//! A run ends at a Stop, at an unusable observation, or at an actual absence of
//! Raw observations. A short pause below the Stop criteria stays inside the run,
//! because the pause is observed movement history rather than a boundary.
use super::{
    model::{Observation, Target},
    stops::Stop,
};

/// Half-open observation index range `[start, end)` of one continuous movement run.
pub(super) struct MovementRun {
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn runs(points: &[Observation], stops: &[Stop], target: &Target) -> Vec<MovementRun> {
    let mut runs = Vec::new();
    let mut start: Option<usize> = None;
    for (index, point) in points.iter().enumerate() {
        if !point.usable() || covered(stops, index) {
            close(&mut runs, &mut start, index);
            continue;
        }
        match start {
            // A run opens on the first usable observation outside a Stop.
            None => start = Some(index),
            Some(open) if !continuous(points, index, target) => {
                // An actual absence of observations ends the run here.
                runs.push(MovementRun {
                    start: open,
                    end: index,
                });
                start = Some(index);
            }
            Some(_) => {}
        }
    }
    close(&mut runs, &mut start, points.len());
    runs
}

/// A run needs at least two observations that are not the same position, so a
/// drawn Route Part never repeats a coordinate to satisfy the LineString contract.
pub(super) fn drawable(points: &[Observation], run: &MovementRun) -> bool {
    points[run.start..run.end]
        .windows(2)
        .any(|pair| pair[0].lat != pair[1].lat || pair[0].lon != pair[1].lon)
}

fn close(runs: &mut Vec<MovementRun>, start: &mut Option<usize>, end: usize) {
    if let Some(open) = start.take() {
        runs.push(MovementRun { start: open, end });
    }
}

fn continuous(points: &[Observation], index: usize, target: &Target) -> bool {
    let previous = points[index - 1].recorded_at;
    // Compare at millisecond precision, matching `evidence` and `stops`. GPS Records
    // carry millisecond timestamps and `gps_points` has no uniqueness on
    // `(device_id, recorded_at)`, so two accepted records may share a second. A
    // sub-second spacing is an immediate re-observation, not an absence: truncating to
    // whole seconds would end the run and split one continuous chain into two Trips.
    // Only a gap longer than the threshold ends a run.
    let elapsed_ms = (points[index].recorded_at - previous).num_milliseconds();
    elapsed_ms <= target.observation_gap_s * 1_000
}

fn covered(stops: &[Stop], index: usize) -> bool {
    stops
        .iter()
        .any(|stop| stop.start_index <= index && index < stop.end_index)
}
