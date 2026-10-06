# 05: Distinguish quality failures, GPS Gaps and Evidence Holes

Status: resolved
Type: task
Labels: ready-for-agent
Blocked by: 04

**What to build:** The Owner sees a truthful processed day when GPS contains jumps, outages or unusable observations: no impossible Route excursion, no invented connector, and separate explanations for missing GPS versus evidence that cannot establish activity.

**Blocked by:** 04 — Show UNKNOWN Trips with raw Route Parts.

## Acceptance criteria

- [x] Classify usable/low-quality/excluded observations deterministically without mutating Raw GPS; configurable implied-speed/jump/quality policies reject impossible excursions from derived geometry.
- [x] Detect temporal GPS Gaps from absence of Raw observations between known observations before quality filtering; do not create Gaps before the first or after the last Raw Record.
- [x] A GPS Gap terminates supported Trip continuity, contributes only Gap duration and does not infer movement/Stop or any straight Route connector.
- [x] Existing but unreliable observations produce Evidence Hole coverage metadata rather than a GPS Gap; activities on either side may have open actual boundaries.
- [x] Expose evidence_holes with projected intervals, reasons and Raw counts separately from Timeline events. Do not assert one Trip or an inferred Stop across the hole.
- [x] Evidence state reflects supported activity coverage: sufficient, partial or insufficient. Entirely unusable data can produce a successful empty view; usable/excluded counts never exceed total Raw count.
- [x] Map and Timeline show disconnected geometry and distinct Gap/Evidence Hole explanations. No DOM marker is created per GPS Record.
- [x] Use fixtures with a genuine timestamp absence, continuous low-quality observations and one impossible jump; test public read responses and rendered distinctions, not private classifier call order.
- [x] Raw timestamps and observations remain unchanged; this slice must not introduce OSRM-based filling or automatic deletion.

## Answer

Implemented and verified. See [operator behavior and acceptance evidence](../../../docs/development/phase-2-quality-gaps-acceptance.md).

## Comments

2026-10-06: Delivered three distinct facts about a Day, each with its own vocabulary and its own surface. `quality` classifies every Raw observation as `Usable`, `LowQuality` or `Excluded` from immutable acquisition metadata plus the centrally configurable policy now on `device_processing_control` (migration 0007, target `quality-gaps-v1`). `gaps` detects temporal absence from the complete Raw series *before* quality filtering, so a poor record never invents an absence; no Gap precedes the first or follows the last Raw Record. `holes` publishes intervals that *contain* observations which cannot support activity, with a reason and a Raw count, separately from the `gap` Timeline events. Raw GPS is never updated or deleted, and no OSRM filling or auto-deletion was introduced.

The old `unresolved_intervals` field conflated absence with unusable observations and was removed from the contract, OpenAPI, tests and fixtures. `evidence_state` now ignores explicit Gaps: reliable activity beside a Gap stays `sufficient`, and only unresolved Evidence Hole coverage makes it `partial`.

A two-axis review of the merge found two real defects. First, the classifier's three-way distinction was published as two ways — `excluded_point_count` was computed as `point_count - usable_count`, so every low-quality record was reported as *impossible*. `low_quality_point_count` is now published separately, the excluded count is the real `Excluded` class, and the three counts always partition `point_count`, asserted through the public Daily View for an impossible jump versus continuous poor-quality observations. Second, the "No DOM marker is created per GPS Record" assertion was vacuous: it compared source and layer counts across two views differing only in `summary.point_count`, and the processed fixture contains no Raw GPS Records, so a per-record marker loop passed unconditionally. It now counts GeoJSON features inside every source and pins the marker-bearing sources to the published Route Parts and Stops; a per-record marker loop fails it.

Standards findings were fixed too: the `absent()` predicate that defines a GPS Gap existed twice (now a single `gaps::absent` that `holes` calls), `evidence.rs` was renamed `holes.rs` to match its single responsibility, `Target` owns the quality `Policy` through `#[sqlx(flatten)]` instead of copying it field by field, the `Observation::usable()` middle man is gone, and the class-to-reason vocabulary moved into `quality`. The unreachable `unsupported_classification` reason was dropped from the OpenAPI enum and the web reason map; it arrives with the classification slice. The 541-line `quality_gap_processing.rs` was split by responsibility into `quality_classification.rs` and `gap_evidence.rs`, with the shared harness in `tests/support/` and the `at()` duplication against `trip_processing.rs` removed.

Two findings were deliberately left alone. `TARGET_COLUMNS` stays a `&str` interpolated into SQL: it is the pre-existing pattern shared by `capture` and `activation`, and changing it is a separate refactor with no ticket behind it. And a Trip ending at a GPS Gap still does not confirm that boundary — ADR-0001 needs evidence on both sides that only later range-expansion work establishes, so the boundary stays open rather than asserting a departure time the observations cannot prove. The criterion permits open boundaries, so nothing is left unmet.

Final validation on the integration branch tip passed: all 26 Rust tests across 14 binaries with every integration file explicitly enabled against a disposable PostGIS 17/3.5 instance, Web typecheck, all 167 Web tests across 24 files, production build, eight Python simulator/protocol tests, `cargo fmt --check`, and strict Clippy across all targets. Two mutations were re-run independently to confirm the new assertions are load-bearing: restoring the subtractive `excluded_point_count` fails 4 tests, and a one-marker-per-GPS-Record loop fails 3.