# 09: Web Daily View — display_geometry refactor

Status: resolved
Type: task
Labels: phase-2, web, map
Blocked by: 08

## Context

Spec §15/§19/§22. `web/src/map/route-parts.ts`, `web/src/components/RouteMap.vue`,
`web/src/features/daily-map/**` hiện consume canonical `part.geometry` +
`vertex_distance_m`/`progress_anchors`. Daily Map phải render `display_geometry`.

## Scope

- `route-parts.ts`: dựng map features từ `display_geometry` (không canonical).
- RouteMap render display geometry; bỏ phụ thuộc `vertex_distance_m`.
- Web yêu cầu `projection_schema_version == 2`, **fail-fast** nếu nhận v1
  (§14) — không silent fallback.
- Bỏ raw GPS khỏi default Daily Map load (§17); raw chỉ qua hành động rõ ràng.
- Giữ distance/time authority từ `distance_m` (không tính từ display).
- Evidence/timeline (`features/timeline/events.ts`) vẫn dùng canonical
  `tripDistanceM` (không đổi sang display).

## Acceptance

- Daily Map render từ `display_geometry`, không đọc canonical `geometry`.
- v1 snapshot → error rõ ràng, không render sai.
- Test hiện có của timeline/evidence không regress.
- **Layout Daily page không đổi (§19.0)**: workspace grid, header grid, vị trí
  toggle Raw GPS, khung loading/empty/error giữ nguyên; `DailyMapPage.test.ts`
  pass.

## Out of scope

- Playback (Ticket 10), export (Ticket 11).

## Answer

`DailyRoutePart` added to the activity model; `tripDisplayParts` replaces
`tripParts`. `route-parts.ts`, `RouteMap.fitRoute`, `daily-layers` and
`visibleTripCoordinates` render `display_geometry`; `tripDistanceM` still sums
`visible_distance_m`. No client-side simplification and no distance from display
coordinates. RouteMap tests updated; the Daily Map renders display geometry with
no controller until playback loads.
