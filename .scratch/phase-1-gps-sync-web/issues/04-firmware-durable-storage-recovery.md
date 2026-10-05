# 04 — Implement manifest-backed batch durability, recovery, and SD capacity policy

Status: resolved
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

## Comments

Implemented manifest-backed durable batch storage behind `lifetrail_storage`:

- Batch files now transition from `.ndjson.open` through an fsynced immutable
  manifest to verified `.ndjson.ready`, and successful acknowledgement moves
  only the data file to `.ndjson.acked`.
- Boot recovery verifies canonical `gps/1` bytes and manifest metadata, trims
  only an unterminated final `.open` tail, rebuilds a missing ready manifest,
  and moves malformed, mismatched, or orphaned artifacts into Quarantine.
- Storage health reports recovery, rebuild, Quarantine, cleanup, free-space,
  low-space, and pause state. Cleanup is restricted to ACKed files; the
  16/4/8 MiB hysteresis and 24-hour ACK retention policy are enforced.

Host CMake/CTest passed both `gps_epoch_batch_tests` and
`batch_store_tests`. Native ESP-IDF/SD-card execution remains unverified in
this environment.
