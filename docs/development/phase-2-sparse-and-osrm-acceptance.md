# Phase 2 tickets 01–02: sparse publication and routed fixtures

The server now schedules background work in the same transaction as a new Batch commit. `serve` runs a durable worker with a 30-second reclaimable lease and fencing token. The worker captures Raw input, processing target and Owner timezone in a repeatable-read transaction, stages immutable candidates, then checks those identities under a Device control lock before swapping publication pointers. No routing dependency runs in this slice.

Only days containing zero or one Raw Record can publish `processed` / `insufficient` snapshots here. Their geometry and activity arrays are empty, while counts and observation timestamps include excluded fixes. Dense days remain Raw and report `activity_processing_not_available`; a previously published sparse snapshot remains available as stale after late input makes that day dense. A timezone change hides incompatible publications. Artifacts are retained, including rejected candidates.

Use the existing provisioning CLI and `LT_DATABASE_URL`. To explicitly queue an empty day or reprocess a sparse day:

```sh
cargo run --manifest-path server/Cargo.toml -- process-day \
  --device-id '<device UUID>' --date 2026-10-06
```

This command drains available queued work and prints that day's status; an already running job completes in the server worker. Read `/api/v1/devices/<UUID>/days/<date>/status` for background state, freshness and published revision. The Daily View includes the same status. Web refresh updates it; automatic polling belongs to ticket 07.

## Internal OSRM and simulator

Obtain a bounded PBF covering the scenario. The demonstration uses Monaco from `https://download.geofabrik.de/europe/monaco-latest.osm.pbf`. A latest extract may change; the retained fixture records its exact SHA-256 and normalized Route result. Reproducing exact geometry requires that extract and the pinned image, rather than a later download.

```sh
curl -fL https://download.geofabrik.de/europe/monaco-latest.osm.pbf \
  -o /tmp/lifetrail-monaco.osm.pbf
python3 tools/prepare_osrm.py --pbf /tmp/lifetrail-monaco.osm.pbf \
  --dataset-version monaco-test
mkdir -p runtime/osrm-demo
LT_OSRM_DATASET_VERSION=monaco-test docker compose -f deploy/docker-compose.yml \
  --profile osrm up -d osrm-car osrm-bike osrm-foot

docker compose -f deploy/docker-compose.yml --profile osrm-tools run --rm simulator \
  --scenario tools/scenarios/monaco-car.json \
  --metadata data/osrm/monaco-test/car/metadata.json \
  --osrm-url http://osrm-car:5000 --output runtime/osrm-demo/car
```

Preparation refuses an existing profile directory: use a new dataset version after a failed preparation or when changing the extract. Each profile has its own `extract → partition → customize` MLD artifacts and Lua script. The image is pinned by version and digest in the preparation tool and Compose. OSRM has no published host port; only operator containers access it. Route is simulator-only. Nearest is available on the internal OSRM service for diagnostics and has no product endpoint.

The scenario records `[longitude, latitude]` waypoints, `profile` (`car`, `bike`, `foot`), `started_at` with an offset matching its IANA `timezone`, positive `duration_s` / `interval_s`, and `seed`. Geometry comes from Route; historical timing comes from the scenario, independently of OSRM's estimated duration. The simulator writes ready bodies, firmware-compatible manifests and `route-evidence.json`. Same inputs and dependency response yield identical body bytes and Batch IDs.

For bike or foot, copy the scenario and change `profile`; use the corresponding metadata and `http://osrm-bike:5000` or `http://osrm-foot:5000`. To generate, commit and replay in one command, append:

```sh
  --endpoint http://server:8080/api/v1/device/batches --token '<device token>'
```

The server must be running in Compose on the same network. For host ingestion, use `tools/batch_acceptance.py commit-and-replay` with the generated ready/manifest pair (see `--help`). Open `/devices/<device UUID>/day/2026-10-05` in the existing Web application. The 25-record demonstration stays Raw until later activity slices exist.

## Observed acceptance, 2026-10-06

- Real OSRM `v6.0.0`, pinned digest `729461bcc9ae9e6aafa92c0f93db9b060a32e85d5e72092c01ae4a4a9f1eb564`; all three profiles prepared and served separately, with no host ports.
- Each profile generated 25 valid Records over 120 logical seconds. Retained bodies/manifests/scenarios and complete response provenance live in `tools/fixtures/osrm-monaco/{car,bike,foot}/`.
- Car Batch `35146d33-0d91-4932-8c3d-2079bf0fd3a8` uploaded to a real server/PostGIS via HTTP: first response `duplicate: false`, second `duplicate: true`. Raw Daily View returned 25 points, a LineString and Europe/Monaco observation timestamps 08:00–08:02.
- Built Web opened the canonical Device/day path in Chromium. The existing Daily Map displayed Raw GPS, 25 records, 1.10 km and two minutes. Local screenshot evidence is `runtime/phase2-acceptance/raw-map.png`; tokens and provisioned Device identity remain only in ignored runtime files.
- `server/tests/sparse_processing.rs` runs the HTTP → real worker → PostGIS → Daily View/status seam, including replay/rejection generation checks, excluded single-record and empty publication, late dense deferral, immutable history and timezone invalidation.
- `tools/tests/test_routed_scenario.py` uses a deterministic HTTP Route response to verify sampling, framing, hashes, seed stability and historical timing. Web tests verify queued and processed insufficient-evidence presentation.

Fixtures contain synthetic GPS Records. They are evidence of Route generation and ingestion, not ground truth for later activity classification or Map Match quality.
