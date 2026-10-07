# GPS persistence acceptance — Phase 2

Implemented on `integration/phase-2-timeline-osrm`, against the Rev 2 [spec](gps-persistence-optimization.md) and [ADR-0001](../adr/0001-firmware-gps-persistence-policy.md).

The host filter compiles the actual firmware policy. Simulations select the original observations by accepted timestamp; they do not reimplement the policy or replace source coordinates. The server acceptance test uploads baseline and filtered evidence through real ingestion, runs the processing worker, and reads published Daily Views and Raw GPS views from an isolated PostgreSQL/PostGIS database. It checks Timeline activities, GPS Gap/Evidence Hole semantics, Trip continuity, Route Part progress/playback anchors, Stop boundaries and point-to-polyline error.

| Scenario | Baseline records | Persisted records | Maximum geometry error | Maximum Stop boundary error |
| --- | ---: | ---: | ---: | ---: |
| Stationary 8 hours | 28,800 | 254 | 0 m | 0 s |
| Walking 1.4 m/s | 900 | 301 | < 0.01 m | 0 s |
| Driving 15 m/s | 900 | 451 | < 0.01 m | 0 s |
| Driving with 90° turn | 900 | 451 | 10.61 m | 0 s |
| Stop → movement → long stop → movement | 1,800 | 245 | 10 m | 2 s |
| 20-second short pause | 900 | 447 | < 0.01 m | 0 s |
| Eight-hour no-fix interval | 1,200 valid epochs outside gap | 25 | 0 m | 0 s |

The lost-heartbeat variant removes one accepted heartbeat downstream: the resulting 240-second interval stays below the server's 300-second GPS Gap threshold and preserves the Stop. No-fix instead publishes one real Gap; no record timestamp is inside the eight-hour missing-fix interval.

The actual filesystem/queue/Batch writer test for an eight-hour stationary stream measures **254 append, 332 flush and 328 fsync operations**, including manifest durability. Relative to the previous per-epoch write/fsync baseline of 28,800, record reduction is **99.12%** and fsync reduction is **98.86%**. Unit coverage also checks heading bypass at one second, angle wrap-around, slow-speed heading suppression, jump heartbeat exclusion, jitter, timestamp regression, candidate stability reset, immediate transitions/backfill and drop-oldest queue overflow.

The server uses `observation_gap_s=300`, `short_failure_max_s=10`, `stop_min_duration_s=180`, and `stop_radius_m=30`. Sparse usable observations create no false Evidence Holes: short-failure bridging applies only to rejected observations in `processing/continuity.rs`. No server threshold changes or protocol metadata were required.

## Reproduce

```sh
cmake -S firmware/esp32/host_tests -B /tmp/lifetrail-host-tests
cmake --build /tmp/lifetrail-host-tests
ctest --test-dir /tmp/lifetrail-host-tests --output-on-failure
PYTHONPATH=tools python3 -m unittest discover -s tools/tests
python3 tools/gps_persistence_acceptance.py --output /tmp/lifetrail-gps-fixtures
# LT_TEST_DATABASE_URL must point to a disposable PostgreSQL/PostGIS database.
# Integration suites truncate their test database; never use the live database.
cargo test --manifest-path server/Cargo.toml --test firmware_persistence -- --ignored --nocapture
# Full server regression suite on the same disposable database:
cargo test --manifest-path server/Cargo.toml -- --include-ignored --test-threads=1
```

`tools/simulate_gps.py --firmware-persistence ...` runs the same compiled policy and writes adaptive 60s/300s batches. Its default remains the 1 Hz baseline for comparisons. `LT_GPS_POLICY_FILTER` may point to an already-built host filter. Firmware builds with ESP-IDF 6.1; the reproducible container used here is `espressif/idf@sha256:8ac794c57fd4cac246cb8d2ada4002fa26337ac7df683047b5b83743dbedb6b7`.

These measurements are deterministic simulation and host-filesystem results. Physical GPS reception, SD removal/power cycling and a hardware soak test require the Device; no hardware flash or field measurement was performed.

Final verification passes all five firmware host suites, all 11 Python tests, and the full server suite including ignored integration/scale tests. ESP-IDF 6.1 `reconfigure build size` succeeds after the watchdog/config changes (image size 296,714 bytes).

## Review — Standards

The independent standards review identified missing task watchdog coverage and unnamed policy/runtime thresholds. Both are resolved: GPS/storage tasks register, reset and deregister a configurable 30-second progress watchdog; thresholds and scheduling limits have explicit names. Python adapters now use named motion and persistence-reason values. Responsibility-based module separation passed review.

## Review — Spec

The independent spec review identified premature wall-clock Batch rotation during backlog draining and a shutdown freshness guard that discarded the last real observation during no-fix. Both are resolved: prefetch prevents idle rotation while backlog remains, and shutdown unconditionally finishes the policy without fabricating observations. Regression tests cover delayed Batch rotation, finishing after no-fix and slow-walking recovery without state oscillation. Host suites, Python suites and the real server acceptance suite pass after these fixes.
