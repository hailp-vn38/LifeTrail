mod support;

use axum::http::StatusCode;
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    db, processing,
};
use serde_json::{Value, json};
use support::{assert_route_part, ndjson, read, record, upload};
use uuid::Uuid;

/// Meters per degree of latitude on the sphere used by the server's geodesic
/// helper. Independent of the implementation under test.
const METER_PER_DEGREE: f64 = 111_194.926_644_558_74;
/// 0.0005 degrees of latitude: 55.6 m, beyond the default 30 m Stop radius.
const STEP: f64 = 0.0005;
const TOLERANCE_M: f64 = 0.5;

fn at(hour: u32, minute: u32) -> i64 {
    chrono::DateTime::parse_from_rfc3339(&format!("2026-10-05T{hour:02}:{minute:02}:00Z"))
        .unwrap()
        .timestamp_millis()
}

fn offsets(base: i64, minutes: &[(u32, f64)]) -> Vec<Value> {
    minutes
        .iter()
        .map(|(minute, offset)| record(base + *minute as i64 * 60_000, 10.7700 + offset, 106.7000))
        .collect()
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn movement_between_stops_publishes_unknown_trips_with_server_owned_progress() {
    let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("TRUNCATE users CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    let owner = db::create_owner(&pool, "Owner", "UTC").await.unwrap();
    let token = generate_device_token();
    let device = db::create_device(&pool, owner.id, "Device", &token)
        .await
        .unwrap();
    let router = app::router(AppState { db: pool.clone() }, None);

    let base = at(8, 0);
    // A qualifying Stop, a Trip containing a 120 second pause below the Stop
    // minimum, a second qualifying Stop, then a Trip ending at the observation edge.
    let records = offsets(
        base,
        &[
            (0, 0.0),
            (1, 0.0),
            (2, 0.0),
            (3, 0.0),
            (4, 0.0),
            (5, 0.0),
            (6, 0.0),
            (7, STEP),
            (8, 2.0 * STEP),
            (9, 3.0 * STEP),
            (10, 3.0 * STEP),
            (11, 3.0 * STEP),
            (12, 4.0 * STEP),
            (13, 5.0 * STEP),
            (16, 6.0 * STEP),
            (17, 6.0 * STEP),
            (18, 6.0 * STEP),
            (19, 6.0 * STEP),
            (20, 6.0 * STEP),
            (21, 6.0 * STEP),
            (22, 6.0 * STEP),
            (23, 7.0 * STEP),
            (24, 8.0 * STEP),
            (25, 9.0 * STEP),
        ],
    );
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &ndjson(&records))
            .await
            .0,
        StatusCode::OK
    );
    let raw: Vec<String> =
        sqlx::query_scalar("SELECT row_to_json(g)::text FROM gps_points g ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(processing::process_next(&pool).await.unwrap());

    let path = format!("/api/v1/devices/{}/days/2026-10-05", device.id);
    let view = read(&router, &path).await;
    assert_eq!(view["processing_state"], "processed");
    assert_eq!(view["evidence_state"], "sufficient");
    assert!(view["evidence_holes"].as_array().unwrap().is_empty());
    assert!(view["unresolved_intervals"].as_array().unwrap().is_empty());

    // Raw GPS stays untouched and separately selectable after publication.
    let raw_view = read(&router, &format!("{path}?view=raw")).await;
    assert_eq!(raw_view["processing_state"], "raw");
    assert_eq!(
        raw_view["route"]["geometry"]["coordinates"]
            .as_array()
            .unwrap()
            .len(),
        records.len()
    );
    let after: Vec<String> =
        sqlx::query_scalar("SELECT row_to_json(g)::text FROM gps_points g ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(raw, after, "Raw GPS is immutable");

    // Projected Trip/Stop items are chronological.
    let timeline = view["timeline"].as_array().unwrap();
    let kinds: Vec<&str> = timeline
        .iter()
        .map(|item| item["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["stop", "trip", "stop", "trip"]);

    let first_trip = &timeline[1];
    assert_eq!(first_trip["observed_from_at"], "2026-10-05T08:07:00Z");
    assert_eq!(first_trip["observed_until_at"], "2026-10-05T08:13:00Z");
    // The 120 second pause is inside the Trip: Trip time is 08:07..08:13.
    assert_eq!(first_trip["observed_duration_s"], 360);
    assert_eq!(first_trip["daily_observed_duration_s"], 360);
    assert_eq!(first_trip["movement_segment_count"], 1);
    let segment = &first_trip["movement_segments"][0];
    assert_eq!(segment["mode"], "unknown");
    assert_eq!(segment["source"], "raw");
    assert_eq!(segment["quality"], "sufficient");
    // Both boundaries confirmed by an adjoining Stop transition.
    assert_eq!(first_trip["start_boundary"], "confirmed");
    assert_eq!(first_trip["end_boundary"], "confirmed");
    assert_eq!(first_trip["actual_start_at"], "2026-10-05T08:07:00Z");
    assert_eq!(first_trip["actual_end_at"], "2026-10-05T08:13:00Z");
    assert_eq!(first_trip["full_duration_s"], 360);

    // The final Trip ends at the observation edge, so only its end is open.
    let last_trip = &timeline[3];
    assert_eq!(last_trip["start_boundary"], "confirmed");
    assert_eq!(last_trip["end_boundary"], "open");
    assert!(last_trip["actual_end_at"].is_null());
    assert!(last_trip["full_duration_s"].is_null());
    // The first Stop begins at the observation edge, so only its start is open.
    assert_eq!(timeline[0]["start_boundary"], "open");
    assert_eq!(timeline[0]["end_boundary"], "confirmed");

    // Each drawable raw Route Part identifies segment/source/observed coverage.
    let parts = view["route_parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0]["trip_id"], first_trip["id"]);
    assert_eq!(parts[0]["movement_segment_id"], segment["id"]);
    assert_eq!(parts[0]["source"], "raw");
    assert_eq!(parts[0]["quality"], "sufficient");
    assert_eq!(parts[0]["observed_from_at"], "2026-10-05T08:07:00Z");
    assert_eq!(parts[0]["observed_until_at"], "2026-10-05T08:13:00Z");
    assert_eq!(parts[0]["source_record_count"], 7);
    for (index, part) in parts.iter().enumerate() {
        assert_route_part(part, &format!("part {index}"), TOLERANCE_M);
    }
    // No coordinate is invented to satisfy the LineString contract.
    assert_eq!(
        parts[0]["geometry"]["coordinates"]
            .as_array()
            .unwrap()
            .len(),
        parts[0]["source_record_count"].as_u64().unwrap() as usize
    );
    // Four moving legs of one Step; the 120 second pause adds no distance.
    let walking = 4.0 * STEP * METER_PER_DEGREE;
    assert!(
        (parts[0]["distance_m"].as_f64().unwrap() - walking).abs() < TOLERANCE_M,
        "Trip distance is the sum of its published part length"
    );

    // Daily distance sums published part lengths only.
    let summed: f64 = parts
        .iter()
        .map(|part| part["distance_m"].as_f64().unwrap())
        .sum();
    assert!((view["summary"]["distance_m"].as_f64().unwrap() - summed).abs() < 1e-9);
    // Trip time includes the pause and is never labeled moving duration.
    assert_eq!(view["summary"]["trip_duration_s"], 360 + 120);
    assert_eq!(view["summary"]["stop_duration_s"], 2 * 360);
    assert_eq!(view["summary"]["gap_duration_s"], 0);
    assert_eq!(view["summary"]["duration_s"], 360 + 120 + 2 * 360);
    assert_eq!(view["summary"]["trip_count"], 2);
    assert_eq!(view["summary"]["stop_count"], 2);
    assert!(view["summary"].get("moving_duration_s").is_none());
    // Processed parts never enter the legacy Raw-only playback clock contract.
    assert!(view["route"].is_null());
    assert!(view["start"].is_null());
    assert!(view["end"].is_null());

    // Event and Route Part identity comes from the creating Activity Revision.
    let revision = timeline[0]["activity_revision"].as_str().unwrap();
    assert!(!revision.is_empty());
    assert_eq!(
        view["provenance"]["manifest_version"].is_string(),
        json!(true)
    );
    for item in timeline {
        assert_eq!(item["activity_revision"], json!(revision));
        assert!(
            item["id"].as_str().unwrap().starts_with(revision),
            "event id carries its creating activity revision"
        );
    }
    for part in parts {
        let trip = timeline
            .iter()
            .find(|item| item["id"] == part["trip_id"])
            .expect("a part belongs to a published Trip");
        assert!(
            part["id"].as_str().unwrap().starts_with(revision),
            "Route Part identity carries its creating activity revision"
        );
        assert_eq!(part["trip_id"], trip["id"]);
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn repeated_location_visits_publish_separate_trips_and_stops() {
    let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("TRUNCATE users CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    let owner = db::create_owner(&pool, "Owner", "UTC").await.unwrap();
    let token = generate_device_token();
    let device = db::create_device(&pool, owner.id, "Device", &token)
        .await
        .unwrap();
    let router = app::router(AppState { db: pool.clone() }, None);

    // Dwell at the same location twice, separated by two legs of sparse movement.
    let records = offsets(
        at(9, 0),
        &[
            (0, 0.0),
            (2, 0.0),
            (4, 0.0),
            (6, 0.0),
            (8, 2.0 * STEP),
            (10, 4.0 * STEP),
            (12, 2.0 * STEP),
            (14, 0.0),
            (16, 0.0),
            (18, 0.0),
            (20, 0.0),
            (22, 0.0),
            (24, 2.0 * STEP),
            (26, 4.0 * STEP),
        ],
    );
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &ndjson(&records))
            .await
            .0,
        StatusCode::OK
    );
    assert!(processing::process_next(&pool).await.unwrap());

    let view = read(
        &router,
        &format!("/api/v1/devices/{}/days/2026-10-05", device.id),
    )
    .await;
    let timeline = view["timeline"].as_array().unwrap();
    let kinds: Vec<&str> = timeline
        .iter()
        .map(|item| item["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["stop", "trip", "stop", "trip"]);
    // The two visits share a location but remain distinct Stops.
    let stops: Vec<&Value> = timeline
        .iter()
        .filter(|item| item["kind"] == "stop")
        .collect();
    assert_eq!(stops.len(), 2);
    assert_eq!(stops[0]["center"], stops[1]["center"]);
    assert_ne!(stops[0]["id"], stops[1]["id"]);
    assert_eq!(stops[0]["observed_from_at"], "2026-10-05T09:00:00Z");
    assert_eq!(stops[1]["observed_from_at"], "2026-10-05T09:14:00Z");
    // The middle Trip has both boundaries confirmed by the two Stops.
    assert_eq!(timeline[1]["start_boundary"], "confirmed");
    assert_eq!(timeline[1]["end_boundary"], "confirmed");
    assert_eq!(timeline[1]["full_duration_s"], 240);

    // Sparse two-minute geometry stays sparse: one coordinate per accepted record.
    let parts = view["route_parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2);
    for part in parts {
        assert_route_part(part, "sparse part", TOLERANCE_M);
        assert_eq!(
            part["geometry"]["coordinates"].as_array().unwrap().len(),
            part["source_record_count"].as_u64().unwrap() as usize
        );
    }
    assert_eq!(parts[0]["source_record_count"], 3);
    assert_eq!(parts[1]["source_record_count"], 2);
    // Every Part carries its own identity and identifies its Movement Segment.
    let ids: Vec<&str> = parts
        .iter()
        .map(|part| part["id"].as_str().unwrap())
        .collect();
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), ids.len());
    let segments: Vec<&str> = parts
        .iter()
        .map(|part| part["movement_segment_id"].as_str().unwrap())
        .collect();
    let mut unique = segments.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), segments.len(), "one segment per Trip");
    let summed: f64 = parts
        .iter()
        .map(|part| part["distance_m"].as_f64().unwrap())
        .sum();
    assert!((summed - view["summary"]["distance_m"].as_f64().unwrap()).abs() < 1e-9);
    assert!(
        (summed - 6.0 * STEP * METER_PER_DEGREE).abs() < TOLERANCE_M,
        "two sparse legs out and back, plus one leg away"
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn daily_distance_conserves_part_length_across_days_without_connectors() {
    let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("TRUNCATE users CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    let owner = db::create_owner(&pool, "Owner", "UTC").await.unwrap();
    let token = generate_device_token();
    let device = db::create_device(&pool, owner.id, "Device", &token)
        .await
        .unwrap();
    let router = app::router(AppState { db: pool.clone() }, None);

    // Movement straddles midnight: 23:50, 23:54, 23:58, 00:02, then a genuine
    // absence of observations, then movement that resumes inside the next day.
    let base = at(23, 50);
    let mut records = offsets(
        base,
        &[(0, 0.0), (4, 2.0 * STEP), (8, 4.0 * STEP), (12, 6.0 * STEP)],
    );
    records.extend(offsets(
        base + 52 * 60_000,
        &[(0, 8.0 * STEP), (2, 10.0 * STEP)],
    ));
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &ndjson(&records))
            .await
            .0,
        StatusCode::OK
    );
    assert!(processing::process_next(&pool).await.unwrap());

    let path = format!("/api/v1/devices/{}/days/", device.id);
    let first = read(&router, &format!("{path}2026-10-05")).await;
    let second = read(&router, &format!("{path}2026-10-06")).await;
    let first_distance = first["summary"]["distance_m"].as_f64().unwrap();
    let second_distance = second["summary"]["distance_m"].as_f64().unwrap();
    assert!(first_distance > 0.0 && second_distance > 0.0);
    // Both days sum only published clipped part lengths.
    for day in [&first, &second] {
        let summed: f64 = day["route_parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| part["visible_distance_m"].as_f64().unwrap())
            .sum();
        assert!((summed - day["summary"]["distance_m"].as_f64().unwrap()).abs() < 1e-9);
        for part in day["route_parts"].as_array().unwrap() {
            assert_route_part(part, "clipped part", TOLERANCE_M);
        }
    }
    // Adjacent days conserve the crossing Part's length within tolerance.
    let crossing = first["route_parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["continues_after"] == json!(true))
        .expect("a part continues into the next day");
    let total = crossing["distance_m"].as_f64().unwrap();
    let tail = second["route_parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["id"] == crossing["id"])
        .expect("the crossing part is published in both days");
    let conserved = crossing["visible_distance_m"].as_f64().unwrap()
        + tail["visible_distance_m"].as_f64().unwrap();
    assert!(
        (conserved - total).abs() < TOLERANCE_M,
        "adjacent day distances sum to the part distance: {conserved} vs {total}"
    );
    assert_eq!(crossing["continues_before"], json!(false));
    assert_eq!(crossing["observed_from_at"], "2026-10-05T23:50:00Z");
    assert_eq!(crossing["observed_until_at"], "2026-10-06T00:02:00Z");
    // The crossing Trip keeps one source identity in both daily projections.
    let trip = crossing["trip_id"].as_str().unwrap();
    assert!(
        second["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == trip && item["continues_before"] == json!(true))
    );
    // Day two's distance adds only its own clipped portions.
    assert!(second_distance > tail["visible_distance_m"].as_f64().unwrap());
    assert!(second_distance < total);

    // The absence is disclosed and never bridged by a Route Part.
    let reasons: Vec<&str> = second["unresolved_intervals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|interval| interval["reason"].as_str().unwrap())
        .collect();
    assert!(reasons.contains(&"missing_observations"), "{reasons:?}");
    // No published part's observed coverage spans the observation absence.
    let absent_from = "2026-10-06T00:02:00Z";
    let absent_until = "2026-10-06T00:42:00Z";
    for part in second["route_parts"].as_array().unwrap() {
        let from = part["observed_from_at"].as_str().unwrap();
        let until = part["observed_until_at"].as_str().unwrap();
        let spans = from < absent_until && absent_from < until;
        assert!(!spans, "no part bridges the absence: {from}..{until}");
    }
    // Movement on both sides of the absence belongs to different Trips.
    let trips: Vec<&str> = second["route_parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| part["trip_id"].as_str().unwrap())
        .collect();
    assert_eq!(trips.len(), 2, "{trips:?}");
    assert_ne!(trips[0], trips[1]);
    assert_eq!(second["evidence_state"], json!("partial"));
}

/// Sub-second spacing between two accepted records at different positions is one
/// continuous movement chain, while a real absence above `observation_gap_s`
/// still ends it. `gps_points` has no uniqueness on `(device_id, recorded_at)`,
/// so two records may share a second.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn sub_second_spacing_stays_one_trip_while_a_real_absence_splits() {
    let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("TRUNCATE users CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    let owner = db::create_owner(&pool, "Owner", "UTC").await.unwrap();
    let token = generate_device_token();
    let device = db::create_device(&pool, owner.id, "Device", &token)
        .await
        .unwrap();
    let router = app::router(AppState { db: pool.clone() }, None);

    // One continuous run whose third and second records share the second 08:08,
    // then a 400 second absence, then movement that resumes.
    let base = at(8, 7);
    let leg = |ts_ms: i64, offset: f64| record(ts_ms, 10.7700 + offset, 106.7000);
    let records = vec![
        leg(base, 0.0),
        leg(base + 60_000, STEP),
        leg(base + 60_400, 2.0 * STEP),
        leg(base + 120_000, 3.0 * STEP),
        leg(base + 180_000, 4.0 * STEP),
        leg(base + 600_000, 5.0 * STEP),
        leg(base + 660_000, 6.0 * STEP),
    ];
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &ndjson(&records))
            .await
            .0,
        StatusCode::OK
    );
    let raw: Vec<String> =
        sqlx::query_scalar("SELECT row_to_json(g)::text FROM gps_points g ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(processing::process_next(&pool).await.unwrap());

    let path = format!("/api/v1/devices/{}/days/2026-10-05", device.id);
    let view = read(&router, &path).await;
    let after: Vec<String> =
        sqlx::query_scalar("SELECT row_to_json(g)::text FROM gps_points g ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(raw, after, "Raw GPS is immutable");
    assert_eq!(raw.len(), 7, "every uploaded record is stored");

    // The absence still splits the day into exactly two Trips.
    let kinds: Vec<&str> = view["timeline"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        ["trip", "trip"],
        "a real absence ends the first Trip"
    );

    let parts = view["route_parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2, "one Route Part per Trip");
    let first = &view["timeline"][0];
    let second = &view["timeline"][1];
    assert_eq!(first["observed_from_at"], "2026-10-05T08:07:00Z");
    assert_eq!(first["observed_until_at"], "2026-10-05T08:10:00Z");
    assert_eq!(first["movement_segment_count"], 1);
    assert_eq!(second["observed_from_at"], "2026-10-05T08:17:00Z");
    assert_eq!(second["observed_until_at"], "2026-10-05T08:18:00Z");

    // The whole run, including both same-second records, is one drawable Part.
    assert_route_part(&parts[0], "sub-second part", TOLERANCE_M);
    assert_eq!(parts[0]["trip_id"], first["id"]);
    assert_eq!(parts[0]["source_record_count"], 5);
    assert_eq!(
        parts[0]["geometry"]["coordinates"]
            .as_array()
            .unwrap()
            .len(),
        5,
        "both same-second observations keep their coordinate"
    );
    let leg_m = STEP * METER_PER_DEGREE;
    assert!((parts[0]["distance_m"].as_f64().unwrap() - 4.0 * leg_m).abs() < TOLERANCE_M);
    assert!((parts[1]["distance_m"].as_f64().unwrap() - leg_m).abs() < TOLERANCE_M);
    // Five connected legs in total; the absence carries none.
    let legs = 5.0 * leg_m;
    // No distance is silently dropped at the sub-second boundary.
    let summed: f64 = parts
        .iter()
        .map(|part| part["distance_m"].as_f64().unwrap())
        .sum();
    assert!(
        (summed - legs).abs() < TOLERANCE_M,
        "published parts carry every leg of the day"
    );
    assert!((view["summary"]["distance_m"].as_f64().unwrap() - summed).abs() < 1e-9);
    let reasons: Vec<&str> = view["unresolved_intervals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|interval| interval["reason"].as_str().unwrap())
        .collect();
    assert_eq!(reasons, ["missing_observations"], "{reasons:?}");
}

/// Daily clipping must rebase visible progress to zero and place an anchor at a
/// calendar boundary without extending the source Part's observed coverage.
#[tokio::test]
#[ignore = "requires PostgreSQL/PostGIS"]
async fn clipping_rebases_progress_and_keeps_observed_coverage_distinct() {
    let pool = db::connect(&std::env::var("LT_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db::migrate(&pool).await.unwrap();
    sqlx::query("TRUNCATE users CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    let owner = db::create_owner(&pool, "Owner", "UTC").await.unwrap();
    let token = generate_device_token();
    let device = db::create_device(&pool, owner.id, "Device", &token)
        .await
        .unwrap();
    let router = app::router(AppState { db: pool.clone() }, None);

    // Movement straddles midnight: 23:58, 23:59 and 00:02 on the next local day.
    let records = offsets(at(23, 58), &[(0, 0.0), (1, 2.0 * STEP), (4, 4.0 * STEP)]);
    assert_eq!(
        upload(&router, &token, Uuid::new_v4(), &ndjson(&records))
            .await
            .0,
        StatusCode::OK
    );
    assert!(processing::process_next(&pool).await.unwrap());

    let path = format!("/api/v1/devices/{}/days/", device.id);
    let first = read(&router, &format!("{path}2026-10-05")).await;
    let second = read(&router, &format!("{path}2026-10-06")).await;
    let before = &first["route_parts"][0];
    let after = &second["route_parts"][0];
    // Observed coverage belongs to the Activity, not to either day.
    assert_eq!(before["observed_from_at"], "2026-10-05T23:58:00Z");
    assert_eq!(before["observed_until_at"], "2026-10-06T00:02:00Z");
    assert_eq!(after["observed_from_at"], before["observed_from_at"]);
    assert_eq!(after["observed_until_at"], before["observed_until_at"]);
    assert_eq!(before["visible_until_at"], "2026-10-06T00:00:00Z");
    assert_eq!(after["visible_from_at"], "2026-10-06T00:00:00Z");
    // Local midnight falls inside a leg, so each day gets a synthetic boundary
    // anchor and its visible distance is a difference in the original progress.
    let leg = 2.0 * STEP * METER_PER_DEGREE;
    let before_distance = before["visible_distance_m"].as_f64().unwrap();
    let after_distance = after["visible_distance_m"].as_f64().unwrap();
    // 23:58..00:00 covers one whole leg and one third of the next.
    assert!((before_distance - leg * 4.0 / 3.0).abs() < TOLERANCE_M);
    assert!((after_distance - leg * 2.0 / 3.0).abs() < TOLERANCE_M);
    assert!((before_distance + after_distance - 2.0 * leg).abs() < TOLERANCE_M);
    // Day one keeps its two observed vertices plus the synthetic midnight anchor;
    // day two keeps the synthetic anchor and its own final vertex.
    assert_eq!(before["vertex_distance_m"].as_array().unwrap().len(), 3);
    assert_eq!(after["vertex_distance_m"].as_array().unwrap().len(), 2);
    // The synthetic anchor is shared by both daily projections.
    assert_eq!(
        before["progress_anchors"][2]["at"],
        after["progress_anchors"][0]["at"]
    );
    // Both clipped parts rebase visible progress to zero.
    for part in [before, after] {
        assert_route_part(part, "rebased part", TOLERANCE_M);
        assert_eq!(part["vertex_distance_m"][0], json!(0.0));
    }
    // Each Trip counts in both days and its time never extrapolates to day end.
    let trip = &first["timeline"][0];
    assert_eq!(trip["kind"], "trip");
    assert_eq!(trip["daily_observed_duration_s"], 120);
    assert_eq!(trip["continues_after"], json!(true));
    assert_eq!(second["timeline"][0]["id"], trip["id"]);
    assert_eq!(second["timeline"][0]["continues_before"], json!(true));
    assert_eq!(second["timeline"][0]["daily_observed_duration_s"], 120);
    assert_eq!(first["summary"]["trip_duration_s"], 120);
    assert_eq!(second["summary"]["trip_duration_s"], 120);
}
