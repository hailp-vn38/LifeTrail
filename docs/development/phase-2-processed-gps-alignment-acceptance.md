# Phase 2 processed GPS alignment acceptance

Verified on 2026-10-06 (Asia/Ho_Chi_Minh) against the current processed-GPS baseline and ADR-0007.

## Result

Phase 2 derives/publishes GPS geometry without routing clients, retry/confidence policies, profile URLs or routing Compose services. Newly published Route Parts and Movement Segments use `processed_gps`. Raw GPS remains unchanged. Historical matcher migrations, immutable JSON evidence and future routing tools are retained as history; migration 0015 retires active matcher schema and queues replacements.

Daily snapshots are built from authoritative manifest slices. Late data replaces only a continuous range bounded conservatively by unchanged Raw gaps or observation edges. Activity outside that range retains its creating revision. Quality classifications and timestamps are retained as generic audit for timezone-only projections. Geometry shares transition vertices between modes so mode changes do not discard a leg or split a Trip.

## Validation

- Rust: all 42 unit/integration tests passed with `--include-ignored --test-threads=1`, using a dedicated PostgreSQL/PostGIS container/database on localhost port 55434.
- Master fixture: base ingestion, late midnight-gap fill, overlap/better quality, exact replay and same-ID conflict passed through the real ingestion endpoint and durable worker.
- Scale fixture: 100 Batches, exactly 30,000 immutable Raw observations, one Owner and one Device, generation delta 100, processed publication current, geometry/progress invariants valid.
- Publication safety: captured input-generation/configuration/fencing races retain the old publication; failed snapshot staging retains the complete processed view; lease expiry is reclaimed.
- Quality/geometry: short failures and isolated jumps, long/unbounded holes, true gaps, sparse empty geometry, millisecond observations, Stops/Trips, cross-midnight clipping and distance conservation passed.
- Timezone: projection reuses the same manifest/revisions with stale source disclosed, preserving source quality counts and reducer provenance without a new quality pass.
- Web: 184 tests across 30 files passed; API type generation, Vue/TypeScript checking and production build passed. Added tests cover first publication replacing Raw, changed publication refetch, and hidden/visible status events. Existing tests cover selection/playback reset and evidence labels.
- Python: all 9 host-tool tests passed with `PYTHONPATH=tools`.
- Compose service validation, shell syntax, Rust formatting and `git diff --check` passed. The test image now copies fixture assets and compiles/runs the complete suite by default.

Reproduce Rust acceptance against a dedicated test database (the tests truncate its data):

```sh
LT_TEST_DATABASE_URL=postgres://... cargo test --manifest-path server/Cargo.toml \
  -- --include-ignored --test-threads=1 --nocapture
```

## Scale measurements

Final local debug-build run, with routing containers stopped:

| Measurement | Result |
|---|---:|
| Worker processing + staging + activation | 3,424 ms |
| Snapshot DB read + JSON decoding | 147 ms |
| Daily View through in-process Axum request | 460 ms |
| Snapshot JSON bytes, uncompressed | 3,369,392 |
| Daily View JSON bytes, uncompressed | 3,369,632 |
| Test-process peak RSS (`VmHWM`) | 168,384 KiB (164.4 MiB) |

Peak RSS includes the integration harness, ingestion, master fixture and processing in the same test process; it is not an isolated production-worker measurement. API latency includes serialization/body consumption and excludes LAN transport. This is one local run, without an inferred SLA or production performance claim. The scale fixture's date is read from the generated snapshot rather than assuming the master scenario's date.

## Fixture findings and limits

The master includes a roughly 1 km discontinuity at Owner-local 18:41 in addition to the intentionally injected 18:45 jump. Under the captured default speed policy, 14 records are excluded, rather than just the single advertised injected jump. They remain Raw GPS. The bounded 18:35 fixture avoids repeated tiny holes; the complete master retains a few short unresolved boundary intervals where reliable neighboring activity does not cover both sides.

The reducer still captures/classifies complete Device history for context, and rebuilds all captured day snapshots in one transaction. Revision storage reuses unaffected slices, but CPU/read/projection work is not fully incremental. Safe cuts currently use unchanged Gaps; continuous histories without such cuts can require broader replacement. Future optimization must preserve semantic safety and source provenance.

The production Web build still reports the existing large-bundle warning. No browser/manual visual acceptance or LAN latency benchmark was performed in this alignment run.

## Local runtime state

The three old OSRM containers were stopped, preserving graph data and all existing database volumes. The application server/Web deployment was not rebuilt or migrated during this implementation run. Restart with `scripts/lifetrail start all` to build the new application and apply migration 0015; existing publications stay available until queued processed-GPS replacements succeed. The temporary acceptance database container is removed after verification.
