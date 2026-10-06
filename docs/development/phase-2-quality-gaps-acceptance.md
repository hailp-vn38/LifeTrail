# Ticket 05: quality failures, GPS Gaps and Evidence Holes

A processed Day now separates three different situations that previously shared one "unresolved interval" concept. Missing observations are a **GPS Gap** published as a Timeline event. Observations that exist but cannot support activity are **Evidence Hole** coverage with a reason. Rejected observations are **classified** deterministically and withheld from derived geometry only, with Raw GPS untouched.

## Quality classification

`quality::classify` (recorded as `raw-quality-classification-v1`) assigns every captured observation `usable`, `low_quality` or `excluded` from its immutable Raw metadata plus the captured policy. It runs in `capture`, so a published classification is reproducible from the Raw metadata and the recorded policy alone.

Acquisition metadata that cannot support a position — `fix_quality <= 0`, no satellite in view, or HDOP above `max_hdop` — is `low_quality`. An observation is `excluded` from derived geometry when the receiver reports a speed above `max_implied_speed_mps`, or when its displacement from the previous `usable` observation exceeds `max_implied_speed_mps × elapsed` with an absolute `jump_distance_floor_m` for very short intervals. Displacement is measured from the previous *trusted* position, so an already-rejected record cannot push the next reliable record into an impossible jump of its own.

Raw GPS is never updated, deleted or rewritten. `gps_points` keeps the jump, so the Owner can still inspect what the Device recorded, and the jump contributes no Stop, Trip, Route Part, distance or daily duration.

## GPS Gaps

`gaps::detect` (recorded as `observed-gap-detection-v1`) reads the complete Raw series **before** quality filtering, at millisecond precision against `observation_gap_s`. A Gap never precedes the first or follows the last Raw observation, and it is never inferred from quality filtering: low-quality records at existing timestamps produce Evidence Holes instead.

A Gap is published as a `kind: "gap"` Timeline item alongside Trips and Stops, in chronological order with observed and visible bounds, `continues_before`/`continues_after` and `daily_observed_duration_s`, feeding `gap_count` and `gap_duration_s`. It ends Trip continuity, contributes only its own duration, and produces no movement, Stop, matcher call or straight Route connector. Geometry on either side stays disconnected, one Route Part per Trip.

## Evidence Holes

`evidence::holes` produces coverage only for intervals that contain Raw observations. `insufficient_quality` covers unreliable metadata, `insufficient_geometry` covers an impossible position, and `ambiguous_activity` covers reliable observations no Stop or Trip explains. `unsupported_classification` is in the contract vocabulary for the mode-classification slice; a matcher failure alone is never a hole. Each hole publishes a projected day-clipped interval, a reason and the Raw record count, separately from Timeline events.

Neither side of a hole is claimed to be one Trip and no Stop is inferred across it, so activity adjacent to a hole keeps `open` actual boundaries with null `actual_start_at`/`actual_end_at` and null `full_duration_s`. Nothing is extrapolated to now or to the end of the day.

## Evidence state

`evidence_state` reflects supported activity coverage only. No Trip or Stop means `insufficient`, supported activity plus Evidence Holes means `partial`, and supported activity with no unresolved coverage means `sufficient`. An explicit GPS Gap alone does not downgrade `sufficient` to `partial`: an absence is a truthful fact about the Day, not unresolved coverage. An entirely unusable day still publishes a successful empty view, and `usable_point_count + excluded_point_count` never exceeds `point_count`.

## Configurable policy and provenance

Migration `0007_quality_gaps.sql` adds `max_hdop`, `max_implied_speed_mps` and `jump_distance_floor_m` to `device_processing_control`, with the same `target_generation` advance trigger pattern as migration 0005, and advances the target identity to `quality-gaps-v1`. Publication built by the earlier reducer therefore reads as stale until rebuilt, and a policy change retires the current publication the same way. Each Activity Revision records the policy and the algorithm list in `config`, and snapshots report `reducer_version: 2`.

## API and Web

The canonical contract adds the `DailyGap` schema and the `gap` activity kind, replaces `UnresolvedEvidence` with `EvidenceHole` and its reason enum, and removes `unresolved_intervals` from `DailyView`. Web types are regenerated with `npm run api:generate`.

`web/src/features/timeline/evidence.ts` owns both explanations. A Gap renders as its own Timeline item titled "GPS Gap" stating that no movement, Stop or connector is inferred. `DailyEvidenceNotice.vue` lists only Evidence Holes with their reason and Raw record count, and never claims observations are missing. The Timeline kind union gained `gap`, with a distinct marker. The Map draws each published Route Part as its own GeoJSON line, so a Gap reads as a break in the geometry, and it creates no marker per GPS Record.

## Boundary of this slice

No OSRM matching, mode classification, cross-midnight work, processed playback clock or incremental range reuse is included; those are tickets 06–15. A Trip ending at a GPS Gap does not confirm that boundary: the ADR-0001 Stable Segmentation Boundary rule needs evidence on both sides that only later range-expansion work establishes, so the boundary stays open rather than asserting a departure time the observations cannot prove. `gps/1` ingestion semantics and firmware are unchanged.

## Validation evidence

- `quality_gap_processing.rs`: real HTTP upload → worker → PostGIS → Daily View reads. A genuine absence publishes `trip, gap, trip` with a 660 second Gap, no Stop, no bridging Route Part, `gap_duration_s` 660, `evidence_state: sufficient` and Raw rows identical before and after. An impossible jump keeps all seven Raw records, reports `usable 6 / excluded 1`, discloses one `insufficient_geometry` hole, keeps both Trips' boundaries open and adds no distance. Low-quality records at the *same* timestamps as the absence case produce zero Gaps, one `insufficient_quality` hole of seven records, two Trips and `partial` evidence.
- `quality_gap_processing.rs` fixtures (`fixtures/quality_gaps.json`): nine scenarios covering a genuine absence, continuous low-quality observations, one impossible jump, a Gap and a hole in one day, an entirely unusable day, ambiguous activity, poor HDOP, a fix without satellites and a receiver reporting impossible speed. Each asserts chronological kinds, Gap bounds, hole reason/bounds/record count, Trip bounds, published distance, evidence state and the exhaustive Raw counts.
- `quality_gap_processing.rs` configurable policy: relaxing `max_hdop` reclassifies the same Raw records into a Stop, the previous publication stays readable as `stale`, `target_generation` advances by one, and the Activity Revision's `config` records the policy plus both algorithm identities.
- Existing ticket 01–04 tests stay green, including the previously `missing_observations` assertions in `trip_processing.rs` and `stationary_fixtures.rs`, now expressed as Gap Timeline events and renamed hole reasons.
- Mutation checks: treating every observation as usable, dropping the implied-speed/jump policy, detecting Gaps only between usable observations, connecting movement across a Gap, letting a Gap downgrade evidence to `partial`, dropping the policy from provenance and removing the migration's target bump each fail the suite.
- Web: `features/timeline/evidence.test.ts`, `timeline/events.test.ts`, `components/RouteMap.test.ts` and `DailyMapPage.test.ts` cover the distinct Gap and Evidence Hole explanations, the Gap Timeline item without a coordinate, disconnected Part geometry, no marker per GPS Record at 30 000 records versus 2, and the hole notice never claiming missing observations. A mutation that reuses the absence message for a hole fails three tests.
- The real published responses (day with a Gap, a hole and a jump, adjacent day, `?view=raw`, empty day) validate against the canonical OpenAPI 3.1 contract with a JSON Schema 2020-12 validator. Five deliberate mutations — a Gap carrying a center, an extra hole property, the old `missing_observations` reason, a Gap missing `daily_observed_duration_s`, and a reintroduced `unresolved_intervals` — all fail validation.

Run server integration checks with a disposable PostGIS database and `LT_TEST_DATABASE_URL`; these tests truncate their test data and must run serially (`--test-threads=1`). They are marked ignored by default and must be explicitly included for acceptance.

Final validation on 2026-10-06 passed: all 28 Rust tests, including every integration file explicitly enabled with `--include-ignored --test-threads=1` against a disposable PostGIS 17/3.5 instance; Web typecheck and all 165 tests across 24 files; production build; eight Python simulator/protocol tests; `cargo fmt --check`; strict Clippy across all targets; and canonical OpenAPI validation of the real published responses plus five rejected mutations.