# LifeTrail Phase 2 — Processed GPS Timeline and Published Daily Snapshots

Status: implemented
Type: spec
Date: 2026-10-06

This is the current Phase 2 spec. The directory slug is retained for issue links. The previous matcher spec is historical Git content, superseded by [the alignment guide](../../docs/lifetrail-phase2-post-implementation-alignment.md) and [ADR-0007](../../docs/adr/0007-phase2-processed-gps.md).

## Scope

Ingest immutable Device Batches, evaluate GPS quality/outliers, derive continuous UTC activity, publish Owner-local Daily Views atomically and render/play back published history. Runtime requires Web, Rust server/worker and PostgreSQL/PostGIS only. Routing/map matching, matcher retries/confidence/chunking and road-profile URLs are deferred to a later phase.

## Processing

- Raw GPS coordinates and millisecond timestamps remain immutable, including poor-quality and excluded records.
- Derived geometry uses `source = processed_gps` for every mode: WALK, BIKE, CAR, UNKNOWN (wire values retain existing lowercase mode vocabulary).
- Exact timestamp duplicates select a deterministic representative by usable class, fix quality, HDOP, satellites and record ID. Distinct millisecond epochs remain distinct. Counts include all Raw observations.
- Gap requires absence of Raw observations between known boundaries. It ends continuous journeys and never acquires a connector or inferred movement.
- Evidence Hole contains observations that cannot support reliable activity/geometry. It is separate from Gap and from processing failure, and need not be a Timeline event.
- Short failures bracketed by plausible usable evidence may be excluded without creating holes. Long or unbounded failures retain unresolved coverage.
- A Trip contains ordered Movement Segments. Mode changes and short pauses do not themselves split a Trip; qualifying Stops and Gaps do.
- Actual activity boundaries stay nullable/open where unconfirmed. Observed boundaries describe only available evidence; full duration is null unless confirmed. Never extend an open interval to now or day end.

## Geometry and playback

Route Parts carry contiguous processed GPS LineStrings, historical progress anchors and cumulative server-owned `vertex_distance_m`. Progress starts at zero, never decreases, and anchors have strictly increasing instants. Parts never connect across Gap/Evidence Hole. Distance sums published parts only. Daily clipping interpolates at calendar boundaries using progress differences, conserving distance across days.

## Revision and publication

Activity Revisions and Activity Manifests are immutable and independent of calendar days. New revisions own only their continuous replacement range; immutable manifest slices reuse activity outside it. Safe cuts conservatively use unchanged Raw Gaps or observation edges; without a safe cut the range expands. Configuration changes require complete reprocessing. Reducer context capture still reads/classifies complete history; no incremental CPU/DB performance claim is made.

Each newly committed Batch increments `input_generation` once, including late, overlapping and poor-quality data. Replay, conflict, validation/authentication failure, rollback, config/version and timezone changes do not increment it.

PostgreSQL stores durable Device jobs. One worker uses leases and fencing. Candidate processing occurs outside the short activation transaction. Publication verifies generation, work/processing/projection target and fencing authority, then atomically activates every affected daily snapshot. Failed or stale candidates never replace the last good snapshot.

Timezone belongs to projection only. Changing Owner timezone reuses published UTC activity without rerunning quality, segmentation or classification; quality counts are projected from immutable per-observation processing audit, and Raw counts may include newer pending Batches; stale source freshness is disclosed while newer Raw data remains pending.

## API and Web

Daily View serves one complete publication with Timeline, Route Parts, summary, evidence and provenance. Activity revision identity and published snapshot identity remain distinct. Summary distinguishes Raw/usable/low-quality/excluded counts, Trip/Stop/Gap counts and durations. Trip duration can include short pauses.

Sparse/poor-evidence days may publish successfully with empty Timeline/geometry and `evidence_state = insufficient`. Evidence state is independent of processing state.

Web polls lightweight processing status every five seconds while queued/running and visible. Hidden tabs stop polling; restoring visibility/focus fetches status immediately. A changed publication replaces the full Daily View and resets selection/playback. The first processed publication also replaces an initially Raw view. Evidence reason rendering has a safe unknown-reason label.

## Acceptance

Run Rust unit and real PostGIS integration tests serially in a dedicated test database, Web tests/typecheck/build and Python host-tool tests. `server/tests/phase2_acceptance.rs` runs the staged master and 30,000-record fixture against ingestion, worker and Daily View. Check Raw immutability, quality continuity, multimode journeys, open boundaries, gaps/holes, late data, replay, range reuse, cross-midnight publication, timezone projection, monotonic progress and daily distance conservation.

Measure processing wall time, process memory, snapshot DB read latency and Daily View payload for one Owner, one Device and 30,000 Raw records/day. Measurements and practical limitations belong in the acceptance report; no undocumented performance threshold is implied.
