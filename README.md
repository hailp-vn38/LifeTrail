# LifeTrail

LifeTrail ghi nhận lịch sử vị trí từ Device hoạt động offline và hiển thị Daily View trong mạng cục bộ. Phase 1 thiết lập Raw GPS, Batch bền vững và đồng bộ LAN. Phase 2 bổ sung xử lý chất lượng GPS, lịch sử hoạt động và Daily View đã xử lý, không phụ thuộc routing engine.

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
```

```sh
scripts/lifetrail start all
scripts/lifetrail status
scripts/lifetrail test
scripts/lifetrail logs web
scripts/lifetrail stop all
```

Web và API cùng origin tại `http://localhost:8080` (Nginx phục vụ `/` và proxy `/api/*`). Lệnh `stop` không xóa volume PostgreSQL.

## Phase 2

Phase 2 tạo Trip/Stop/Gap/Evidence Hole, Movement Segments và processed GPS Route Parts trực tiếp từ Raw GPS bất biến. Không cần routing service. Daily View được công bố nguyên tử và giữ snapshot tốt trước đó khi xử lý dữ liệu đến muộn hoặc thất bại.

- [Baseline Phase 2](docs/lifetrail-phase2-post-implementation-alignment.md)
- [Đặc tả hiện hành](.scratch/phase-2-timeline-osrm/spec.md)
- [Fixture suite](tools/lifetrail-phase2-testdata/README.md)

```sh
scripts/lifetrail test server-tests
python3 tools/simulate_gps.py --help
```

Integration tests chạy tuần tự trong database test riêng. Routing tools và graph provenance được giữ cho phase sau, ngoài topology và acceptance Phase 2.
