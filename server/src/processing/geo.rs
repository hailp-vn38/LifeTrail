//! Great-circle distance shared by Stop radius and derived Route Part progress.
const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

/// Distance in meters between two `[longitude, latitude]` positions.
pub(super) fn distance_m(a: [f64; 2], b: [f64; 2]) -> f64 {
    let lat = (b[1] - a[1]).to_radians();
    let lon = (b[0] - a[0]).to_radians();
    let chord = ((lat / 2.0).sin().powi(2)
        + a[1].to_radians().cos() * b[1].to_radians().cos() * (lon / 2.0).sin().powi(2))
    .clamp(0.0, 1.0);
    2.0 * EARTH_RADIUS_METERS * chord.sqrt().asin()
}
