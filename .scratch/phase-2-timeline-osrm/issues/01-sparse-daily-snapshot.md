# 01: Publish Daily Snapshots for sparse evidence

Status: claimed
Type: task
Labels: ready-for-agent
Blocked by: None (can start immediately)

**What to build:** The Owner can upload a single GPS Record, watch durable background processing complete, and open a truthful processed Daily View with insufficient evidence instead of an invented Trip, Stop or Route. This is the first narrow ingestion-to-worker-to-publication-to-Web slice; existing Raw views remain usable while later slices add activity processing.

**Blocked by:** None (can start immediately).

## Acceptance criteria

- [ ] A newly committed valid Batch advances the Device input generation once and durably schedules work in the same transaction; replay/conflict/authentication or validation failure/rollback does not advance it.
- [ ] The worker builds a candidate from a consistent input/target capture and activates a minimal immutable manifest/snapshot through a short Device-control-row-serialized transaction with generation, target and fencing checks; no processing dependency call is inside a DB transaction.
- [ ] The first slice supports empty or single-observation test days without creating an empty-length activity revision or a fake LineString. A single Record yields insufficient evidence with no activity events or drawable parts.
- [ ] Persisted publication identity is distinct from processing/activity identity. Published snapshots are immutable and expose their source input, manifest and projection provenance.
- [ ] Canonical OpenAPI and generated Web types expose separate background state, freshness/availability and evidence state; an idle successful empty snapshot is not a failed job.
- [ ] Point counts and first/last observation timestamps include all Raw records in the selected day; missing Devices remain 404 and existing empty days remain valid responses.
- [ ] The Web renders processed insufficient-evidence/status states while preserving the existing Raw response and UI for not-yet-processed histories. Do not falsely publish unsupported activity as absent evidence.
- [ ] The new contract is introduced compatibly with existing Raw reads and firmware ingestion, keeping type checks and existing tests green; do not replace the application shell.
- [ ] An integration test uploads a Batch through the HTTP contract, runs the real worker with PostGIS, and reads the snapshot/status; Web tests verify the resulting empty/status presentation.
- [ ] Raw GPS and persisted processing artifacts remain intact; there is no automatic cleanup or OSRM dependency in this slice.
