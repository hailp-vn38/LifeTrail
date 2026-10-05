use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use uuid::Uuid;

use super::persistence::RoutePoint;

const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

#[derive(Serialize)]
pub(crate) struct DailyView {
    device_id: Uuid,
    date: String,
    timezone: String,
    processing_state: &'static str,
    summary: DailySummary,
    route: Option<GeoJsonFeature>,
    start: Option<GeoJsonFeature>,
    end: Option<GeoJsonFeature>,
}

#[derive(Serialize)]
struct DailySummary {
    point_count: usize,
    distance_m: f64,
    duration_s: i64,
    first_fix_at: Option<DateTime<Utc>>,
    last_fix_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
struct GeoJsonFeature {
    #[serde(rename = "type")]
    feature_type: &'static str,
    properties: GeoJsonProperties,
    geometry: GeoJsonGeometry,
}

#[derive(Serialize)]
struct GeoJsonProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    recorded_at: Option<DateTime<Utc>>,
    /// RFC 3339 UTC timestamps, one per LineString coordinate, index-aligned.
    /// `None` for Point features.
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamps: Option<Vec<DateTime<Utc>>>,
}

#[derive(Serialize)]
struct GeoJsonGeometry {
    #[serde(rename = "type")]
    geometry_type: &'static str,
    coordinates: Coordinates,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Coordinates {
    Point([f64; 2]),
    LineString(Vec<[f64; 2]>),
}

impl DailyView {
    pub(super) fn from_points(
        device_id: Uuid,
        date: NaiveDate,
        timezone: &str,
        points: Vec<RoutePoint>,
    ) -> Self {
        let first_fix_at = points.first().map(|point| point.recorded_at);
        let last_fix_at = points.last().map(|point| point.recorded_at);
        let duration_s = match (first_fix_at, last_fix_at) {
            (Some(first), Some(last)) => (last - first).num_seconds(),
            _ => 0,
        };
        let distance_m = route_distance_m(&points);
        let route = (points.len() >= 2).then(|| line_string_feature(&points));
        let start = points.first().map(point_feature);
        let end = points.last().map(point_feature);

        Self {
            device_id,
            date: date.format("%Y-%m-%d").to_string(),
            timezone: timezone.to_owned(),
            processing_state: "raw",
            summary: DailySummary {
                point_count: points.len(),
                distance_m,
                duration_s,
                first_fix_at,
                last_fix_at,
            },
            route,
            start,
            end,
        }
    }
}

fn line_string_feature(points: &[RoutePoint]) -> GeoJsonFeature {
    GeoJsonFeature {
        feature_type: "Feature",
        properties: GeoJsonProperties {
            recorded_at: None,
            // Built from the exact same ordered slice as `coordinates` below,
            // so timestamps[i] always describes coordinates[i].
            timestamps: Some(points.iter().map(|point| point.recorded_at).collect()),
        },
        geometry: GeoJsonGeometry {
            geometry_type: "LineString",
            coordinates: Coordinates::LineString(points.iter().map(coordinates).collect()),
        },
    }
}

fn point_feature(point: &RoutePoint) -> GeoJsonFeature {
    GeoJsonFeature {
        feature_type: "Feature",
        properties: GeoJsonProperties {
            recorded_at: Some(point.recorded_at),
            timestamps: None,
        },
        geometry: GeoJsonGeometry {
            geometry_type: "Point",
            coordinates: Coordinates::Point(coordinates(point)),
        },
    }
}

fn coordinates(point: &RoutePoint) -> [f64; 2] {
    [point.lon, point.lat]
}

fn route_distance_m(points: &[RoutePoint]) -> f64 {
    points
        .windows(2)
        .map(|pair| haversine_m(&pair[0], &pair[1]))
        .sum()
}

fn haversine_m(first: &RoutePoint, second: &RoutePoint) -> f64 {
    let latitude_delta = (second.lat - first.lat).to_radians();
    let longitude_delta = (second.lon - first.lon).to_radians();
    let first_latitude = first.lat.to_radians();
    let second_latitude = second.lat.to_radians();
    let half_chord = (latitude_delta / 2.0).sin().powi(2)
        + first_latitude.cos() * second_latitude.cos() * (longitude_delta / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_METERS * half_chord.sqrt().atan2((1.0 - half_chord).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone as _;
    use serde_json::Value;

    fn route_point(recorded_at: DateTime<Utc>, lat: f64, lon: f64) -> RoutePoint {
        RoutePoint {
            recorded_at,
            lat,
            lon,
        }
    }

    fn sample_points() -> Vec<RoutePoint> {
        vec![
            route_point(
                Utc.with_ymd_and_hms(2026, 10, 5, 10, 0, 0).unwrap(),
                10.7760,
                106.7000,
            ),
            route_point(
                Utc.with_ymd_and_hms(2026, 10, 5, 10, 0, 3).unwrap(),
                10.7764,
                106.7005,
            ),
            route_point(
                Utc.with_ymd_and_hms(2026, 10, 5, 10, 0, 25).unwrap(),
                10.7770,
                106.7010,
            ),
        ]
    }

    fn daily_view(points: Vec<RoutePoint>) -> Value {
        let device_id = Uuid::nil();
        let date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let view = DailyView::from_points(device_id, date, "Asia/Ho_Chi_Minh", points);
        serde_json::to_value(&view).expect("DailyView serializes")
    }

    fn parse_timestamps(value: &Value) -> Vec<DateTime<Utc>> {
        value
            .as_array()
            .expect("timestamps is an array")
            .iter()
            .map(|entry| {
                entry
                    .as_str()
                    .expect("timestamp is a string")
                    .parse::<DateTime<Utc>>()
                    .expect("timestamp is RFC 3339")
            })
            .collect()
    }

    #[test]
    fn route_serializes_one_timestamp_per_coordinate_in_order() {
        let points = sample_points();
        let view = daily_view(points.clone());

        let coordinates = view["route"]["geometry"]["coordinates"]
            .as_array()
            .expect("route coordinates is an array");
        let timestamps = parse_timestamps(&view["route"]["properties"]["timestamps"]);

        assert_eq!(
            timestamps.len(),
            coordinates.len(),
            "timestamps.length === coordinates.length"
        );
        for (index, point) in points.iter().enumerate() {
            assert_eq!(
                timestamps[index], point.recorded_at,
                "timestamps[{index}] matches the RoutePoint used for coordinates[{index}]"
            );
            assert_eq!(coordinates[index][0].as_f64().unwrap(), point.lon);
            assert_eq!(coordinates[index][1].as_f64().unwrap(), point.lat);
        }
    }

    #[test]
    fn start_and_end_recorded_at_match_first_and_last_route_points() {
        let points = sample_points();
        let view = daily_view(points.clone());

        let start_at = view["start"]["properties"]["recorded_at"]
            .as_str()
            .expect("start recorded_at is a string")
            .parse::<DateTime<Utc>>()
            .unwrap();
        let end_at = view["end"]["properties"]["recorded_at"]
            .as_str()
            .expect("end recorded_at is a string")
            .parse::<DateTime<Utc>>()
            .unwrap();
        let timestamps = parse_timestamps(&view["route"]["properties"]["timestamps"]);

        assert_eq!(start_at, points.first().unwrap().recorded_at);
        assert_eq!(end_at, points.last().unwrap().recorded_at);
        assert_eq!(start_at, timestamps.first().copied().unwrap());
        assert_eq!(end_at, timestamps.last().copied().unwrap());
        assert!(view["start"]["properties"].get("timestamps").is_none());
        assert!(view["route"]["properties"].get("recorded_at").is_none());
    }

    #[test]
    fn empty_day_has_no_route_or_markers() {
        let view = daily_view(Vec::new());

        assert!(view["route"].is_null());
        assert!(view["start"].is_null());
        assert!(view["end"].is_null());
        assert_eq!(view["summary"]["point_count"], 0);
    }

    #[test]
    fn single_point_day_has_markers_but_no_route() {
        let points = vec![sample_points().remove(0)];
        let view = daily_view(points.clone());

        assert!(view["route"].is_null());
        assert!(!view["start"].is_null());
        assert!(!view["end"].is_null());
        let start_at = view["start"]["properties"]["recorded_at"]
            .as_str()
            .unwrap()
            .parse::<DateTime<Utc>>()
            .unwrap();
        assert_eq!(start_at, points[0].recorded_at);
    }
}
