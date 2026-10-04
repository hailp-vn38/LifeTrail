# ESP32 firmware

ESP-IDF project boundary for the Device. Components will own GPS, durable storage, Wi-Fi, sync and health independently; shared wire semantics come only from [`../../protocol/`](../../protocol/README.md).

The empty `app_main` is a build entrypoint only. It intentionally does not collect GPS, persist data, or perform network work.
