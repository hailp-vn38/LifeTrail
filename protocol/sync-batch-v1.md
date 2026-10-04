# Sync Batch `sync/1`

A **Batch** is an immutable Device-owned `gps/1` NDJSON file with a device-generated canonical UUIDv4. The upload endpoint is:

```http
POST /api/v1/device/batches
Authorization: Bearer <device_token>
Content-Type: application/x-ndjson
```

The token resolves the Device; a request does not send `device_id`.

## Durable manifest

Before upload, the Device writes an immutable manifest containing:

- `batch_id`: canonical UUIDv4.
- `schema`: `gps/1`.
- `byte_length`: exact NDJSON body byte count.
- `sha256`: SHA-256 of exactly those body bytes.
- `record_count`: number of NDJSON records.
- `first_ts_ms` and `last_ts_ms`: first and last record timestamps.

Only a verified `.ndjson.ready` file and matching manifest are upload eligible. A retry sends the original bytes without reserializing them.

## Required request headers

| Header | Required value | Verification rule |
| --- | --- | --- |
| `X-LifeTrail-Batch-Id` | Canonical UUIDv4 | Must equal the Batch manifest ID. |
| `X-LifeTrail-Schema` | `gps/1` | Must name this contract version. |
| `X-LifeTrail-Content-SHA256` | 64 lowercase hexadecimal characters | Server hashes the raw received body bytes and requires exact equality before replay acknowledgement. |
| `X-LifeTrail-Byte-Length` | Unsigned decimal integer | Server counts actual raw received bytes and requires exact equality. |
| `X-LifeTrail-Record-Count` | Unsigned decimal integer | Server parses strict NDJSON and requires exact equality. |

`Content-Length` may be transport metadata but is never the canonical integrity truth. The server also derives first/last timestamps itself, accepts at most 1 MiB, and verifies framing and every `gps/1` record before opening its transaction.

## Atomic replay behavior

For a newly observed `(device_id, batch_id)`, the server atomically inserts the ingest Batch and all Raw GPS records, then commits before responding. It permits timestamp overlap across Batches.

| Condition | Response |
| --- | --- |
| New ID and valid body | `200`, `duplicate: false` |
| Same Device/Batch ID and same verified hash | `200`, `duplicate: true`; inserts no points |
| Same Device/Batch ID and different verified hash | `409 batch_conflict` |
| Invalid framing, record, hash, byte count, or record count | `422 invalid_batch` |
| Body over 1 MiB | `413` |
| Transaction failure | `5xx`, with no partial persistence |

Success uses this envelope:

```json
{
  "batch_id": "f273162b-31a4-42db-a0a0-32f342e72a27",
  "status": "committed",
  "record_count": 300,
  "duplicate": false
}
```

The Device marks a Batch acknowledged only when HTTP `200` has matching `batch_id`, `status: "committed"`, and manifest `record_count`.

## Single error envelope

Every API-v1 error has exactly this outer shape; `code` is stable machine-readable behavior, while clients must not branch on `message`.

```json
{
  "error": {
    "code": "invalid_batch",
    "message": "Batch contains an invalid GPS Record.",
    "request_id": "req_01...",
    "details": { "line": 317, "field": "lat", "reason": "out_of_range" }
  }
}
```

`details` is optional. `request_id` correlates the response with server logs. The reusable OpenAPI representation is [`openapi/lifetrail-v1.yaml`](openapi/lifetrail-v1.yaml).

## Retry classification

Sync is single-flight and selects ready Batches by `first_ts_ms ASC, batch_id ASC`.

- `409`, `413`, and `422`: quarantine only this Batch and continue with the next one.
- `401` and `403`: enter `AUTH_BLOCKED`, retain all ready data, and resume only after provisioning/token change or explicit retry/reboot policy.
- Network/TLS/timeout, `429`, and `5xx`: retain ready data, stop the pass, and retry with exponential backoff plus jitter. Honor `Retry-After` when present.
