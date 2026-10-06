//! Ticket 05: a genuine absence is a GPS Gap, existing unreliable records are an
//! Evidence Hole, and an impossible position is neither.
//!
//! Every assertion here reads the public Daily View after a real `gps/1`
//! upload, real PostGIS persistence and the real processing worker. Raw GPS is
//! compared before and after processing, never through a private classifier.
mod support;

use serde_json::{Value, json};
use support::{
    assert_route_part,
    scenarios::{self, METER_PER_DEGREE, STEP, TOLERANCE_M},
};

/// A genuine absence of Raw observations is a GPS Gap, never a Stop, movement
/// or a straight Route connector, and contributes only its own duration.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn genuine_absence_publishes_a_gap_event_and_ends_trip_continuity() {
    let case = scenarios::case("genuine timestamp absence");
    let published = scenarios::publish(&scenarios::records(&case)).await;
    let view = &published.view;

    assert_eq!(view["processing_state"], "processed");
    assert_eq!(scenarios::kinds(view), ["trip", "gap", "trip"]);

    let gap = &view["timeline"][1];
    assert_eq!(gap["observed_from_at"], "2026-10-05T08:02:00Z");
    assert_eq!(gap["observed_until_at"], "2026-10-05T08:13:00Z");
    assert_eq!(gap["observed_duration_s"], 660);
    assert_eq!(gap["daily_observed_duration_s"], 660);
    assert_eq!(gap["continues_before"], json!(false));
    assert_eq!(gap["continues_after"], json!(false));
    // A Gap asserts neither movement nor a stationary location.
    assert!(gap.get("center").is_none());
    assert!(gap.get("movement_segments").is_none());
    assert_eq!(view["evidence_holes"], json!([]));
    // An explicit GPS Gap alone does not downgrade otherwise reliable activity.
    assert_eq!(view["evidence_state"], "sufficient");
    assert!(view.get("unresolved_intervals").is_none());

    // Gap duration is contributed, and no Stop is inferred inside it.
    assert_eq!(view["summary"]["gap_count"], 1);
    assert_eq!(view["summary"]["gap_duration_s"], 660);
    assert_eq!(view["summary"]["stop_count"], 0);
    assert_eq!(view["summary"]["trip_count"], 2);
    assert_eq!(view["summary"]["trip_duration_s"], 120 + 120);
    assert_eq!(view["summary"]["duration_s"], 120 + 660 + 120);

    // Two Trips with disconnected geometry; no part bridges the absence.
    let parts = view["route_parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2);
    for part in parts {
        assert_route_part(part, "gap-adjacent part", TOLERANCE_M);
    }
    assert_eq!(parts[0]["observed_until_at"], "2026-10-05T08:02:00Z");
    assert_eq!(parts[1]["observed_from_at"], "2026-10-05T08:13:00Z");
    assert_ne!(parts[0]["trip_id"], parts[1]["trip_id"]);
    // Distance is the published legs only; the Gap adds no connector.
    let leg = STEP * METER_PER_DEGREE;
    let summed = scenarios::published_distance_m(view);
    assert!((summed - 4.0 * leg).abs() < TOLERANCE_M, "{summed}");
    assert!(
        (view["summary"]["distance_m"].as_f64().unwrap() - summed).abs() < 1e-9,
        "daily distance sums published part lengths only"
    );
}

/// An impossible jump stays in Raw GPS but contributes no derived geometry,
/// distance or activity, and leaves the activity on either side open.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn impossible_jump_leaves_raw_intact_but_publishes_no_excursion() {
    let case = scenarios::case("one impossible jump");
    let published = scenarios::publish(&scenarios::records(&case)).await;
    let view = &published.view;

    // Every Raw record is counted, and the rejected one is counted separately.
    assert_eq!(view["summary"]["point_count"], 7);
    assert_eq!(view["summary"]["usable_point_count"], 6);
    assert_eq!(view["summary"]["excluded_point_count"], 1);
    assert_eq!(view["summary"]["low_quality_point_count"], 0);

    // The jump is disclosed as Evidence Hole coverage, never as a GPS Gap.
    let holes = view["evidence_holes"].as_array().unwrap();
    assert_eq!(holes.len(), 1, "{holes:?}");
    assert_eq!(holes[0]["reason"], "insufficient_geometry");
    assert_eq!(holes[0]["observed_from_at"], "2026-10-05T08:02:00Z");
    assert_eq!(holes[0]["observed_until_at"], "2026-10-05T08:04:00Z");
    assert_eq!(view["summary"]["gap_count"], 0, "a jump is not a Gap");
    assert_eq!(view["summary"]["gap_duration_s"], 0);
    assert_eq!(view["evidence_state"], "partial");

    // No published geometry reaches the impossible position.
    let parts = view["route_parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2, "movement either side stays separate Trips");
    for part in parts {
        for coordinate in part["geometry"]["coordinates"].as_array().unwrap() {
            assert_ne!(*coordinate, json!([106.7, 10.82]), "{coordinate}");
        }
    }
    let leg = STEP * METER_PER_DEGREE;
    let summed = scenarios::published_distance_m(view);
    assert!(
        (summed - 4.0 * leg).abs() < TOLERANCE_M,
        "the jump contributes no distance: {summed}"
    );
    assert!((view["summary"]["distance_m"].as_f64().unwrap() - summed).abs() < 1e-9);

    // Activity beside the hole keeps open actual boundaries rather than
    // asserting a start or end the observations cannot prove.
    let trips = scenarios::trips(view);
    assert_eq!(trips.len(), 2);
    assert_eq!(trips[0]["end_boundary"], "open");
    assert!(trips[0]["actual_end_at"].is_null());
    assert!(trips[0]["full_duration_s"].is_null());
    assert_eq!(trips[1]["start_boundary"], "open");
    assert!(trips[1]["actual_start_at"].is_null());
    assert!(trips[1]["full_duration_s"].is_null());
}

/// Poor observations at the *same* timestamps as a real absence behave
/// differently: existing records become Evidence Hole coverage, never a Gap.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn low_quality_observations_at_the_same_timestamps_are_evidence_holes() {
    let absent = scenarios::case("genuine timestamp absence");
    let poor = scenarios::case("continuous low-quality observations");
    // Both scenarios describe 08:00..08:15; only one of them is missing data.
    // The default observation gap is 300 seconds.
    let observed_every = |records: &[Value]| {
        records
            .iter()
            .map(|record| record["ts_ms"].as_i64().expect("ts_ms"))
            .collect::<Vec<i64>>()
    };
    let poor_times = observed_every(&scenarios::records(&poor));
    let absent_times = observed_every(&scenarios::records(&absent));
    assert!(
        poor_times
            .windows(2)
            .all(|pair| pair[1] - pair[0] <= 300_000),
        "the poor scenario keeps every minute observed: {poor_times:?}"
    );
    assert!(
        absent_times
            .windows(2)
            .any(|pair| pair[1] - pair[0] > 300_000),
        "the absent scenario really misses observations: {absent_times:?}"
    );
    let published = scenarios::publish(&scenarios::records(&poor)).await;
    let view = &published.view;

    assert_eq!(view["processing_state"], "processed");
    assert_eq!(view["summary"]["point_count"], 11);
    assert_eq!(view["summary"]["usable_point_count"], 6);
    assert_eq!(view["summary"]["low_quality_point_count"], 5);
    assert_eq!(view["summary"]["excluded_point_count"], 0);
    // Every record exists, so no interval without observations is a GPS Gap.
    assert_eq!(view["summary"]["gap_count"], 0);
    assert_eq!(view["summary"]["gap_duration_s"], 0);
    assert_eq!(scenarios::kinds(view), ["trip", "trip"]);

    // The unreliable run is disclosed as Evidence Hole coverage with a reason.
    let holes = view["evidence_holes"].as_array().unwrap();
    assert_eq!(holes.len(), 1, "{holes:?}");
    assert_eq!(holes[0]["reason"], "insufficient_quality");
    assert_eq!(holes[0]["observed_from_at"], "2026-10-05T08:02:00Z");
    assert_eq!(holes[0]["observed_until_at"], "2026-10-05T08:13:00Z");
    assert_eq!(holes[0]["source_record_count"], 7);
    // Reliable activity plus unresolved coverage is partial evidence.
    assert_eq!(view["evidence_state"], "partial");

    // Neither side of the hole is claimed to be one Trip, and no Stop is
    // inferred across it.
    let trips = scenarios::trips(view);
    assert_eq!(trips.len(), 2, "the hole splits movement into two Trips");
    assert_eq!(view["summary"]["stop_count"], 0);
    assert_ne!(trips[0]["id"], trips[1]["id"]);
    let parts = view["route_parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2);
    // No geometry bridges the hole, and the Trip beside it stays open.
    assert_eq!(parts[0]["observed_until_at"], "2026-10-05T08:02:00Z");
    assert_eq!(parts[1]["observed_from_at"], "2026-10-05T08:13:00Z");
    assert_eq!(trips[0]["end_boundary"], "open");
    assert!(trips[0]["actual_end_at"].is_null());
    assert_eq!(trips[1]["start_boundary"], "open");
    assert!(trips[1]["actual_start_at"].is_null());
}

/// Every deterministic scenario publishes exactly the disclosed evidence: Gap
/// events, Evidence Holes with reasons, Trips and truthful class counts.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn quality_gap_fixtures_publish_truthful_evidence() {
    for case in scenarios::cases() {
        let name = case["name"].as_str().unwrap();
        let records = scenarios::records(&case);
        let view = scenarios::publish(&records).await.view;

        assert_eq!(view["processing_state"], "processed", "{name}");
        assert_eq!(
            json!(scenarios::kinds(&view)),
            case["kinds"],
            "{name}: chronological kinds"
        );
        assert_eq!(view["evidence_state"], case["evidence_state"], "{name}");

        assert_eq!(
            json!(scenarios::published_gaps(&view)),
            case["gaps"],
            "{name}: GPS Gaps are Timeline events"
        );
        assert_eq!(
            view["summary"]["gap_count"].as_u64().unwrap() as usize,
            case["gaps"].as_array().unwrap().len(),
            "{name}: gap_count"
        );

        assert_eq!(
            json!(scenarios::published_holes(&view)),
            case["holes"],
            "{name}: Evidence Holes"
        );

        let trips = scenarios::trips(&view);
        assert_eq!(
            json!(scenarios::published_trip_bounds(&view)),
            case["trip_bounds"],
            "{name}: Trips"
        );
        assert_eq!(
            view["summary"]["trip_count"].as_u64().unwrap() as usize,
            trips.len(),
            "{name}: trip_count"
        );

        // Rejected geometry and connectors never inflate published distance.
        let legs = case["trip_legs"].as_f64().expect("trip_legs");
        assert_eq!(
            view["route_parts"].as_array().unwrap().len(),
            trips.len(),
            "{name}"
        );
        let summed = scenarios::published_distance_m(&view);
        assert!(
            (summed - legs * STEP * METER_PER_DEGREE).abs() < TOLERANCE_M,
            "{name}: published distance {summed} carries no rejected geometry"
        );

        // Raw counts are exhaustive and name all three classes separately, so a
        // poor-quality record is never reported as an impossible one.
        assert_eq!(view["summary"]["point_count"], records.len(), "{name}");
        assert_eq!(
            view["summary"]["usable_point_count"], case["usable_point_count"],
            "{name}: usable"
        );
        assert_eq!(
            view["summary"]["low_quality_point_count"], case["low_quality_point_count"],
            "{name}: low quality"
        );
        assert_eq!(
            view["summary"]["excluded_point_count"], case["excluded_point_count"],
            "{name}: excluded"
        );
        let classes: u64 = [
            "usable_point_count",
            "low_quality_point_count",
            "excluded_point_count",
        ]
        .iter()
        .map(|field| view["summary"][field].as_u64().unwrap())
        .sum();
        assert_eq!(
            classes,
            records.len() as u64,
            "{name}: the three classes partition the Raw total"
        );
    }
}
