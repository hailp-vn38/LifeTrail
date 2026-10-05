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
        properties: GeoJsonProperties { recorded_at: None },
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
