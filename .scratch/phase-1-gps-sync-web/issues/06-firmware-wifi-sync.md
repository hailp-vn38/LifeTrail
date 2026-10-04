# 06 — Implement serial provisioning and resilient single-flight LAN sync

Status: open
Type: task
Blocked by: 04, 05

## Goal

Provision a Device over serial and synchronize verified ready Batches from SD to the local HTTP endpoint without compromising offline recording.

## Scope

- Implement NVS provisioning behavior for Wi-Fi, `api_url`, and device token, including status and explicit factory reset.
- Use `POST /api/v1/device/batches`, exact manifest bytes/headers, and stream upload from SD.
- Upload single-flight ordered by `first_ts_ms`, then `batch_id`.
- Verify semantic ACK before `.ready → .acked`.
- Implement `AUTH_BLOCKED`, Batch-specific quarantine statuses, transient retry/backoff/jitter, and `Retry-After` support.

## Acceptance criteria

- Wi-Fi/server/token failures never erase NVS or stop GPS/storage.
- Valid first commit and replay ACK transition to `.acked`; malformed/mismatched ACK does not.
- `409/413/422` quarantine and continue; `401/403` block sync; network/429/5xx retain ready data and back off.
- Tests simulate timeout after server commit, Wi-Fi loss during body streaming, and reboot before ACK.

## Blocked by

04, 05.
