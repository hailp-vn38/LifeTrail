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

## OSRM-routed scenarios

`prepare_osrm.py` builds three separate, versioned MLD datasets with a pinned OSRM image. `generate_routed_scenario.py` samples Route geometry in seeded historical time and optionally uploads/replays valid Batches. See [Phase 2 acceptance](../docs/development/phase-2-sparse-and-osrm-acceptance.md) for internal Compose commands, scenario fields and retained three-profile fixtures.

### Vietnam dataset

Run from the repository root:

```sh
bash tools/download_osrm_vietnam.sh
python3 tools/prepare_osrm.py --pbf data/osrm/source/vietnam-latest.osm.pbf \
  --flat --dataset-name vietnam --dataset-version vietnam-<date>-<sha256-prefix> \
  --source-url https://download.geofabrik.de/asia/vietnam-latest.osm.pbf
docker compose --env-file deploy/osrm-vietnam.env -f deploy/docker-compose.yml \
  --profile osrm up -d osrm-car osrm-bike osrm-foot
```

Replace the version placeholder with the download date and source SHA-256 prefix.
The downloader validates Geofabrik's MD5 and prints SHA-256. Preparation uses
`car.lua`, `bicycle.lua` and `foot.lua` independently and runs extract, partition
and customize sequentially with two threads. The source remains in
`data/osrm/source/`; generated `vietnam.osrm*` and `metadata.json` live under
`data/osrm/{car,bike,foot}/`. Temporary profile PBF copies are removed after success.
Existing profile directories are refused; preserve previous artifacts before
preparing a replacement. `data/` is ignored by Git; retain dataset provenance
under `tools/datasets/`, without committing PBF or OSRM artifacts.

The 2026-10-06 full-Vietnam build used roughly 12 GB peak RAM during extraction.
On a 16 GB workstation running other applications, additional swap was needed;
preprocess profiles sequentially and allow several GB of disk per profile.

Compose defaults to these Vietnam paths; the Vietnam env file selects them
explicitly. To use the retained Monaco graphs, set
`LT_OSRM_DATASET_VERSION=monaco-test` and `LT_OSRM_DATASET_NAME=test-region`
when starting OSRM. All three routing services expose port 5000 only on the
Compose network.
# Realistic routed days

`generate_realistic_osrm_day.py` produces a complete, seeded road-network day from
the timed Home/Coffee/Office/Restaurant/Supermarket scenario. Movement time is
chosen by the scenario (with acceleration, deceleration and speed variation), not
by OSRM's duration estimate. It emits normal immutable `gps/1` Batches and
manifests, plus factual `scenario.json`, `ground-truth.json` and
`routing-evidence.json`. Ground truth records inputs and injected observation
conditions; it deliberately does not predict Trips, Stops, Gaps or matching output.

Copy both `*.example.json` files, replace the dataset identities with the bounded
extract actually used, then run from the repository root:

```sh
python3 tools/generate_realistic_osrm_day.py \
  --scenario tools/scenarios/realistic-osrm-day.json \
  --routing-versions /path/to/routing-versions.json \
  --osrm-urls /path/to/osrm-urls.json \
  --output /tmp/lifetrail-realistic-day \
  --endpoint http://localhost:8080/api/v1/device/batches \
  --token "$LIFETRAIL_DEVICE_TOKEN"
```

The optional endpoint/token uploads the exact generated Batches, then replays
them. Open the existing Raw Daily Map for the scenario's local date to inspect the
unchanged Raw observations. `routing-evidence.json` freezes every normalized Route
response and request URL along with engine/dataset provenance; regenerate with the
same dependency evidence and inputs to compare bytes and Batch IDs.

See [`scenarios/realistic-osrm-variants.md`](scenarios/realistic-osrm-variants.md)
for the focused multimode, cross-midnight, Evidence Hole, late-upload and matcher
failure scenarios.
