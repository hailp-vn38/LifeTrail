# GPS Record `gps/1`

`gps/1` is the canonical schema for one immutable **GPS Record** in a Batch. Firmware serializes it as one UTF-8 JSON object per line; the server validates it before any Raw GPS persistence.

## Navigation Epoch provenance

A GPS Record exists only after one valid **Navigation Epoch**:

1. RMC has a valid checksum, status `A`, and valid UTC date/time.
2. GGA has a valid checksum and the same GNSS whole-second (`YYYY-MM-DD` plus `hh:mm:ss`, with date from RMC).
3. RMC and GGA may arrive in either order but are cached for at most two seconds of monotonic arrival time.
4. Exactly one GPS Record is emitted for the matched epoch. An unmatched epoch is discarded; GGA fields are never carried into another epoch.

`ts_ms` is the RMC UTC timestamp, including receiver-provided milliseconds when present. System or NTP time is diagnostic-only and must not create a GPS Record.

## Object schema

| Field | JSON type | Required | Phase 1 validation and meaning |
| --- | --- | --- | --- |
| `ts_ms` | integer | yes | UTC Unix milliseconds; greater than `0`. |
| `lat` | number | yes | Latitude in `[-90, 90]`. |
| `lon` | number | yes | Longitude in `[-180, 180]`. |
| `alt_m` | number or `null` | no | Altitude in metres when the receiver provides it. |
| `speed_mps` | number or `null` | no | Speed in metres/second; non-negative when present. |
| `course_deg` | number or `null` | no | Course in degrees; `[0, 360)` when present. |
| `fix_quality` | integer | yes | Receiver fix quality; non-negative. |
| `satellites` | integer | yes | Number of satellites; non-negative. |
| `hdop` | number or `null` | no | HDOP; non-negative when present. |

No other field names are part of `gps/1`. Missing required fields, values of the wrong JSON type, and unrecognized fields reject the whole Batch. A JSON value must have the object shape above; arrays and primitives are invalid records.

## Batch rules

Within one Batch, `ts_ms` must be strictly increasing. A duplicate or decreasing timestamp rejects the entire Batch as `invalid_batch`; overlap between different Batches is allowed.

Poor fix quality, weak HDOP, drift, speed anomalies and jumps are accepted **Raw GPS**. They are not structural/range violations and therefore must not reject ingestion in Phase 1.

## UTF-8 NDJSON framing

The request body is strict UTF-8 NDJSON:

- Each physical line contains exactly one JSON object.
- Line delimiter is LF byte `0x0A` only; the body must end with LF.
- Blank lines, CRLF, a missing final LF, malformed JSON, arrays, primitives, empty bodies and multiple objects on one physical line are invalid.
- The `X-LifeTrail-Content-SHA256` digest is calculated over these exact body bytes, including every LF.

See [`fixtures/`](fixtures/README.md) for acceptance examples.
