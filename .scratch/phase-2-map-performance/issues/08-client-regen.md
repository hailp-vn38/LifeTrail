# 08: Client regen — API generated types + query hooks

Status: open
Type: task
Labels: phase-2, web, codegen
Blocked by: 01

## Context

`web/src/api/generated/lifetrail-v1.d.ts` sinh từ
`protocol/openapi/lifetrail-v1.yaml`. Contract v2 (Ticket 01) đổi `RoutePart`
Daily View và thêm playback path → phải regen trước khi web consume.

## Scope

- Regen generated types từ OpenAPI v2.
- Cập nhật `web/src/api/query-keys.ts` + queries cho playback endpoint.
- `RoutePart` Daily View type: có `display_geometry`, `display_tolerance_m`,
  `display_vertices_over_budget`, `visible_distance_m`, `distance_m`; không có
  `vertex_distance_m`/`progress_anchors`/canonical `geometry`.
- Playback query hook (dùng chung cho play + export/video).
- Raw GPS type giữ nhưng không default.

## Acceptance

- `npm run typecheck` (hoặc tương đương) pass với contract v2.
- Query keys playback có key theo (device, date, manifest_version).

## Out of scope

- UI consume (Ticket 09/10).
