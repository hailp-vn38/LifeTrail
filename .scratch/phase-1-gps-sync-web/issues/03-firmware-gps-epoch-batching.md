# 03 — Implement GPS Navigation Epoch recording and strict NDJSON batches

Status: resolved
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

## Comments

- Test seams: `lifetrail_gps` receives NMEA lines and emits only canonical GPS Records plus diagnostics; `lifetrail_storage` receives those Records and emits batch-open, LF-NDJSON append, and rotation events. The seams follow the existing firmware ownership map and `gps/1` contract, keeping Wi-Fi outside both modules.

## Answer

- Added the `lifetrail_gps` Navigation Epoch collector backed by pinned `minmea`: it validates RMC/GGA, accepts either arrival order for two seconds, emits once per matched epoch, and keeps diagnostics for checksum, invalid RMC, unmatched, and duplicate epochs.
- Added the `lifetrail_storage` batch writer: it creates UUIDv4 IDs through an injected entropy source, produces fixed-buffer LF-only `gps/1` NDJSON, enforces strict timestamp ordering, and rotates before the 300-second or 262,144-byte limits.
- Fixture-driven host tests cover every stated parser and rotation case. Host CMake/CTest passed; ESP-IDF is not installed in this environment, so a native IDF build remains unverified.
