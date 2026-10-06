//! Clip persisted Route Parts using server-owned historical progress.
use super::model::Day;
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

fn bounds(value: &Value) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    Some((
        value["observed_from_at"].as_str()?.parse().ok()?,
        value["observed_until_at"].as_str()?.parse().ok()?,
    ))
}

pub(super) fn project_part(mut value: Value, day: &Day) -> Option<Value> {
    let (from, until) = bounds(&value)?;
    let visible_from = from.max(day.from);
    let visible_until = until.min(day.until);
    if visible_from >= visible_until {
        return None;
    }
    let anchors = value["progress_anchors"].as_array()?;
    let coordinates = value["geometry"]["coordinates"].as_array()?;
    // Route Parts persist historical anchors, so projection can clip their
    // geometry without matching again or using an estimated travel duration.
    if anchors.len() != coordinates.len() || anchors.len() < 2 {
        return None;
    }
    let at = |instant| anchor_position(anchors, coordinates, instant);
    let (start_coordinate, start_distance) = at(visible_from)?;
    let (end_coordinate, end_distance) = at(visible_until)?;
    let mut visible = vec![(visible_from, start_coordinate, start_distance)];
    for (anchor, coordinate) in anchors.iter().zip(coordinates) {
        let instant = anchor["at"].as_str()?.parse::<DateTime<Utc>>().ok()?;
        let distance = anchor["distance_m"].as_f64()?;
        if instant > visible_from && instant < visible_until {
            visible.push((instant, coordinate.clone(), distance));
        }
    }
    visible.push((visible_until, end_coordinate, end_distance));
    value["visible_from_at"] = json!(visible_from);
    value["visible_until_at"] = json!(visible_until);
    value["continues_before"] = json!(from < day.from);
    value["continues_after"] = json!(until > day.until);
    value["visible_distance_m"] = json!(end_distance - start_distance);
    value["geometry"] = json!({"type":"LineString","coordinates":visible.iter().map(|(_, coordinate, _)| coordinate).collect::<Vec<_>>()});
    value["vertex_distance_m"] = json!(
        visible
            .iter()
            .map(|(_, _, distance)| distance - start_distance)
            .collect::<Vec<_>>()
    );
    value["progress_anchors"] = json!(
        visible
            .iter()
            .map(|(at, _, distance)| json!({"at":at,"distance_m":distance-start_distance}))
            .collect::<Vec<_>>()
    );
    Some(value)
}

fn anchor_position(
    anchors: &[Value],
    coordinates: &[Value],
    instant: DateTime<Utc>,
) -> Option<(Value, f64)> {
    let indexed = anchors.iter().enumerate().find(|(_, anchor)| {
        anchor["at"]
            .as_str()
            .and_then(|at| at.parse::<DateTime<Utc>>().ok())
            .is_some_and(|at| at >= instant)
    })?;
    let next = indexed.0;
    if next == 0 {
        return Some((coordinates[0].clone(), anchors[0]["distance_m"].as_f64()?));
    }
    let previous = next - 1;
    let from = anchors[previous]["at"]
        .as_str()?
        .parse::<DateTime<Utc>>()
        .ok()?;
    let until = anchors[next]["at"]
        .as_str()?
        .parse::<DateTime<Utc>>()
        .ok()?;
    let ratio = (instant - from).num_milliseconds() as f64
        / (until - from).num_milliseconds().max(1) as f64;
    let from_distance = anchors[previous]["distance_m"].as_f64()?;
    let until_distance = anchors[next]["distance_m"].as_f64()?;
    let from_coordinate = coordinates[previous].as_array()?;
    let until_coordinate = coordinates[next].as_array()?;
    Some((
        json!([
            from_coordinate[0].as_f64()?
                + (until_coordinate[0].as_f64()? - from_coordinate[0].as_f64()?) * ratio,
            from_coordinate[1].as_f64()?
                + (until_coordinate[1].as_f64()? - from_coordinate[1].as_f64()?) * ratio
        ]),
        from_distance + (until_distance - from_distance) * ratio,
    ))
}
