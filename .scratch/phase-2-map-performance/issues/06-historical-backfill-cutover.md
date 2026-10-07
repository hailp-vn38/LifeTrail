# 06: Historical backfill + atomic publication cutover

Status: resolved
Type: task
Labels: phase-2, server, migration
Blocked by: 05

## Context

Spec §14/§39/§40 (rev 2 sửa). Cutover v1→v2 là single coordinated cutover,
nhưng **không** xoá/rewrite snapshot v1 (immutable). Acceptance đổi từ
"không còn snapshot v1" thành **"không còn `daily_publications` trỏ tới v1"**.

## Scope

- Enqueue projection-only reproject cho mọi day đang có publication active.
- Mỗi day: tạo snapshot v2 mới → switch publication atomically sang v2.
- Không update JSON body snapshot v1; không delete v1 rows.
- Verify: mọi publication hiện hành trỏ `projection_schema_version = 2`.
- Xử lý lỗi/retry cho day fail (không kẹt half-cutover).

## Acceptance

- Query: 0 `daily_publications` active trỏ snapshot v1.
- Snapshot v1 cũ vẫn tồn tại, đọc nguyên vẹn.
- Day fail được retry, không publish v1/v2 lẫn lộn.

## Out of scope

- Web compatibility (web fail-fast v1 — Ticket 08/09).

## Answer

`processing::backfill_projection_schema` enqueues projection-only reprojection for
every published day below the current schema, idempotently (re-running accepts
only still-stale, unseen requests). Operator entry point:
`lifetrail-server reproject-schema` in `main.rs`. Verified by
`backfill_enqueues_only_stale_publications`.
