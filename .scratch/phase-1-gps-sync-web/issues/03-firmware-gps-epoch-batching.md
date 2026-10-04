# 03 — Implement GPS Navigation Epoch recording and strict NDJSON batches

Status: open
Type: task
Blocked by: 01

## Goal

Make ESP32 emit one canonical `gps/1` record only from a valid RMC/GGA Navigation Epoch and append it to correctly rotated local Batches.

## Scope

- Parse/checksum-validate RMC and GGA and merge either arrival order by matching GNSS whole-second with a two-second monotonic cache.
- Emit exactly one record per epoch only for valid RMC status/date/time plus matching valid GGA; increment diagnostics for unmatched epochs.
- Serialize strict UTF-8 LF-only NDJSON, including final LF.
- Generate UUIDv4 Batches and rotate before the next record at 300 seconds or 262,144 bytes.
- Keep GPS/storage independent from Wi-Fi readiness.

## Acceptance criteria

- Fixture-driven tests cover RMC-first, GGA-first, mismatch/timeout, duplicate epoch, invalid checksum, invalid RMC status, and fractional timestamp behavior.
- No record is written before valid RMC UTC plus matching GGA.
- Every generated Batch has strictly increasing `ts_ms`, exact final LF, and is under the default byte limit.
- Rotation tests prove both time-triggered and next-line-size-triggered behavior.

## Blocked by

01.
