# 07 — Implement timezone-aware raw Daily View and OpenAPI artifact

Status: open
Type: task
Blocked by: 02, 05

## Goal

Expose a stable, typed read model for one Device/Owner-local day without introducing a processing worker.

## Scope

- Implement `GET /api/v1/devices/:deviceId/days/:date` using Owner IANA timezone boundaries, including DST-safe next-midnight conversion.
- Select route points with `fix_quality > 0`, ordered by `recorded_at ASC, id ASC`.
- Produce GeoJSON LineString/start/end, `processing_state: "raw"`, summary point count/duration/first/last/distance, and the zero/null empty response.
- Calculate `distance_m` as f64 pairwise Haversine distance; do not use degree geometry as meters.
- Generate and commit OpenAPI v1 used by the Web typed client.

## Acceptance criteria

- A nonexistent Device returns 404; a valid Device/no-data day returns 200 with the specified zero/null response.
- Tests cover timezone day boundaries and deterministic same-timestamp cross-Batch ordering.
- Distance tests use an explicit tolerance and cover 0, 1, and multiple route points.
- Response has no `raw_data_complete`, trips, stops, events, smoothing, or derived processing claim.

## Blocked by

02, 05.
