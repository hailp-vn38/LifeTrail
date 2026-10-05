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

`scripts/lifetrail` là điểm vào để quản lý Compose local. Ba service runtime:

```text
web       Nginx + Vue SPA đã build, giữ host port 8080, proxy /api/* về server
server    Rust/Axum API, chỉ chạy trên Docker network nội bộ (server:8080)
postgres  PostgreSQL/PostGIS, chỉ chạy trên Docker network nội bộ
```

```sh
scripts/lifetrail start all
scripts/lifetrail status
scripts/lifetrail test
scripts/lifetrail logs web
scripts/lifetrail stop all
```

Web và API cùng origin tại `http://localhost:8080` (Nginx phục vụ `/` và proxy `/api/*`). Lệnh `stop` không xóa volume PostgreSQL.

## Trạng thái

Ticket 02 thiết lập Rust/Axum, PostgreSQL/PostGIS, CLI provisioning và topology local/LAN. GPS collection, storage, đồng bộ Batch và Daily View sẽ được triển khai ở các ticket sau.
