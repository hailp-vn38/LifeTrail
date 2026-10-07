# 10: Web lazy playback + controller lifecycle/race guards

Status: open
Type: task
Labels: phase-2, web, playback
Blocked by: 08, 09

## Context

Spec §7/§18/§21/§22/§23/§32. Playback cần canonical geometry (không có trong
Daily View v2) → lazy fetch playback endpoint. Có race khi user chuyển ngày
giữa chừng.

## Scope

- Lazy fetch `/playback` khi user bấm Play (không prefetch lúc load Daily View).
- Lifecycle/race guards (§32): pause + clear playback cache + refetch khi đổi
  ngày; huỷ request cũ; không apply payload của ngày cũ.
- Pin `manifest_version` khi playback bắt đầu; `410` → fail-fast UI (ngày bị
  thay đổi).
- Controller cũ dispose trước khi replace controller mới (§23).
- Cache playback theo (device, date, manifest_version).

## Acceptance

- Đổi ngày khi đang playback → không render nhầm geometry ngày cũ.
- `410` → UI báo ngày đã thay đổi, không crash.
- Không fetch playback khi chỉ mở Daily Map (lazy).
- **Layout Daily page không đổi (§19.0)**: state loading/error của playback
  non-reflowing (inline/overlay), không xô lệch workspace/header grid.

## Out of scope

- Export/video (Ticket 11).
