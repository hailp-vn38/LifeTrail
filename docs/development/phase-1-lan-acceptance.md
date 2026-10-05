# Phase 1 — LAN acceptance run

Run this guide from the repository root. It separates repeatable host gates from the physical Device evidence that only a real ESP32, GNSS receiver, microSD, and LAN can provide.

## Automated host gates

```sh
PYTHONPATH=tools python3 -m unittest tools/tests/test_batch_acceptance.py
python3 tools/batch_acceptance.py inspect \
  --body tools/fixtures/lan-acceptance/f273162b-31a4-42db-a0a0-32f342e72a27.ndjson.ready \
  --manifest tools/fixtures/lan-acceptance/f273162b-31a4-42db-a0a0-32f342e72a27.manifest
cd server && cargo test --test static_web
cd ../web && npm test && npm run typecheck && npm run build
```

`inspect` checks the firmware manifest, SHA-256, byte length, strict LF NDJSON framing, record count, and first/last `ts_ms` against the exact bytes. It never reserializes the body.

## One-origin LAN topology

Choose the host's reachable LAN address and retain it for the run; `LAN_IP` must be a static address or DHCP reservation. The Device uses this HTTP origin in NVS `api_url`.

```sh
export LAN_IP=192.168.1.50
docker compose -f deploy/docker-compose.yml up --build -d
curl --fail "http://${LAN_IP}:8080/health/ready"
python3 tools/verify_single_origin.py \
  --origin "http://${LAN_IP}:8080" \
  --device-id f273162b-31a4-42db-a0a0-32f342e72a27 \
  --date 2026-10-04
```

The Compose image builds `web/` and the Rust server serves it from the same origin. Repeat the verifier from a second LAN machine. Do not use the Vite development address for this gate.

## Physical Device run

Create the Owner and Device, then serial-provision Wi-Fi, `api_url` as `http://${LAN_IP}:8080`, and the printed Device token. Do not commit the token.

```sh
export LT_DATABASE_URL=postgres://lifetrail:lifetrail_dev_only@localhost:5432/lifetrail
cd server
cargo run -- owner create --display-name "LifeTrail Owner" --timezone Asia/Ho_Chi_Minh
cargo run -- device create --owner-id <owner-uuid> --name "GPS Recorder"
```

1. Disable Wi-Fi and record outdoors for at least ten minutes. Confirm at least two `.ndjson.ready` and matching `.manifest` pairs on microSD.
2. Power-cycle while recording. Confirm committed records remain, the open Batch recovers, and recording resumes.
3. Copy one ready pair without changing it and inspect it:

   ```sh
   python3 tools/batch_acceptance.py inspect --body <batch>.ndjson.ready --manifest <batch>.manifest
   ```

4. Re-enable LAN Wi-Fi. Retain Device logs showing oldest-first commit of at least two Batches, their IDs, and server request IDs.

## Replay and Daily View evidence

Copy a committed pair unchanged and replay its original bytes twice. Both responses must be `duplicate: true`.

```sh
python3 tools/batch_acceptance.py replay \
  --endpoint "http://${LAN_IP}:8080/api/v1/device/batches" \
  --token "<device-token>" \
  --body <committed-batch>.ndjson.ready \
  --manifest <committed-batch>.manifest
docker compose -f deploy/docker-compose.yml exec -T postgres \
  psql -U lifetrail -d lifetrail -c "SELECT count(*) FROM gps_points WHERE device_id = '<device-uuid>';"
```

Record this count before and after replay; it must not increase. From a second LAN browser, open `http://${LAN_IP}:8080/devices/<device-uuid>/day/<owner-local-date>` and capture nonempty Route, start/end, summary, timestamps, duration, and distance. Then select an independently chosen no-data date and capture the zero/null state.

## Evidence boundary

Automated tests prove host-tool behavior and the server static canonical URL. They do not prove outdoor GNSS, power-loss durability, ESP32 Wi-Fi/SD streaming, or reachability from a second LAN machine. Resolve the ticket only when the physical run and its logs, hashes, counts, and screenshots are retained.
