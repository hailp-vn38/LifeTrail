# 04: Show UNKNOWN Trips with raw Route Parts

Status: resolved
Type: task
Labels: ready-for-agent
Blocked by: 03

**What to build:** The Owner can see movement between Stops as a Trip with ordered UNKNOWN Movement Segments, inspect its raw-derived Route, and select the Trip from either Timeline or Map. Route Parts establish the shared geometry/progress contract before matching is introduced.

**Blocked by:** 03 — Show observed Stops in the Timeline.

## Acceptance criteria

- [x] Derive continuous Trip movement bounded by supported Stop transitions or open observation edges; a short pause below Stop criteria does not itself end a Trip.
- [x] A Trip contains ordered Movement Segments rather than one movement_segment_id. This slice uses UNKNOWN mode without guessing an OSRM profile.
- [x] Each drawable raw Route Part identifies its segment/source/observed coverage and has a valid LineString, strictly increasing historical-time anchors and non-decreasing bounded progress, including endpoint anchors.
- [x] Publish server-computed vertex_distance_m aligned with coordinates, zero origin and final part distance; never fabricate a second coordinate or timestamp.
- [x] Daily distance uses published part lengths without invented connectors. Trip time includes short pauses and is labeled trip_duration_s rather than physical moving duration.
- [x] Projected Trip/Stop items are chronological, preserve actual/observed/visible boundary distinctions, and have IDs from their creating activity revision.
- [x] Trip selection highlights/fits visible geometry; selecting its map Route selects the Timeline item. The complete-day Route remains distinguishable.
- [x] Expose generated API types and static processed rendering without assuming one timestamp per geometry vertex. Processed playback is delivered separately; do not pass parts into the legacy Raw-only clock contract.
- [x] Tests cover clean movement, short pauses, repeated location visits and sparse geometry, asserting upload-to-Trip response/render behavior and server progress invariants.

## Answer

Implemented and verified. See [operator behavior and acceptance evidence](../../../docs/development/phase-2-unknown-trip-acceptance.md).

## Comments

2026-10-06: Delivered continuous-UTC movement runs as Trips with one ordered raw UNKNOWN Movement Segment per run, server-owned `vertex_distance_m` and strictly increasing historical progress anchors, anchored daily clipping with conserved distance, confirmed/open boundary states, and synchronized Trip highlight/fit plus map-to-Timeline selection. No OSRM matching or mode classification was introduced. Raw GPS remains immutable and the `?view=raw` path stays selectable; processed Route Parts deliberately stay out of the legacy Raw-only playback clock.

Review found one real defect: `continuous()` compared whole seconds and required a positive elapsed value, so two accepted Raw GPS Records sharing a second ended a movement run and split one continuous chain into two Trips and two Route Parts. Since `gps_points` has no uniqueness on `(device_id, recorded_at)`, that input is legitimately reachable. The threshold is now compared at millisecond precision, consistent with the Stop and unresolved-evidence detectors, and a regression test keeps a same-second pair in one Trip while a real 400 second absence still splits. Anchor order is now compared as instants because an optional fractional second breaks lexical RFC 3339 order.

Final validation on the integration branch tip passed: all 22 Rust tests with every integration file explicitly enabled against a disposable PostGIS 17/3.5 instance, Web typecheck, all 158 Web tests across 23 files, production build, eight Python simulator/protocol tests, `cargo fmt --check`, and strict Clippy across all targets. GPS Gap activities, classification, OSRM matching, chunk seams, incremental range reuse and processed playback remain tickets 05 and 07 and 10 through 13.
