# LifeTrail

LifeTrail ghi nhận lịch sử vị trí từ Device hoạt động offline và hiển thị Daily View trong mạng cục bộ. Phase 1 tập trung vào GPS thô, Batch bền vững và đồng bộ LAN; không gồm xác thực Web, TLS hay xử lý chuyến đi.

## Bản đồ repository

- [Bối cảnh domain](CONTEXT.md)
- [Đặc tả Phase 1](.scratch/phase-1-gps-sync-web/spec.md)
- [Kiến trúc project](docs/project/codebase.md)
- [Kiến trúc firmware ESP32](docs/firmware/esp32.md)
- [Kiến trúc Rust server](docs/server/architecture.md)
- [Kiến trúc Web](docs/web/architecture.md)
- [Protocol contracts và fixtures](protocol/README.md)

Các boundary thực thi là `firmware/esp32/`, `server/` và `web/`. Chúng không import source của nhau; mọi wire contract dùng chung thuộc `protocol/`.

## Chạy local

`scripts/lifetrail` là điểm vào để quản lý Compose local. `server` build và phục vụ luôn ứng dụng `web`, nên `web` là alias của `server`.

```sh
scripts/lifetrail start server
scripts/lifetrail status
scripts/lifetrail test
scripts/lifetrail logs server
scripts/lifetrail stop all
```

Web và API cùng chạy tại `http://localhost:8080`. Lệnh `stop` không xóa volume PostgreSQL.

## Trạng thái

Ticket 02 thiết lập Rust/Axum, PostgreSQL/PostGIS, CLI provisioning và topology local/LAN. GPS collection, storage, đồng bộ Batch và Daily View sẽ được triển khai ở các ticket sau.
