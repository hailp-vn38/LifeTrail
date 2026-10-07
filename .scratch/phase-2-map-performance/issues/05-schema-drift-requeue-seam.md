# 05: Schema-drift projection-only requeue seam

Status: resolved
Type: task
Labels: phase-2, server, processing
Blocked by: 03

## Context

Spec §14/§31/§41. Khi `projection_schema_version` đổi (v1→v2), các publication
cũ cần được reproject. Nhưng snapshot body là immutable, nên không được rewrite
tại chỗ: phải **projection-only requeue** — enqueue một job chỉ chạy lại
projection, tạo snapshot v2 mới, không chạy lại matcher/GPS processing.

Seam này phải **tách khỏi timezone-generation seam** hiện có (Q16). File liên
quan: `server/src/processing/activation.rs`, `capture.rs`, `queue.rs`, `mod.rs`,
migration `0015_processed_gps.sql`.

## Scope

- Thêm khái niệm projection-only job: chạy lại reprojection cho một day mà
  không re-run matcher, không đọc lại raw batch (không tạo Activity Revision mới).
- Idempotent: enqueue nhiều lần cho cùng (device, date, target schema) không
  tạo nhiều snapshot v2 trùng; dùng unique/guard.
- Tách khỏi `timezone_generation` seam: schema-drift trigger riêng, không đụng
  logic advance timezone generation.
- Publication chỉ switch atomically sau khi snapshot v2 sẵn sàng (Ticket 06).

## Acceptance

- Requeue projection-only không tạo Activity Revision mới (assert count).
- Enqueue 2 lần idempotent (1 snapshot v2).
- Timezone-generation flow không bị ảnh hưởng (test hiện có pass).

## Out of scope

- Mass backfill + cutover (Ticket 06).

## Answer

`projection_requeues` (migration `0016`) is the durable, idempotent
(device, day, target schema) guard. `capture.rs` marks a day projection-only when
the active publication predates `DAILY_PROJECTION_SCHEMA_VERSION` (2) and Raw is
clean; the Daily view flags `display_projection_stale` and lazily calls
`queue_projection`. Reprojection reuses the active immutable manifest and never
re-derives Raw or bumps generations. Verified by `playback.rs`
(`schema_drift_is_flagged_and_requeued_once`).
