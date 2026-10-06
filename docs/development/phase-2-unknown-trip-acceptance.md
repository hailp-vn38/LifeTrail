# Ticket 04: UNKNOWN Trips with raw Route Parts

Movement between Stops is now a Trip of ordered UNKNOWN Movement Segments with a raw-derived Route. Trips are derived on the Device's continuous UTC history, projected into Owner-local days and published through the existing snapshot, activation and status contract. Raw GPS is unchanged and stays separately selectable.

## Derivation

`continuous-movement-raw-v1` (recorded in each Activity Revision's `config.algorithms`) treats an observation as usable movement when it is inside the observation-gap threshold of its predecessor and outside every Stop. The first usable observation opens a run; a Stop, an unusable observation or an actual absence of observations ends it. A pause shorter than the Stop minimum therefore stays inside one run and never creates an artificial Trip boundary. A run whose positions never differ is not drawable, so no Part is produced for it.

The gap threshold is compared at millisecond precision, like the Stop and unresolved-evidence detectors. `gps_points` has no uniqueness on `(device_id, recorded_at)` and records carry millisecond timestamps, so two accepted Raw GPS Records may share a second. A sub-second spacing is an immediate re-observation rather than an absence: comparing whole seconds would end the run and split one continuous chain into two Trips. Only a gap longer than `observation_gap_s` ends a run.

Each Trip gets one Movement Segment per run, with `mode: "unknown"` and `source: "raw"`. This slice introduces no OSRM request, profile selection or mode classification. `movement_segment_count` and the ordered `movement_segments` array are the published shape, so a later classification slice adds segments without changing the read model.

A Trip boundary is `confirmed` only when an adjoining Stop confirms it. Dataset edges, actual observation absences and unusable-record Evidence Holes leave `actual_start_at`/`actual_end_at` null with an `open` boundary state, and `full_duration_s` stays null unless both boundaries are confirmed. Observed bounds and duration are always known, and no interval is extrapolated to the current time or day end.

`evidence::unresolved` now resolves an interval when both of its observations belong to a derived Stop or Trip, so a Stop→Trip transition is activity rather than missing evidence. An actual observation absence or an unusable Raw record is still disclosed even when derived activity covers both sides, because a Trip must never imply continuity across it.

## Route Parts and progress

`route_parts` is the canonical processed geometry. Each Part identifies its `trip_id` and `movement_segment_id`, its `source`, and its observed coverage, which belongs to the Movement Segment rather than to any single day. Coordinates come from accepted GPS Records one-to-one: a coordinate is never duplicated to satisfy the LineString contract and a timestamp is never invented.

The server owns the progress metric. `vertex_distance_m` is aligned one-to-one with `geometry.coordinates`, starts at zero, is non-decreasing, and its final value equals the part length within numerical tolerance. `progress_anchors` map historical instants to progress, with strictly increasing times, non-decreasing and bounded progress, and anchors at both endpoints. There is deliberately no one-timestamp-per-vertex contract.

Daily clipping (`clip`) keeps the observed portion inside the day, places a synthetic anchor at a calendar boundary and rebases visible progress to zero. `visible_distance_m` is the difference in original progress at the clipping endpoints, so adjacent days sum to the source part length within tolerance. `continues_before`/`continues_after` report observed coverage outside the day only.

Daily `distance_m` sums published visible part lengths. Rejected geometry and implicit connectors contribute nothing, and no part bridges an observation absence. Trip time is published as `trip_duration_s`, includes pauses below the Stop criteria, and is never labeled physical moving duration; the Web labels it "Thời gian trong Trip" with an explicit note that short pauses are included.

## Boundary of this slice

Processed Route Parts are not fed into the legacy Raw-only playback clock. `route`, `start` and `end` stay null in a published snapshot, so the existing Raw playback controls remain bound to the Raw projection and processed playback stays ticket 13. GPS Gap activities, classification, OSRM matching, chunk seams, incremental manifest splicing and processed playback remain later tickets.

Migration `0006_trip_movement_segments.sql` advances the processing target identity to `trips-v1`, so publication built by the earlier reducer reports as stale until it is rebuilt. Raw GPS is not modified, previous revisions/manifests/snapshots remain readable, and an existing publication is retained while a replacement is staged.

## API and Web

The canonical contract adds `RoutePart`, `ProgressAnchor`, `MovementSegment`, `DailyTrip` and a `DailyActivity` union over `DailyActivityBoundaries`, and `route_parts`/`timeline` now describe processed activity. Composite Trip/Stop schemas use `unevaluatedProperties` so shared boundary fields are validated rather than rejected.

`web/src/features/activity/model.ts` narrows the published union for Timeline and Map. Trip Timeline items show observed time, daily overlap including short pauses, the sum of server-published part lengths and the ordered segment modes with `unknown` shown as "chưa xác định". `web/src/map/route-parts.ts` draws one GeoJSON line per published Part, highlights and fits the selected Trip, and selects its Timeline item; it never recomputes a length. Trip selection, Stop selection, the Raw GPS toggle and the empty view are unchanged, and selection is still reset on publication change.

## Validation evidence

- `trip_processing.rs`: real HTTP upload → worker → PostGIS → Daily View reads. Two Stops with a Trip between them whose 120 second pause stays inside Trip time; chronological `stop, trip, stop, trip` projection; one UNKNOWN raw segment per Trip; confirmed and open boundaries with distinct actual/observed/visible values; server progress invariants per Part; Raw rows unchanged before and after processing; Raw view still selectable with all coordinates; `route`/`start`/`end` null; no `moving_duration_s`; event and Part identity carried from the creating Activity Revision.
- `trip_processing.rs` distance conservation: a Trip crossing local midnight plus a 40 minute absence. Both days sum only their clipped part lengths, adjacent-day distances sum to the part distance within 0.5 m, one Trip identity spans both days, no part bridges the absence, movement either side belongs to different Trips, and evidence stays partial.
- `trip_processing.rs` clipping: midnight inside a leg produces a shared synthetic anchor, per-day visible progress rebased to zero, conserved total distance and per-day Trip time of 120 s.
- `trip_processing.rs` repeated visits: two Stops at the same location stay distinct, the middle Trip is confirmed on both sides, sparse two-minute geometry keeps one coordinate per record and one segment per Trip.
- `trip_processing.rs` sub-second spacing: two accepted Raw GPS Records sharing a second at different positions stay inside one Trip and one Route Part carrying both coordinates and all distance, while a real 400 second absence still ends the Trip and is disclosed as `missing_observations`. Anchor order is compared as instants, because an optional fractional part breaks lexical RFC 3339 order.
- `stationary_fixtures.rs` with `fixtures/stationary.json` extended: clean movement, short pause, repeated visits, sparse geometry, bad fixes, high HDOP, absent observations and configured radius/duration, asserting Stop durations and boundaries plus Trip count and per-Trip raw UNKNOWN segments. Zero-speed moving traces become Trips, which is the intended new behavior.
- Mutation checks: adding a constant to the published progress origin and forcing a two-coordinate Part from one observation each fail the suite, so the progress and no-fabricated-coordinate assertions are load-bearing.
- Web: `features/activity/model.test.ts`, `timeline/events.test.ts`, `components/RouteMap.test.ts` and `DailyMapPage.test.ts` cover Trip rendering, chronological ordering, published-distance consumption, Route Part drawing without recomputation, Trip highlight/fit/map-to-Timeline selection, the Raw toggle after publication, and the unchanged Stop behavior.
- The real published response (processed day, adjacent day, `?view=raw`, single-record day and status) validates against the canonical OpenAPI 3.1 contract with a JSON Schema 2020-12 validator.

Run server integration checks with a disposable PostGIS database and `LT_TEST_DATABASE_URL`; these tests truncate their test data and must run serially (`--test-threads=1`). They are marked ignored by default and must be explicitly included for acceptance.

Final validation on 2026-10-06 passed: all 22 Rust tests, including all integration files explicitly enabled with `--include-ignored --test-threads=1` against a disposable PostGIS 17/3.5 instance; Web typecheck and all 158 tests across 23 files; production build; eight Python simulator/protocol tests; `cargo fmt --check`; strict Clippy across all targets; and canonical OpenAPI validation of the real published responses.
