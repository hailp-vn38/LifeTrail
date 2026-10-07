# 03: Projection schema v2 — lightweight Daily Snapshot shape

Status: open
Type: task
Labels: phase-2, server, projection
Blocked by: 01, 02

## Context

Spec §13/§14/§20. `projection_schema_version` hiện `= 1`. Snapshot shape phải
chuyển sang v2: chỉ phát display projection, bỏ payload canonical nặng khỏi
initial Daily Map load.

File liên quan: `server/src/processing/reprojection.rs`,
`server/src/daily_view/projection.rs`, `server/src/processing/route_parts.rs`,
`server/src/processing/progress_projection.rs`.

## Scope

- Bump `projection_schema_version = 2` trong snapshot provenance (shape + field
  chốt ở Ticket 01).
- Snapshot v2 `RoutePart`:
  - `display_geometry` (từ Ticket 02),
  - `display_tolerance_m`,
  - `display_vertices_over_budget`,
  - `visible_distance_m` (progress trên canonical, dùng để clip/playback),
  - `distance_m` (canonical distance — nguồn sự thật, không suy từ display).
- Bỏ khỏi snapshot body v2: `vertex_distance_m`, `progress_anchors`, canonical
  `geometry` của Route Part.
- **Không** mutate body snapshot v1 đã có (immutable). Snapshot mới sinh là v2.
- `progress_projection.rs`: refactor để canonical progress vẫn được suy từ
  Activity Revision geometry (không từ display). Giữ semantics clip part.
- Provenance mô tả rõ nguồn processed GPS (giữ theo ADR 0007).

## Acceptance

- Snapshot mới có `projection_schema_version = 2` và đúng shape trên.
- `visible_distance_m`/`distance_m` tính từ canonical geometry (test so với
  canonical length, không lệch vì simplify).
- `progress_projection` test cũ vẫn pass sau refactor.
- Snapshot v1 cũ đọc lại vẫn nguyên vẹn (không bị ghi đè).

## Out of scope

- Backfill historical (Ticket 06).
- Playback endpoint (Ticket 04) — độc lập, đi trước.
