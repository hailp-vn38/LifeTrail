# 04 — Implement manifest-backed batch durability, recovery, and SD capacity policy

Status: open
Type: task
Blocked by: 03

## Goal

Make microSD storage preserve unacknowledged GPS history across reboot/power loss and deterministically distinguish recoverable tails from Quarantine.

## Scope

- Implement `.ndjson.open`, immutable manifest, `.ndjson.ready`, `.acked`, and Quarantine lifecycle.
- Produce manifest only after closed/fsynced data and calculate exact-byte SHA-256, byte length, record count, first/last timestamps.
- Implement boot recovery matrix from the Phase 1 spec, including tail truncation only to last valid LF and Quarantine for malformed middle content/hash mismatch.
- Implement 24-hour ACK retention and SD thresholds: 16 MiB low, 4 MiB pause, 8 MiB resume; delete oldest ACKed only.
- Expose storage/health counters and never auto-delete `.open`, `.ready`, or Quarantine.

## Acceptance criteria

- Power-loss fixtures cover each state boundary between file close, manifest write/rename, and ready rename.
- A partial tail recovers without losing preceding complete records; malformed middle data reaches Quarantine.
- Only verified ready+manifest pairs become sync eligible.
- Low-space cleanup cannot delete unacknowledged data; recording pauses/resumes with hysteresis.

## Blocked by

03.
