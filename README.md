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

`scripts/lifetrail` là điểm vào để quản lý Compose local. Các service runtime:

```text
web       Nginx + Vue SPA đã build, giữ host port 8080, proxy /api/* về server
server    Rust/Axum API, chỉ chạy trên Docker network nội bộ (server:8080)
postgres  PostgreSQL/PostGIS, chỉ chạy trên Docker network nội bộ
osrm-car / osrm-bike / osrm-foot  OSRM cho ba profile, chỉ chạy nội bộ
```

```sh
scripts/lifetrail start all
scripts/lifetrail status
scripts/lifetrail test
scripts/lifetrail logs web
scripts/lifetrail stop all
```

Web và API cùng origin tại `http://localhost:8080` (Nginx phục vụ `/` và proxy `/api/*`). Lệnh `stop` không xóa volume PostgreSQL.

## OSRM và simulator

Sau khi chuẩn bị OSRM graph, dùng `start all` để chạy Web, server, PostgreSQL và ba profile OSRM nội bộ:

```sh
scripts/lifetrail start all
scripts/lifetrail status all
scripts/lifetrail logs osrm
scripts/lifetrail test server-tests
scripts/lifetrail stop all
```

Dùng `start osrm` để chạy riêng ba OSRM service, hoặc chọn từng service như `osrm-car`. `LT_OSRM_DATASET_VERSION` chọn phiên bản graph (mặc định `monaco-test`); đường dẫn `data/osrm` hiện liên kết tới `/mnt/storage/data/osrm` trên máy này.

Lệnh `test server-tests` chạy toàn bộ test server, bao gồm integration PostGIS, trong Docker với database test riêng. Các test này xóa dữ liệu trong database test và chạy tuần tự; không dùng database runtime.

Chạy simulator sau khi OSRM đã khởi động:

```sh
scripts/lifetrail simulate --scenario tools/scenarios/monaco-car.json \
  --metadata data/osrm/monaco-test/car/metadata.json \
  --osrm-url http://osrm-car:5000 --output runtime/osrm-demo/car
```

Dùng `scripts/lifetrail simulate --help` để xem tùy chọn. Xem [hướng dẫn chuẩn bị graph](docs/development/phase-2-sparse-and-osrm-acceptance.md) cho dataset/profile; OSRM không mở cổng ra host.

## Trạng thái

Ticket 02 thiết lập Rust/Axum, PostgreSQL/PostGIS, CLI provisioning và topology local/LAN. GPS collection, storage, đồng bộ Batch và Daily View sẽ được triển khai ở các ticket sau.
