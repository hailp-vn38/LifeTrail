# 08: Recover worker crashes without stale publication

Status: resolved
Type: task
Labels: ready-for-agent
Blocked by: 07

**What to build:** An operator can restart or reclaim interrupted processing and the Owner still sees the last good snapshot. Work arriving during a running job is retained, and an outdated worker cannot overwrite a newer publication.

**Blocked by:** 07 — Refresh late data with atomic semantic-range publication.

## Acceptance criteria

- [ ] PostgreSQL jobs coalesce dirty work per Device; one in-process worker satisfies acceptance without restricting the schema to a single Device.
- [ ] Claims use short skip-locked transactions, finite leases, durable attempts and renewed fencing tokens. Consistent Raw/target capture and all OSRM work remain outside long-running DB transactions.
- [ ] Expired work can be reclaimed; a worker with an older token cannot activate even if its captured generation and target still match.
- [ ] Ingestion, processing-target updates and publication serialize through the authoritative Device control row; changed target with unchanged Raw input still rejects old candidates.
- [ ] New dirty input during processing is retained/requeued and never cleared by completion of an older claim. Nearby overlapping uploads do not cause redundant independent full jobs.
- [ ] Crash/retry before candidate completion, before activation and after activation is safe: temporary duplicate candidates may exist, but duplicate activation does not.
- [ ] Replay/conflict/authentication/validation/rollback generation rules hold under concurrent requests, not only sequential tests.
- [ ] Failure/status metadata remains visible through API/Web while old publication stays available. Diagnostics identify Device, target, generation, attempt, fencing and publication outcome.
- [ ] Record revision bytes, manifest count, snapshot bytes, candidate bytes and matcher-evidence bytes when available, without automatically deleting any persisted artifacts.
- [ ] Integration tests use controlled concurrency/crash/lease transitions to prove stale-token rejection, no lost work and idempotent activation without brittle wall-clock sleeps.

## Answer

Implemented durable per-Device recovery protocol: skip-locked claims issue monotonically renewed fencing tokens, record every attempt with its captured generations/target and reclaim expired or failed work. Activation now renews at durable boundaries, validates the current fencing authority, and only a matching token may requeue or mark the job idle; stale candidates remain retained but cannot activate. Existing Device-control serialization retains new dirty work while a job runs.

Added immutable-attempt diagnostics and storage measurements for revision, manifest, snapshot, candidate and matcher-evidence bytes. The PostGIS integration test controls an expired lease directly, verifies reclaim to a new token and checks the single successful activation/measurement without wall-clock waits.
