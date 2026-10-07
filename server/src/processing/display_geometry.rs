//! Display-only geometry projection for the Daily Snapshot.
//!
//! These coordinates are for visualization only. They never feed distance,
//! progress, playback, or export: canonical geometry stays in Activity
//! Revisions and is served by the Playback endpoint. The simplification is
//! deterministic so the same manifest always produces the same display bytes.
use std::env;

/// Mean Earth radius, matching `geo::distance_m` so the planar plane is close.
const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

const DEFAULT_TOLERANCE_M: f64 = 10.0;
const DEFAULT_MAX_VERTICES: usize = 3_000;
/// Ceiling for deterministic escalation so a pathological day still publishes.
const MAX_TOLERANCE_M: f64 = 1_000.0;
const ESCALATION: f64 = 1.5;

/// Shared per-day display budget, read once from server config.
#[derive(Clone, Copy, Debug)]
pub(super) struct DisplayBudget {
    pub tolerance_m: f64,
    pub max_vertices: usize,
}

impl DisplayBudget {
    pub(super) fn from_env() -> Self {
        Self {
            tolerance_m: env_f64("LT_DAILY_DISPLAY_SIMPLIFY_TOLERANCE_M", DEFAULT_TOLERANCE_M),
            max_vertices: env_usize("LT_DAILY_DISPLAY_MAX_VERTICES", DEFAULT_MAX_VERTICES),
        }
    }
}

/// One day's display result: simplified coordinates per Part plus the single
/// tolerance and budget verdict actually applied across the day.
pub(super) struct DayDisplay {
    pub parts: Vec<Vec<[f64; 2]>>,
    pub tolerance_m: f64,
    pub over_budget: bool,
}

/// Simplify every Part of one day under a shared escalating vertex budget.
///
/// Tolerance is shared across the day (so adjacent Parts stay visually
/// consistent) and escalates deterministically by `ESCALATION` until every Part
/// fits `max_vertices`. If the ceiling is reached while a Part is still over
/// budget, `over_budget` is set and the caller publishes anyway.
pub(super) fn simplify_parts(parts: &[Vec<[f64; 2]>], budget: DisplayBudget) -> DayDisplay {
    let mut tolerance = budget.tolerance_m.max(0.0);
    loop {
        let simplified: Vec<Vec<[f64; 2]>> = parts
            .iter()
            .map(|coordinates| simplify_part(coordinates, tolerance))
            .collect();
        let over_budget = simplified
            .iter()
            .any(|coordinates| coordinates.len() > budget.max_vertices);
        if !over_budget || tolerance >= MAX_TOLERANCE_M {
            return DayDisplay {
                parts: simplified,
                tolerance_m: tolerance,
                over_budget,
            };
        }
        tolerance = (tolerance * ESCALATION)
            .min(MAX_TOLERANCE_M)
            .max(tolerance + 1.0);
    }
}

/// Ramer–Douglas–Peucker on a local equirectangular metric plane.
///
/// Endpoints are always preserved, so clipping boundaries survive
/// simplification and Parts never drift away from their shared edges.
fn simplify_part(coordinates: &[[f64; 2]], tolerance_m: f64) -> Vec<[f64; 2]> {
    let rounded = || {
        dedupe(
            coordinates
                .iter()
                .map(|[lon, lat]| [round6(*lon), round6(*lat)])
                .collect(),
        )
    };
    if coordinates.len() <= 2 {
        return rounded();
    }
    let mean_lat = coordinates.iter().map(|c| c[1]).sum::<f64>() / coordinates.len() as f64;
    let cos_lat = mean_lat.to_radians().cos();
    let planar: Vec<(f64, f64)> = coordinates
        .iter()
        .map(|[lon, lat]| {
            (
                lon.to_radians() * cos_lat * EARTH_RADIUS_METERS,
                lat.to_radians() * EARTH_RADIUS_METERS,
            )
        })
        .collect();
    let keep = rdp_keep(&planar, tolerance_m);
    dedupe(
        coordinates
            .iter()
            .zip(&keep)
            .filter(|(_, keep)| **keep)
            .map(|([lon, lat], _)| [round6(*lon), round6(*lat)])
            .collect(),
    )
}

fn rdp_keep(points: &[(f64, f64)], tolerance: f64) -> Vec<bool> {
    let n = points.len();
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let mut stack = vec![(0usize, n - 1)];
    while let Some((start, end)) = stack.pop() {
        let mut farthest = 0.0;
        let mut index = start;
        for i in (start + 1)..end {
            let distance = segment_distance(points[i], points[start], points[end]);
            if distance > farthest {
                farthest = distance;
                index = i;
            }
        }
        if farthest > tolerance {
            keep[index] = true;
            stack.push((start, index));
            stack.push((index, end));
        }
    }
    keep
}

fn segment_distance(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> f64 {
    let (px, py) = point;
    let (sx, sy) = start;
    let (ex, ey) = end;
    let dx = ex - sx;
    let dy = ey - sy;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f64::EPSILON {
        return ((px - sx).powi(2) + (py - sy).powi(2)).sqrt();
    }
    let t = (((px - sx) * dx + (py - sy) * dy) / length_sq).clamp(0.0, 1.0);
    let proj_x = sx + t * dx;
    let proj_y = sy + t * dy;
    ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt()
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

/// Drop consecutive duplicates introduced by rounding, keeping the endpoints.
fn dedupe(coordinates: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(coordinates.len());
    for coordinate in coordinates {
        if out.last() != Some(&coordinate) {
            out.push(coordinate);
        }
    }
    out
}

fn env_f64(name: &str, default: f64) -> f64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value >= 2)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> Vec<[f64; 2]> {
        (0..=100)
            .map(|i| [106.0 + i as f64 * 0.00001, 10.0])
            .collect()
    }

    #[test]
    fn collinear_line_collapses_to_endpoints() {
        let simplified = simplify_part(&line(), 5.0);
        assert_eq!(simplified.len(), 2);
        assert_eq!(simplified[0], [106.0, 10.0]);
        assert_eq!(simplified[1], [106.001, 10.0]);
    }

    #[test]
    fn endpoints_are_preserved() {
        let simplified = simplify_part(&line(), 0.0);
        assert_eq!(simplified.first().unwrap(), &[106.0, 10.0]);
        assert_eq!(simplified.last().unwrap(), &[106.001, 10.0]);
    }

    #[test]
    fn rounding_is_six_decimals() {
        let simplified = simplify_part(&[[1.123456789, 2.987654321], [1.2, 3.0]], 0.0);
        assert_eq!(simplified[0], [1.123457, 2.987654]);
    }

    #[test]
    fn budget_escalates_until_the_day_fits() {
        let budget = DisplayBudget {
            tolerance_m: 0.0,
            max_vertices: 4,
        };
        // A zig-zag keeps every vertex at zero tolerance, forcing escalation.
        let zigzag: Vec<[f64; 2]> = (0..=100)
            .map(|i| [106.0 + i as f64 * 0.00001, 10.0 + (i % 2) as f64 * 0.001])
            .collect();
        let displayed = simplify_parts(&[zigzag], budget);
        assert!(!displayed.over_budget);
        assert!(displayed.parts[0].len() <= 4);
        assert!(displayed.tolerance_m > budget.tolerance_m);
    }

    #[test]
    fn over_budget_is_reported_at_the_ceiling() {
        let budget = DisplayBudget {
            tolerance_m: 0.0,
            max_vertices: 2,
        };
        // Two zig-zag parts keep more than 2 vertices at any sane tolerance.
        let parts = vec![
            (0..50)
                .map(|i| [106.0 + i as f64 * 0.001, 10.0 + (i % 2) as f64 * 0.01])
                .collect::<Vec<_>>(),
            (0..50)
                .map(|i| [107.0 + i as f64 * 0.001, 11.0 + (i % 2) as f64 * 0.01])
                .collect::<Vec<_>>(),
        ];
        let displayed = simplify_parts(&parts, budget);
        assert!(displayed.over_budget);
    }
}
