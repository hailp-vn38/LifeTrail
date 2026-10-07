# Phase 2 Map Payload & Rendering Performance — Decisions map

Spec gốc: [`docs/web/lifetrail-phase2-web-server-map-performance.md`](../../docs/web/lifetrail-phase2-web-server-map-performance.md) (rev 2).

Session này chốt lại decision tree (Q1–Q18) vì spec rev 2 có ba điểm không còn
khớp với kiến trúc đã chốt: contract `409/410`, migration "không còn snapshot v1",
và thứ tự rollout. Docs-first: sửa artifact nguồn rồi mới code.

## Decisions-so-far

```text
Q1  display_geometry giữ như rev 2 (metric RDP, round 6 decimals, budget escalate).
    kind discriminator KHÔNG serialize trên wire — tách API contract khỏi canonical nội bộ.

Q2  Daily View chỉ phát display projection; canonical geometry/progress qua playback.

Q3  Playback endpoint dedicated: GET /api/v1/devices/{deviceId}/days/{date}/playback.

Q4  Tolerance baseline 10 m, escalation deterministic, round sau cùng.

Q5  Vertex budget 3000/route part; vượt budget → set display_vertices_over_budget.

Q6  Web lazy fetch playback, không fallback display_geometry.

Q7  B — Export/video/GPX/route-CSV lấy canonical từ playback endpoint on demand.
    Guardrail: export MUST NOT fall back to display_geometry.
    (CSV hiện là processed/canonical route export — đã verify export-day.ts.)

Q8  A — ADR 0008 mới + amend 0004/0005/0007. Flag duplicate 0001, không renumber.

Q9  3 glossary concepts: Display Geometry, Playback Payload, Effective Tolerance.
    Không thêm Projection Schema Version. Amend Route Part / Daily View.

Q10 A — single coordinated v2 cutover.
    Acceptance = no PUBLISHED v1 (không còn daily_publications trỏ v1),
    KHÔNG phải "no v1 DB row". Historical immutable v1 rows may remain.

Q11 13 ticket, per architectural slice (không gộp server 01–05).

Q12 a — không serialize kind: "display" trên wire.

Q13 ticket order chốt (xem Thứ tự thực thi).

Q14 Playback endpoint KHÔNG phụ thuộc snapshot v2 → đảo playback lên trước
    projection v2. Playback đọc canonical qua manifest + project_part_for_playback().

Q15 pinned manifest tồn tại → 200; pinned manifest mất → 410. Web fail-fast.

Q16 projection-only requeue seam, idempotent; tách khỏi timezone-generation seam.

Q17 C1 — giữ ticket 03 và 05 tách (failure domain/invariant khác nhau).
    C2 — dedicated /playback endpoint.
    Không `409`; contract playback là pinned 200/410.

Q18 Ràng buộc bắt buộc: update web KHÔNG được phá layout UI Daily page.
    Giữ workspace grid + header grid + vị trí Raw GPS toggle + khung
    loading/empty/error. State mới của playback/export phải non-reflowing.
    Ghi ở spec §19.0, acceptance ticket 09/10/11/13.
```

## Thứ tự thực thi (Q13)

```text
01 → 02 → 04 → 03 → 05 → 06   (server)
→ 08 → 09 → 10 → 11            (web)
→ 12 → 13                      (instrumentation + acceptance)
```

## Ticket index

```text
01  OpenAPI contract v2 (display_geometry, playback, no kind, no vertex_distance/progress_anchors)
02  display_geometry.rs (metric RDP + rounding + deterministic budget escalation + post-round validation)
03  Projection v2 (lightweight snapshot shape + provenance)
04  Playback endpoint (dedicated /playback, pinned manifest 200/410, project_part_for_playback)
05  Schema-drift projection-only requeue seam (idempotent, tách timezone seam)
06  Historical backfill + atomic publication cutover (verify published v2)
07  ADR 0008 + amend 0004/0005/0007 + CONTEXT glossary (docs-first, resolved)
08  Client regen (API generated types/query hooks)
09  Web Daily View display_geometry refactor
10  Web lazy playback + controller lifecycle/race guards
11  Export/video lazy canonical source (no display fallback)
12  Server + Web instrumentation
13  Dense-day benchmark + cross-stack acceptance + final DoD
```

## Status log

- 2026-10-07: chốt decision tree Q1–Q18, tạo ticket plan (13 ticket).
- 2026-10-07: sửa spec §8/§11/§14/§16/§32/§39/§40/§41; viết ADR 0008 +
  amend 0004/0005/0007; amend CONTEXT.md glossary (Display Geometry,
  Playback Payload, Effective Tolerance; amend Daily View, Route Part).
- 2026-10-07: thêm ràng buộc Q18 (không phá layout Daily page) vào spec §19.0 +
  acceptance ticket 09/10/11/13.

## Ghi chú ponytail-review

- Bỏ `spec.md` (nguồn thứ ba trùng); `map.md` là nguồn duy nhất cho decision +
  ticket index, spec gốc giữ nguyên vai trò canonical.
- Bỏ block `## Verification` boilerplate ("Implemented: yes") khỏi ticket chưa
  làm; chỉ ticket resolved (07) giữ verification thật.
- Bỏ dòng `Blocks:` (đảo ngược được từ `Blocked by:`) và đổi
  `ready-for-agent` → `open` cho khớp convention issue-tracker.
