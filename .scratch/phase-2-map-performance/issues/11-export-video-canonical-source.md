# 11: Export/video — lazy canonical source (no display fallback)

Status: open
Type: task
Labels: phase-2, web, export
Blocked by: 08, 10

## Context

Spec §16 + Q7(b). Export (GPX, route CSV) và video export cần canonical
geometry. Hiện `web/src/features/daily-map/lib/export-day.ts` đọc
`part.geometry.coordinates` từ Daily View — sau v2 không còn trường này.

## Scope

- Export/GPX/route-CSV và video export lazy fetch **playback endpoint** để lấy
  canonical geometry (on demand, không prefetch).
- **Guardrail: MUST NOT fall back về `display_geometry`.** Nếu playback fetch
  fail → báo lỗi, không xuất geometry đã simplify.
- `export-day.ts` đổi nguồn từ `part.geometry.coordinates` sang playback payload.
- Giữ semantics CSV hiện tại (processed/canonical route, không raw GPS).

## Acceptance

- Export dùng canonical vertices từ playback (khớp canonical length).
- Không nhánh code nào xuất `display_geometry` khi playback fail.
- Test export-day cập nhật theo nguồn mới.
- **Layout Daily page không đổi (§19.0)**: tiến trình/lỗi export non-reflowing,
  giữ nguyên `DailyMapActions` và header grid.

## Out of scope

- Server export endpoint (không thêm).
