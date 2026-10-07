# 02: display_geometry — metric simplification + rounding + budget escalation

Status: resolved
Type: task
Labels: phase-2, server, geometry
Blocked by: None

## Context

Spec §9–§10. Daily View cần một projection nhẹ, visualization-only, tách khỏi
canonical Activity Revision geometry. File đề xuất:
`server/src/processing/display_geometry.rs`.

## Scope

- `display_geometry.rs`: metric Ramer–Douglas–Peucker (RDP) trên khoảng cách
  vuông góc thật (mét), không phải degree.
- Round toạ độ **6 decimals** ở bước cuối cùng (sau simplify), không trước.
- Effective tolerance: bắt đầu baseline `10 m`, escalate deterministic nếu số
  vertex vượt budget `3000`/route part (spec §9.2/§9.3). Trả về
  `effective_tolerance_m` thực dùng.
- Endpoint preservation (§9.4) và boundary preservation (§9.5): giữ first/last
  vertex và ranh giới part; không bịa connector.
- Post-round validation: nếu sau round vẫn vượt budget, set
  `display_vertices_over_budget = true` và trả geometry đã có (không loop vô hạn).
- `display_geometry` là visualization-only: KHÔNG dùng để tính distance/time.

## Acceptance

- Unit test: RDP đúng trên polyline tổng hợp (đường thẳng giảm còn 2 điểm;
  zig-zag giữ đúng đỉnh); round 6 decimals; escalation chọn tolerance nhỏ nhất
  đạt budget; over-budget set cờ đúng.
- Thuần hàm, không I/O; chạy được không cần DB.

## Out of scope

- Wire vào snapshot shape (Ticket 03).
- Web render (Ticket 08).

## Answer

New module `server/src/processing/display_geometry.rs`: deterministic
Ramer–Douglas–Peucker on a local equirectangular metric plane, 6-decimal rounding,
endpoints preserved, duplicate-vertex dedupe. `simplify_parts` applies one shared
per-day tolerance that escalates (`×1.5`, cap `MAX_TOLERANCE_M = 1000 m`) until every
Part fits `LT_DAILY_DISPLAY_MAX_VERTICES` (default 3000 per Route Part), reporting
`tolerance_m` and `over_budget`. Base tolerance `LT_DAILY_DISPLAY_SIMPLIFY_TOLERANCE_M`
(default 10 m). Unit tests cover collinear collapse, endpoint preservation,
rounding, escalation and the over-budget ceiling.
