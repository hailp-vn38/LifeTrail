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
        if !point.usable || covered(stops, index) {
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
    let elapsed = (points[index].recorded_at - previous).num_seconds();
    elapsed > 0 && elapsed <= target.observation_gap_s
}

fn covered(stops: &[Stop], index: usize) -> bool {
    stops
        .iter()
        .any(|stop| stop.start_index <= index && index < stop.end_index)
}
