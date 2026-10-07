# 12: Server + Web instrumentation

Status: resolved
Type: task
Labels: phase-2, perf, observability
Blocked by: 03, 04, 09, 10

## Context

Spec §26/§27/§30/§36. Không có số đo thì không chứng minh được optimization.

## Scope

- **Server** (§26/§27/§30): log `daily_snapshot_json_bytes`,
  `display_vertices` per route part, `display_vertices_over_budget`,
  `effective_tolerance_m`, `playback_json_bytes`, latency Daily View vs Playback.
- **Web** (§36): log `initial_daily_map_payload_bytes`,
  `display_vertices_loaded`, `playback_fetched`, `playback_latency_ms`.
- Instrumentation không làm tăng payload (log-only, không thêm field vào
  response ngoài contract).

## Acceptance

- Log đủ field để Ticket 13 đo before/after.
- Không field debug nào rò ra wire ngoài OpenAPI.

## Out of scope

- Benchmark execution (Ticket 13).

## Answer

Server: `reprojection::log_display_metrics` (display vertices, over-budget,
effective tolerance, snapshot bytes) plus latency/size debug logs in the Daily
view and Playback handlers. Web: `map/map-metrics.ts` logs `daily_map_loaded`
(display vertices, Daily payload bytes) from RouteMap and `playback_loaded`
(parts, bytes, latency) from DailyMapCanvas. Log-only; nothing on the wire.
