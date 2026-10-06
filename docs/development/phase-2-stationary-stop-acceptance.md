# Ticket 03: observed Stops in the Timeline

The worker now captures continuous UTC observations independently of Owner-local days, detects spatial dwell, persists an immutable Activity Revision, and stages all daily projections under one manifest. Activation retains the existing input/work/target/timezone/fencing checks and swaps all affected daily pointers in one transaction. Reprocessing preserves previous revisions, manifests and snapshots; Raw GPS is unchanged.

## Detection and boundaries

`anchored-spatial-dwell-v1` uses the first reliable observation as the server-derived center of a dwell disk. Every member must remain within the configured radius; successive nearby points cannot drift indefinitely. The published radius is the greatest observed distance from that center. Speed is not a detection input. A fix must have `fix_quality > 0` and absent or at most 5 HDOP. Acquisition metadata remains untouched in Raw GPS.

Per-Device configuration lives on `device_processing_control`: `stop_radius_m` defaults to 30, `stop_min_duration_s` to 180, and `observation_gap_s` to 300. A configuration change advances `target_generation`, invalidating captured candidates. Queue processing with the established `process-day --device-id <UUID> --date <YYYY-MM-DD>` operator command after changing configuration or to backfill history; reads never derive activity synchronously. Configuration and algorithm identity are retained with each revision.

A reliable outside-disk observation within the absence threshold confirms the adjoining boundary at the first/last inside-disk observation. Dataset edges, missing observations and unusable records leave actual boundaries null/open. Observed bounds and duration remain known. Full duration exists only when both boundaries are confirmed. Calendar clipping changes visible bounds and daily observed duration, retaining the source identity and full observed interval. Only strictly positive observed overlap counts a Stop; midnight alone adds none. No current-time or day-end extrapolation is used.

Unusable Raw intervals are exposed as Evidence Holes. Actual observation absences and activity not yet supported by the processor are exposed separately as `unresolved_intervals`, with explicit reasons. This slice publishes no Trips, Gap activities, movement geometry or guessed stationary time for those intervals. Evidence state is sufficient for complete Stop coverage, partial when a Stop coexists with unresolved intervals, and insufficient when no activity is established. Existing Raw views and valid empty/single-record views remain usable. `?view=raw` bypasses the processed snapshot through the established Raw projection, and the Daily Map offers a Raw GPS / activity toggle with separate query identities. Raw route playback and Start/End selection remain available after publication, including moving traces with no Stop.

This first Stop slice rebuilds the full captured Device observation range at genuine observation edges, including neighboring daily projections. Incremental semantic-range splicing/reuse and broader activity processing remain ticket 07 and subsequent activity slices; there is no claim of incremental performance here.

## API and Web

The canonical OpenAPI contract and generated Web types expose revision-local Stop identity, source counts/IDs, center/radius, quality, observed/actual boundaries, daily overlap and continuation flags. Timeline shows observed time/duration, daily observed duration and known or unknown arrival/departure. Statistics show daily Stop count and observed duration.

MapLibre uses GeoJSON layers for Stop centers and geographic meter-radius disks. Timeline selection highlights the center/disk and focuses the center at an appropriate zoom. Selecting either map feature selects the same Timeline item through shared Pinia selection state. A publication or timezone change remounts the map, clearing selection and playback.

## Validation evidence

- `stationary_processing.rs`: real HTTP upload → worker → PostGIS → daily reads; jitter with positive reported speed; one 23:50–00:20 Stop shared across two dates with 600/1200 seconds overlap; open bounds; exact observed times; Raw row equality; immutable retained activity/history; new revision-local identity; late input closing both boundaries; zero overlap at midnight.
- `stationary_fixtures.rs` with `fixtures/stationary.json`: short pause, moving coordinates despite zero speed, bad fixes, high HDOP, absent observations, quality holes, repeated confirmed visits, configurable duration/radius. No invented routes.
- `stationary_publication.rs`: a configuration change after capture rejects the staged candidate, keeps the old snapshot readable/stale, and publishes a fresh candidate without advancing Raw generation.
- Updated sparse regression retains replay/authentication/conflict checks, valid empty/single-record responses, operator enqueue race and timezone fallback; dense unresolved input now publishes explicit evidence instead of deferring indefinitely.
- Web component/model coverage verifies observed/unknown labels, daily totals, Stop-only map display, map disk layers, center focus and bidirectional selection, plus established Raw/empty states and switching to the explicit Raw query.
- Built Web opened in Chromium against the real running server and isolated PostGIS acceptance database. Timeline selection and clicking the center marker both selected the same item; unknown boundary labels rendered without page errors. The same browser run switched to the Raw route and back, verified original Start/End items, and confirmed selection resets. A local blank basemap isolates these interactions from external tile services. Evidence lives in ignored `runtime/stationary-stop-acceptance/` (`daily.json`, `selected-stop.png`, `browser-result.json`). Device credentials remain only in ignored runtime files.

Run server integration checks with a disposable PostGIS database and `LT_TEST_DATABASE_URL`; these tests truncate their test data and must run serially (`--test-threads=1`). They are marked ignored by default and must be explicitly included for acceptance.

Final validation on 2026-10-06 passed: all 13 Rust tests, including all seven PostGIS integration files explicitly enabled with `--include-ignored --test-threads=1`; Web typecheck and all 149 tests across 22 files; production build; eight Python simulator/protocol tests; `cargo fmt --check`; strict Clippy across all targets. The real published response validates against canonical OpenAPI 3.1. Independent Standards/Spec reviews rechecked the naming and Raw-view fixes with zero remaining findings; see [review](../../.scratch/phase-2-timeline-osrm/review-03.md).
