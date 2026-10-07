//! Independent point-to-polyline error of the published processed GPS geometry.
use serde_json::Value;

fn xy(point: &Value) -> [f64; 2] {
    let lon = point[0].as_f64().unwrap();
    let lat = point[1].as_f64().unwrap();
    [
        lon.to_radians() * 6371000.0 * 10_f64.to_radians().cos(),
        lat.to_radians() * 6371000.0,
    ]
}

fn distance_to_segment(point: [f64; 2], first: [f64; 2], last: [f64; 2]) -> f64 {
    let delta = [last[0] - first[0], last[1] - first[1]];
    let denominator = delta[0] * delta[0] + delta[1] * delta[1];
    let fraction = if denominator == 0.0 {
        0.0
    } else {
        (((point[0] - first[0]) * delta[0] + (point[1] - first[1]) * delta[1]) / denominator)
            .clamp(0.0, 1.0)
    };
    (point[0] - first[0] - fraction * delta[0]).hypot(point[1] - first[1] - fraction * delta[1])
}

pub fn maximum_error(baseline: &Value, filtered: &Value) -> f64 {
    let before = baseline["route_parts"].as_array().unwrap();
    let after = filtered["route_parts"].as_array().unwrap();
    assert_eq!(before.len(), after.len(), "processed Route Part continuity");
    let mut maximum: f64 = 0.0;
    for (before_part, after_part) in before.iter().zip(after) {
        let segments = after_part["geometry"]["coordinates"].as_array().unwrap();
        for point in before_part["geometry"]["coordinates"].as_array().unwrap() {
            let nearest = segments
                .windows(2)
                .map(|pair| distance_to_segment(xy(point), xy(&pair[0]), xy(&pair[1])))
                .fold(f64::INFINITY, f64::min);
            maximum = maximum.max(nearest);
        }
    }
    maximum
}
