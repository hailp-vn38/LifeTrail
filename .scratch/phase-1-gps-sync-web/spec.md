# LifeTrail Phase 1 — Offline GPS → LAN Sync → Daily Web Route

## Status

Approved design baseline. This spec overrides earlier proposed architecture notes where they conflict within Phase 1 scope.

## Outcome

An ESP32 records GNSS GPS while offline to microSD, survives power loss, synchronizes verified batches to a local Rust server over HTTP when Wi-Fi is available, and a Vue web app renders the selected Device's daily route.

Phase 1 is local/dev-first. It deliberately excludes Web authentication, TLS, public deployment, captive-portal/BLE provisioning, trip/stop processing, smoothing, outlier removal, and media.

## Domain and scope

- One Owner exists per Phase 1 deployment; the `users → devices` ownership model remains.
- `users` contains `id`, `display_name`, `timezone`, `created_at`, and `updated_at`; it has no required auth fields.
- A Device belongs to one Owner. Its `token_digest` identifies it during device ingestion.
- The Owner's IANA timezone defines a Daily View's calendar boundaries. All stored measurement timestamps are UTC.
- The Web read API is local-network only and unauthenticated in Phase 1.

## Development and acceptance topology

Developer loop:

```text
Vue/Vite localhost:5173 --proxy /api--> Rust/Axum localhost:8080 --> PostgreSQL/PostGIS
ESP32 ---------------------------------> http://<LAN-IP>:8080/api/v1/device/batches
```

Acceptance topology:

```text
Docker Compose: PostgreSQL + Rust server + built Vue static app
http://<LAN-IP>:8080/            # Web
http://<LAN-IP>:8080/api/v1/...  # API
```

PostgreSQL is internal to Docker Compose. The device's `api_url` is an arbitrary HTTP URL stored in NVS; acceptance uses a LAN IP with static address or DHCP reservation. No mDNS/service discovery is required.

## Provisioning and identity

An admin/CLI creates an Owner and Device. The CLI generates one random 256-bit device token, formatted `lt_dev_<base64url-without-padding>`, displays plaintext once, and stores only `SHA-256(token)` as unique `devices.token_digest` on the server.

Serial provisioning writes `wifi_ssid`, `wifi_password`, `api_url`, and `device_token` to NVS. It may expose behavior equivalent to setting Wi-Fi/API/token, showing provisioning status, and explicit factory reset. Wi-Fi disconnect, server failure, and auth failure must not erase provisioning data. GPS acquisition and SD recording do not depend on Wi-Fi provisioning or connection.

The canonical firmware upload endpoint stays:

```http
POST /api/v1/device/batches
Authorization: Bearer <device_token>
```

The token resolves the Device; `device_id` is not supplied in the upload path.

## GPS collection and `gps/1`

The device creates a `GpsRecord` only after a valid Navigation Epoch:

- RMC checksum is valid, RMC status is `A`, and RMC UTC date/time is valid.
- GGA checksum is valid and has the same GNSS whole-second (`YYYY-MM-DD + hh:mm:ss`, date from RMC).
- RMC and GGA may arrive in either order and are cached for at most two seconds of monotonic arrival time.
- Exactly one record is emitted per epoch. No matching pair within two seconds increments `navigation_epoch_unmatched` and is discarded. GGA data is never carried forward.

`ts_ms` is RMC UTC time, including receiver-provided fractional milliseconds when available. No GPS Record is persisted before a valid GNSS epoch; system/NTP time is diagnostic-only in Phase 1.

Each NDJSON object uses schema `gps/1`:

```json
{
  "ts_ms": 1791102702000,
  "lat": 10.781234,
  "lon": 106.692345,
  "alt_m": 12.4,
  "speed_mps": 4.2,
  "course_deg": 127.5,
  "fix_quality": 1,
  "satellites": 8,
  "hdop": 1.3
}
```

Required fields: `ts_ms`, `lat`, `lon`, `fix_quality`, `satellites`. Optional/nullable: `alt_m`, `speed_mps`, `course_deg`, `hdop`.

Server validation: `ts_ms > 0`; latitude in `[-90,90]`; longitude in `[-180,180]`; non-negative `fix_quality`, `satellites`, `speed_mps`, and `hdop`; `course_deg` in `[0,360)` when present. Poor fix/HDOP, drift, speed anomalies, and jumps are accepted Raw GPS, not invalid ingestion.

Body framing is strict UTF-8 NDJSON: one JSON object per physical line, LF (`0x0A`) only, final LF required, no blank lines, CRLF, arrays, primitives, empty body, or multiple objects per line.

## Batch durability and recovery

Each Batch uses a device-generated UUIDv4 and is immutable once ready. Filesystem lifecycle:

```text
<id>.ndjson.open
  -> flush + fsync + close + validate final line + calculate metadata
  -> manifest.tmp + fsync + rename to <id>.manifest
  -> rename data file to <id>.ndjson.ready
  -> valid semantic ACK -> .acked
```

Manifest is immutable and contains: `batch_id`, `schema`, `byte_length`, exact-byte `sha256`, `record_count`, `first_ts_ms`, and `last_ts_ms`. SHA-256 is over exactly the bytes sent in the HTTP body; retries send the original file bytes without reserializing.

Only a verified `.ndjson.ready` plus matching `.manifest` is upload eligible. Boot recovery follows these rules:

- `.open` without manifest: recover/truncate only an incomplete final tail, then finalize again.
- `.open` plus valid manifest: verify and promote to `.ready`.
- `.ready` plus valid manifest: upload eligible.
- `.ready` without manifest: rebuild only after data verification.
- missing data, hash mismatch, malformed middle line, or irrecoverable manifest/data: move to Quarantine.

Quarantine is never auto-deleted. A partial final line may be truncated back to the last valid LF; a malformed committed middle line is not silently skipped.

Batch rotation occurs before writing `next_record` when a non-empty batch would reach 300 seconds from first record or appending `next_line` would exceed 262,144 bytes. Defaults are configurable, not `gps/1` invariants. Server hard request maximum is 1 MiB.

After semantic ACK, `.acked` may be retained for 24 hours. Low-space cleanup deletes oldest `.acked` first and never deletes `.open`, `.ready`, or Quarantine. Defaults: low warning/cleanup below 16 MiB, pause recording below 4 MiB, resume only above 8 MiB. With no deletable space left, pause new GpsRecord creation but retain GPS diagnostics and report storage health.

## Sync request, replay, and errors

An upload has these required headers:

```http
Content-Type: application/x-ndjson
Authorization: Bearer <device_token>
X-LifeTrail-Batch-Id: <canonical UUIDv4>
X-LifeTrail-Schema: gps/1
X-LifeTrail-Content-SHA256: <64 lowercase hex chars>
X-LifeTrail-Byte-Length: <unsigned decimal>
X-LifeTrail-Record-Count: <unsigned decimal>
```

`Content-Length` is optional transport metadata, never canonical integrity truth. The server counts actual bytes, hashes raw body bytes, parses record count, and derives first/last timestamps itself.

Within one Batch, `ts_ms` must strictly increase. Duplicate/decreasing timestamps make the entire Batch `422 invalid_batch`. Across Batches, overlap is permitted; no `(device_id, ts_ms)` uniqueness constraint exists.

The server receives, size-limits, hashes, frames, parses, validates, and retains the at-most-1-MiB validated Batch in memory before opening a transaction. It then compares incoming hash with an existing `(device_id,batch_id)` row, inserts `ingest_batches` and raw `gps_points` atomically when new, and commits before ACK. Hash verification happens before an idempotent replay is acknowledged.

Same device/batch ID plus same hash returns `200` without inserting points; same ID plus different hash returns `409 batch_conflict`. Invalid content/hash/count/length returns `422`; body over hard limit returns `413`; DB insert failure rolls back and returns `5xx`.

Success response:

```json
{
  "batch_id": "f273162b-31a4-42db-a0a0-32f342e72a27",
  "status": "committed",
  "record_count": 300,
  "duplicate": false
}
```

Firmware marks a Batch ACKed only for HTTP 200 with matching `batch_id`, `status: "committed"`, and manifest `record_count`. A mismatch remains `.ready` and increments `protocol_error`; it is not immediately quarantined.

Every error uses:

```json
{
  "error": {
    "code": "invalid_batch",
    "message": "Batch contains an invalid GPS Record.",
    "request_id": "req_...",
    "details": { "line": 317, "field": "lat", "reason": "out_of_range" }
  }
}
```

`code` is stable machine-readable API-v1 behavior; `message` is not for branching; `details` is optional. Request IDs must correlate with server logs.

Sync is single-flight and selects ready Batches by `first_ts_ms ASC, batch_id ASC`. `409`, `413`, and `422` quarantine only the Batch and continue. `401/403` enter `AUTH_BLOCKED`, retain all ready data, and resume only after provisioning/token change or explicit retry/reboot policy. Network/TLS/timeout, `429`, and `5xx` retain ready data, stop the pass, and retry with exponential backoff and jitter; honor `Retry-After` when present.

## Server storage and Daily View

Raw GPS is immutable. Server generates geometry from validated `lon/lat`; it never trusts geometry from the device. `gps_points` uses a monotonic primary key and a `(device_id, recorded_at)` index; it intentionally allows same-timestamp rows across Batches.

Daily View endpoint:

```http
GET /api/v1/devices/:deviceId/days/:date
```

The server resolves `[local midnight, next local midnight)` from the Device Owner's IANA timezone; it must not add a fixed 24 hours. A valid Device with no points returns 200, not 404. Nonexistent Device returns 404.

The Phase 1 route is a query-time raw projection: select points where `fix_quality > 0`, ordered by `recorded_at ASC, id ASC`. No processing worker, smoothing, map matching, jump removal, HDOP threshold, simplification, trips, stops, or timeline events are included. `distance_m` is `f64` total Haversine distance of adjacent route points; zero or one point gives zero.

```json
{
  "device_id": "019...",
  "date": "2026-10-04",
  "timezone": "Asia/Ho_Chi_Minh",
  "processing_state": "raw",
  "summary": {
    "point_count": 13428,
    "distance_m": 12840.527,
    "duration_s": 13320,
    "first_fix_at": "2026-10-04T00:45:12Z",
    "last_fix_at": "2026-10-04T04:27:12Z"
  },
  "route": { "type": "Feature", "properties": {}, "geometry": { "type": "LineString", "coordinates": [] } },
  "start": { "type": "Feature", "properties": { "recorded_at": "2026-10-04T00:45:12Z" }, "geometry": { "type": "Point", "coordinates": [106.692345, 10.781234] } },
  "end": { "type": "Feature", "properties": { "recorded_at": "2026-10-04T04:27:12Z" }, "geometry": { "type": "Point", "coordinates": [106.701234, 10.790123] } }
}
```

For a zero-point Daily View, `route`, `start`, `end`, `first_fix_at`, and `last_fix_at` are null; counts, distance, and duration are zero. `raw_data_complete` is explicitly absent: the server cannot know about offline unsynced Batches.

## Web

`/` lists Devices. `/devices/:deviceId/day/:date` is the canonical Daily View URL. Selecting a Device defaults to today in that Device Owner's timezone; it does not implicitly redirect to the latest day with data.

Web has no login/auth in Phase 1. It uses the OpenAPI-generated typed client and renders loading, error, empty, and route states. MapLibre GL JS renders route/start/end from the Daily View GeoJSON; it never creates a DOM marker for every raw point. MapTiler Cloud is the default configurable basemap (`VITE_MAP_STYLE_URL`, `VITE_MAPTILER_KEY`); the browser key is origin-restricted, not secret. The style URL remains replaceable for future self-hosting.

## Acceptance test

1. Create Owner and Device; provision Wi-Fi, local HTTP `api_url`, and token over serial. Device appears in `GET /api/v1/devices`.
2. With Wi-Fi unavailable, record outdoors for at least 10 minutes, create valid RMC/GGA records, and rotate into ready Batches.
3. Power-cycle while recording; recover `.open`, preserve committed lines, and resume logging.
4. Enable LAN Wi-Fi; sync at least two Batches oldest-first. Server verifies manifest-equivalent hash/count and persists rows.
5. Replay a committed Batch unchanged; receive `200` with `duplicate: true` and no additional `gps_points`.
6. Open canonical Daily View URL in built static Web app; route, start/end, count, timestamps, duration, and distance render.
7. Open a no-data date; receive 200 zero/null Daily View and render empty state.

## Explicit non-goals

- Web or user authentication, sessions, passwords, TLS/ACME, public deployment, and access-control hardening.
- Captive portal, BLE setup, discovery, token rotation, expiry, scopes, or revocation history.
- NTP-derived GPS timestamps, variable-rate GNSS protocol, media, trips, stops, timeline events, PWA/offline Web, SSE, and processing workers.
- GPS smoothing, map matching, outlier detection/removal, HDOP thresholding, route simplification, or cross-Batch deduplication.
