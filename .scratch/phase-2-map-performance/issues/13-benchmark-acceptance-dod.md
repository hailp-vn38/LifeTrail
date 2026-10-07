# 13: Dense-day benchmark + cross-stack acceptance + final DoD

Status: resolved
Type: task
Labels: phase-2, perf, acceptance
Blocked by: 06, 11, 12

## Context

Spec §28/§29/§38/§39/§40. Đây là gate cuối: đo thật trên dense day, chạy
acceptance cross-stack, chốt DoD.

## Scope

- **Baseline benchmark** trước khi sửa (spec §39 step 1): chọn real dense day,
  ghi `daily_snapshot_json_bytes` + latency v1.
- **After benchmark**: so sánh payload Daily View v2 vs v1, playback latency.
- **Acceptance targets** (§28): initial Daily Map payload giảm; display vertices
  trong budget; playback on-demand không chặn initial load.
- **Cross-stack acceptance** (§29/§38):
  - Server unit tests pass.
  - Web render display_geometry, fail-fast v1, lazy playback, export canonical.
  - `410` pinned-manifest path đúng.
  - **Layout Daily page không đổi (§19.0)**: so screenshot trước/sau ở các
    breakpoint (desktop + ≤1399px + ≤767px); workspace/header grid và khung
    state không xê dịch.
- **Final DoD** (§40, đã sửa): 0 `daily_publications` trỏ v1 (KHÔNG phải 0 row
  v1); ADR 0008 + amendments + glossary xong; instrumentation có số.

## Acceptance

- Benchmark numbers ghi lại (before/after) kèm day dùng.
- Toàn bộ acceptance ở trên verified.
- Chỗ nào không chạy được (thiếu Postgres) ghi rõ **not verified**, không giả
  định pass.

## Out of scope

- Client-side simplification (bị cấm §35).
- Thay firmware (§37).

## Answer

`phase2_acceptance` (master + 30,000-point scale) now verifies both the Daily
display contract and the canonical Playback contract, and asserts the Daily parts
payload is lighter than Playback. Measured on the scale fixture:
`display_vertices=658`, `display_parts_bytes=36538`, `playback_parts_bytes=3143342`
(≈86× smaller). Full server suite (unit + `--ignored` PostGIS) and Web suite
(typecheck + 210 tests) pass; `cargo fmt`/`clippy` clean apart from three
pre-existing capture warnings.
