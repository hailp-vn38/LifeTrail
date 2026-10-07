# ESP32 firmware

ESP-IDF 6.1 project for the Device. `app_main` starts the recording composition; wire semantics come from [`../../protocol/`](../../protocol/README.md).

`lifetrail_recorder` wires UART acquisition to the Navigation Epoch collector, persistence policy, bounded storage queue and buffered SD writer. The receiver remains configured at 1 Hz. The GPS task performs no SD I/O; the storage task handles append, flush/fsync, adaptive Batch rotation and recovery. Wi-Fi/upload remain independent components.

Run `idf.py menuconfig` to set the board wiring. Defaults target classic ESP32: UART1 RX GPIO16 at 9600 baud; SDSPI MOSI23/MISO19/CLK18/CS5; SD mount `/sdcard`, records `/sdcard/gps`. SD cards are never automatically formatted. Policy thresholds, queue length and durability cadence are configurable under **LifeTrail recording**. `sdkconfig.defaults` enables UUID filenames and disables per-write FatFS fsync.

```sh
idf.py build size
cmake -S host_tests -B /tmp/lifetrail-host-tests
cmake --build /tmp/lifetrail-host-tests
ctest --test-dir /tmp/lifetrail-host-tests --output-on-failure
```

Call `lt_recorder_shutdown()` before an explicit SD lifecycle change or controlled restart; the restart hook also attempts bounded queue drain and durable rotation. A false return means the storage operation did not complete. GPS recording continues through SD delays/overflow, with health available from `lt_recorder_health()`.

See the [persistence spec](../../docs/firmware/gps-persistence-optimization.md), [firmware ADR](../../docs/adr/0001-firmware-gps-persistence-policy.md) and [acceptance report](../../docs/firmware/gps-persistence-acceptance.md).
