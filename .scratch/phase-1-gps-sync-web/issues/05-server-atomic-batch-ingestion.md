# 05 — Implement atomic device batch ingestion and replay semantics

Status: resolved
Type: task
Blocked by: 01, 02

## Goal

Implement `POST /api/v1/device/batches` so validated NDJSON becomes immutable Raw GPS atomically and deterministic retries never duplicate points.

## Scope

- Authenticate Bearer token by SHA-256 digest; device identity never comes from request path.
- Enforce 1 MiB body limit; stream/count/hash/frame/parse/validate all body bytes before beginning a DB transaction.
- Verify schema, required headers, byte length, lowercase SHA-256, record count, strict in-batch timestamps, and GPS ranges.
- Persist `ingest_batches` metadata and raw `gps_points` with server-created PostGIS geometry in one transaction.
- Implement same-ID/same-hash replay (`200`, `duplicate: true`), same-ID/different-hash conflict (`409`), full-batch rollback, stable error envelope, and request-ID logs.

## Acceptance criteria

- Integration tests run on real PostgreSQL/PostGIS and prove no partial rows on every validation failure.
- Replay cannot insert another GPS point; hash conflict cannot return a replay ACK.
- Cross-Batch same-timestamp points remain permitted while within-Batch duplicates/decreases return 422.
- All documented `401/403/409/413/422/429/5xx` client-visible error classes have test coverage where owned by the endpoint.

## Blocked by

01, 02.

## Answer

Implemented `POST /api/v1/device/batches` with Bearer digest authentication, strict byte-verified `gps/1` NDJSON validation, one-transaction persistence of immutable Batch metadata and Raw GPS, PostGIS geometry generation, idempotent replay, and hash-conflict handling. Real PostGIS integration coverage proves successful commit, replay, conflict, validation rollback, transaction-failure rollback, size limit, canonical error envelopes, and request IDs.
