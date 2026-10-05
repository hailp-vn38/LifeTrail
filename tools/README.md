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
