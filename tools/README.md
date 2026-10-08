# Host-side tools

Host-side acceptance tools consume [`../protocol/`](../protocol/README.md) rather than defining another format.

- `batch_acceptance.py inspect` checks an immutable ready body against its firmware manifest, including exact bytes, SHA-256 and strict framing.
- `batch_acceptance.py commit-and-replay` posts a new body twice and requires commit then idempotent replay.
- `batch_acceptance.py replay` checks an already committed body remains an idempotent replay on both posts.

```sh
PYTHONPATH=tools python3 -m unittest tools/tests/test_batch_acceptance.py
```

`fixtures/lan-acceptance/` contains a ready/manifest pair for inspection. See [`../docs/development/phase-1-lan-acceptance.md`](../docs/development/phase-1-lan-acceptance.md) for the complete run.

`verify_single_origin.py` confirms a running Compose server returns the same built SPA entrypoint and nonempty compiled asset for `/` and a canonical Daily View URL.

## Simulated GPS route

When ESP32/GNSS hardware is unavailable, generate deterministic `gps/1` data and optionally upload it through the real ingestion endpoint:

```sh
PYTHONPATH=tools python3 tools/simulate_gps.py \
  --date 2026-10-05 \
  --endpoint http://localhost:8080/api/v1/device/batches \
  --token '<lt_dev_token>'
```

The default dataset contains 900 records over 15 minutes, split into three valid ready/manifest Batches. Running the same command again reuses the same UUIDv4 batch IDs and exact bytes, so the server should return `duplicate: true` instead of inserting more GPS points.

See [`../docs/development/simulated-gps-data.md`](../docs/development/simulated-gps-data.md) for provisioning, API, replay, empty-state and built-Web checks.

## Realistic GPS dataset

`generate_realistic_day.py` creates the road-aligned, walking-day fixture used to test raw ingestion, Daily View and route playback. Its checked-in baseline is in `fixtures/realistic-human-day/`; see [`../docs/development/realistic-gps-data.md`](../docs/development/realistic-gps-data.md) for the scenario and upload procedure.

## Phase 2 acceptance

Use the deterministic [Phase 2 fixture suite](lifetrail-phase2-testdata/README.md) and `server/tests/phase2_acceptance.rs`. These tests run ingestion, processing and Daily View publication without network routing.

For Daily display geometry and lazy playback with the current firmware, use
[the map performance dataset](fixtures/phase2-web-server-map-performance/README.md).
`generate_map_performance_data.py` produces firmware-adaptive and dense comparator
archives from the same offline 1 Hz observations through the real C policy.

## Future routing tools (outside Phase 2)

The following tools and their Python modules are retained for a future optional routing phase: `prepare_osrm.py`, `download_osrm_vietnam.sh`, `generate_routed_scenario.py`, `generate_realistic_osrm_day.py`, `lifetrail_batch/osrm_client.py` and associated scenario/provenance files. They do not participate in Phase 2 runtime, worker configuration or acceptance. Existing graph data is retained; the canonical Compose topology has no routing services.
