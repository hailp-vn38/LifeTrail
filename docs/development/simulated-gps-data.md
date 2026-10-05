# Simulated GPS data for Server/Web testing

Use this when no ESP32/GNSS/microSD hardware is available. The simulator generates canonical `gps/1` NDJSON ready files plus immutable manifests, then can upload those exact bytes through the real device-ingestion endpoint.

The default dataset is deterministic:

- 15 minutes at 1 Hz = 900 GPS records.
- 3 batches of 300 records.
- A closed route in central Ho Chi Minh City so MapLibre displays a visible path.
- Owner-local start time 08:00 in `Asia/Ho_Chi_Minh`.
- Valid altitude, speed, course, fix quality, satellites and HDOP.
- Stable UUIDv4 batch IDs for the same date/seed, so running it again exercises replay/idempotency.

## 1. Start the application

From the repository root:

```sh
docker compose -f deploy/docker-compose.yml up --build -d
curl --fail http://localhost:8080/health/ready
```

## 2. Create an Owner and simulated Device

The database is internal to Compose, so run the provisioning CLI in the server container:

```sh
docker compose -f deploy/docker-compose.yml exec server \
  /app/lifetrail-server owner create \
  --display-name "Simulated Owner" \
  --timezone Asia/Ho_Chi_Minh
```

Copy the returned Owner UUID, then create the Device:

```sh
docker compose -f deploy/docker-compose.yml exec server \
  /app/lifetrail-server device create \
  --owner-id <owner-uuid> \
  --name "Simulated GPS"
```

Save the returned `id` and one-time `token`.

## 3. Generate and upload simulated GPS

Choose the Owner-local date you want to display:

```sh
PYTHONPATH=tools python3 tools/simulate_gps.py \
  --date 2026-10-05 \
  --endpoint http://localhost:8080/api/v1/device/batches \
  --token '<lt_dev_token>'
```

The command writes exact ready/manifest pairs under `tools/fixtures/simulated-route/` and uploads all batches through the production ingestion contract.

Expected first run: every response has `status: "committed"` and `duplicate: false`.

Run the exact same command again. Because batch IDs and bytes are deterministic, expected responses are `duplicate: true`; GPS row count must not increase.

## 4. Test the API

```sh
curl --fail http://localhost:8080/api/v1/devices
curl --fail \
  http://localhost:8080/api/v1/devices/<device-uuid>/days/2026-10-05
```

The Daily View should contain a non-null route/start/end and a non-zero point count, duration and distance.

For the empty state, choose another date, for example:

```sh
curl --fail \
  http://localhost:8080/api/v1/devices/<device-uuid>/days/2026-10-04
```

That valid Device/day should return HTTP 200 with the Phase 1 zero/null Daily View.

## 5. Test the built Web

Open:

```text
http://localhost:8080/devices/<device-uuid>/day/2026-10-05
```

You should see the simulated route, start/end markers and summary. Then change the URL to `2026-10-04` to verify the empty state.

## Generate-only mode

To inspect fixtures without a running server:

```sh
PYTHONPATH=tools python3 tools/simulate_gps.py --date 2026-10-05
PYTHONPATH=tools python3 -m unittest tools/tests/test_simulate_gps.py
```

Useful overrides:

```sh
PYTHONPATH=tools python3 tools/simulate_gps.py \
  --date 2026-10-05 \
  --seconds 1800 \
  --batch-records 240 \
  --seed another-dataset
```

Changing `--seed` creates a different deterministic set of batch IDs for the same date.
