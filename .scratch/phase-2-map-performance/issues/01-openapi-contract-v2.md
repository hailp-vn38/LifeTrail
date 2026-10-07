# 01: OpenAPI contract v2 — Daily display + Playback

Status: open
Type: task
Labels: phase-2, api, contract
Blocked by: None

## Context

`protocol/openapi/lifetrail-v1.yaml` hiện mô tả Daily Snapshot v1: `RoutePart`
serialize canonical geometry + `vertex_distance_m` + `progress_anchors`, và raw
GPS qua query `?view=raw`. Spec
`docs/web/lifetrail-phase2-web-server-map-performance.md` §6.1/§7/§8/§13 chốt
contract mới. Đây là first code slice vì nó khóa shape cho mọi consumer.

## Scope

- `RoutePart` trong Daily View: chỉ còn `id`, `kind` (server-internal, KHÔNG
  serialize `kind: "display"`), `display_geometry`, `display_tolerance_m`,
  `display_vertices_over_budget`, `visible_distance_m`, `distance_m`.
- Bỏ khỏi Daily View response: `vertex_distance_m`, `progress_anchors`,
  canonical `geometry` của Route Part.
- Thêm schema + path playback:
  `GET /api/v1/devices/{deviceId}/days/{date}/playback`
  với query optional `?manifest_version=<uuid>`.
  - Unpinned: `200` trả current canonical playback payload.
  - Pinned, manifest tồn tại: `200`.
  - Pinned, manifest không tồn tại: `410`.
  - KHÔNG có `409 publication_changed` cho unpinned playback (§8 rev 2 bỏ).
- Playback payload: canonical route geometry + temporal progress (không phải
  `display_geometry`), kèm `manifest_version`, `projection_schema_version`.
- Provenance Daily Snapshot: `projection_schema_version = 2`.
- Raw GPS (`?view=raw`) không còn là default path của Daily Map; giữ endpoint
  nhưng tài liệu hóa là optional/debug (spec §17).

## Acceptance

- OpenAPI validate sạch; `RoutePart` Daily View không còn `vertex_distance_m`
  /`progress_anchors`/canonical `geometry`.
- Playback path + 200/410 response có schema đầy đủ.
- Không còn mô tả `409` cho playback unpinned.
- Generated client regen không lỗi (Ticket 07).

## Out of scope

- Server implementation của display simplification (Ticket 02).
- Playback handler (Ticket 04).
