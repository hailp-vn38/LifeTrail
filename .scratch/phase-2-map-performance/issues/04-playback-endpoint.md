# 04: Playback endpoint — canonical on-demand projection

Status: open
Type: task
Labels: phase-2, server, api
Blocked by: 01

## Context

Spec §7. Playback là consumer canonical duy nhất (playback, GPX, route CSV,
video export). Nó đọc lại immutable Activity Revisions qua manifest và chạy
`project_part_for_playback()`. **Không phụ thuộc snapshot v2** (Q14) → đi trước
Ticket 03.

File đề xuất: `server/src/processing/playback.rs`.

## Scope

- Handler `GET /api/v1/devices/{deviceId}/days/{date}/playback?manifest_version=`.
- Unpinned: resolve current publication → canonical playback payload, `200`.
- Pinned + manifest tồn tại: `200` payload của manifest đó.
- Pinned + manifest không tồn tại: `410 Gone`.
- KHÔNG trả `409` cho unpinned (bỏ semantics `publication_changed`).
- Payload: canonical route geometry + temporal progress (không simplify, không
  round, không display), `manifest_version`, `projection_schema_version`.
- `project_part_for_playback()`: projection thuần từ canonical Route Part +
  Activity Revision progress; không đọc `display_geometry`.
- Không thêm export endpoint (spec §7/§16).

## Acceptance

- Unit test `project_part_for_playback`: giữ nguyên canonical vertices; progress
  time-anchored đúng; không round.
- Contract 200/410 khớp Ticket 01.
- Pinned manifest không tồn tại → 410, không 409/404 mơ hồ.

## Out of scope

- Web fetch/lifecycle (Ticket 09).
- Export consumers (Ticket 10).
