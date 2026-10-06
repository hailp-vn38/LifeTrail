# LifeTrail Phase 2 Test Data Suite

Deterministic fixture suite for the Phase 2 processed GPS domain and publication architecture. Matcher decisions are deferred.

## What is included

- `master/`: staged E2E Raw GPS scenario using valid `gps/1` NDJSON.
- `matcher-fixtures/`: historical fixtures deferred to the future routing phase; excluded from Phase 2 acceptance.
- `progress-fixtures/`: server-owned progress/distance clipping cases.
- `worker-fixtures/`: input-generation, target and fencing races.
- `projection-fixtures/`: timezone-only reprojection.
- `publication-fixtures/`: revision-scoped identity and range replacement.
- `sparse-fixtures/`: valid but insufficient evidence.
- `scale-30000/`: exactly 30,000 valid Raw GPS records for performance acceptance.
- `TEST-MATRIX.md`: maps Q1-Q27 to concrete fixtures.

## Master ingestion order

1. Upload all batches in `master/stage-01-base/`.
2. Process and publish.
3. Upload `master/stage-02-late-midnight-gap-fill/`.
4. Reprocess. This removes the midnight Raw Gap and creates one cross-midnight Stop,
   requiring atomic publication of both affected Daily Views.
5. Upload `master/stage-03-overlap-better-quality/`.
6. Reprocess. Activities should remain equivalent, while generic exact-timestamp selection
   deterministically prefers the new higher-quality overlapping observations.
7. Run `master/stage-04-replay-and-conflict/` to verify `input_generation` does not
   increment for an identical replay or a same-ID conflict.

## Master scenario highlights

- Dataset begins inside a Stop -> left OPEN boundary.
- One Trip contains WALK then CAR without a qualifying Stop between them.
- 12:05-12:10 contains Raw records but deliberately poor quality -> Evidence Hole, not Gap.
- 17:45-17:55 contains no Raw records -> true Gap.
- 18:35 alternates HDOP 3.5/4.5/5.5/6.5. Short rejected runs bracketed by
  usable GPS should preserve activity continuity under the bounded evidence policy,
  rather than create repeated four-record Evidence Holes. Rejected records remain
  in Raw GPS and the low-quality count, but do not enter processed geometry.
- 18:45 contains one impossible ~kilometre-scale jump that remains in Raw GPS but must be
  excluded from published derived geometry.
- Stage 1 omits 23:55-00:05, creating a midnight Gap.
- Stage 2 arrives late and fills that interval, proving a Stop from 19:00 to 00:20 across
  two Owner-local days.
- Dataset ends inside a Stop -> right OPEN boundary.
- Stage 3 adds overlapping same-timestamp records from a different Batch with better
  fix quality, lower HDOP and more satellites.

## Protocol properties

Every generated Batch in the master and scale suites:

- is strict UTF-8 NDJSON;
- ends in LF;
- uses only `gps/1` fields;
- has strictly increasing `ts_ms` within the Batch;
- includes an immutable manifest with SHA-256, byte length, record count and first/last time;
- uses a deterministic canonical UUIDv4 Batch ID;
- respects the 300-second rotation boundary.

Timestamp overlap is intentionally present across different Batches in Stage 3.

## Important distinction

The static master traces use deterministic synthetic HCMC-region polylines and GPS noise.
They are designed for repeatable processing tests without network access.

Phase 2 requires no routing engine. Historical OSRM tooling and matcher fixtures are preserved only for a future optional enhancement.

Run the executable processing fixture and scale acceptance tests against a dedicated test database:

```sh
LT_TEST_DATABASE_URL=postgres://... cargo test --manifest-path server/Cargo.toml \
  --test phase2_acceptance -- --include-ignored --test-threads=1 --nocapture
```

The scale test reports worker wall time, process peak RSS (including test harness/ingestion), snapshot DB read latency, Daily View API read latency and uncompressed JSON payload bytes. No performance SLA is inferred from these measurements.

## Counts

Stage 1 Raw records: 20820
Stage 2 late records: 120
Stage 3 overlap records: 61
Scale records: 30000

See `master/oracle.json` and `TEST-MATRIX.md` for expected semantics.
