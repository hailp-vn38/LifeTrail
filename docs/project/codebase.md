# LifeTrail — Codebase & Project Architecture

> Tài liệu này là **source of truth ở cấp project/repository** cho LifeTrail.  
> Chi tiết implementation của từng subsystem được tách sang tài liệu riêng, đặc biệt là firmware ESP32, server Rust và web Vue.

## 1. Mục đích tài liệu

Tài liệu này mô tả:

- LifeTrail là gì và mục tiêu sản phẩm;
- các nguyên tắc kiến trúc không được phá vỡ;
- roadmap và phạm vi Phase 1;
- kiến trúc tổng thể giữa Device / Server / Web;
- cấu trúc monorepo;
- ownership giữa các module;
- vị trí và vai trò của tài liệu kỹ thuật;
- protocol/spec nào là contract dùng chung;
- quy tắc mở rộng codebase trong tương lai.

Tài liệu này **không đi sâu vào implementation firmware**. Phần đó nằm trong [`../firmware/esp32.md`](../firmware/esp32.md).

---

## 2. Tổng quan project

### 1. Tổng quan

**LifeTrail** là một hệ thống ghi lại hành trình cá nhân theo thời gian và vị trí, cho phép lưu trữ, đồng bộ và hiển thị lại các sự kiện trong cuộc sống trên một timeline gắn với bản đồ.

Hệ thống được thiết kế theo hướng **offline-first**:

- Thiết bị có thể hoạt động và ghi dữ liệu khi không có Internet.
- Dữ liệu được lưu cục bộ an toàn trên thiết bị.
- Khi có Wi-Fi phù hợp, thiết bị tự động đồng bộ dữ liệu lên server.
- Server chịu trách nhiệm lưu trữ, xử lý, phân tích và hiển thị dữ liệu trên giao diện web.

Mục tiêu dài hạn của LifeTrail là hợp nhất nhiều loại dữ liệu vào cùng một timeline:

- GPS / vị trí.
- Tuyến đường di chuyển.
- Điểm dừng.
- Ảnh.
- Ghi âm.
- Các loại sự kiện hoặc cảm biến khác trong tương lai.

---

### 2. Mục tiêu sản phẩm

LifeTrail hướng tới việc trả lời các câu hỏi như:

- Hôm nay tôi đã đi qua những đâu?
- Tôi đã dừng ở một địa điểm trong bao lâu?
- Tôi đã chụp ảnh hoặc ghi âm ở vị trí nào?
- Tôi có thể xem lại toàn bộ hành trình của một ngày trên bản đồ hay không?
- Tại một thời điểm cụ thể, có những sự kiện nào xảy ra?
- Có thể xem ảnh, audio và vị trí trên cùng một timeline hay không?

Ví dụ timeline:

```text
08:12  Rời nhà
       📍 GPS

08:36  Dừng lại
       📍 Một địa điểm
       ⏱ 32 phút

08:40  Chụp ảnh
       📷 photo_001.jpg

08:43  Ghi âm
       🎙 note_001.opus

09:20  Tiếp tục di chuyển
       🗺 Route

10:05  Dừng
       📍 Địa điểm khác
```

---

### 3. Phạm vi chính

LifeTrail có bốn nhóm chức năng chính:

#### 3.1 GPS

Thiết bị ghi lại:

- Thời gian.
- Latitude.
- Longitude.
- Altitude nếu có.
- Speed.
- Course / heading.
- Fix quality.
- Số vệ tinh.
- HDOP.

GPS là nguồn dữ liệu chính trong Phase 1.

---

#### 3.2 Map

Web UI hiển thị dữ liệu vị trí dưới dạng:

- Tuyến đường theo ngày.
- Các điểm dừng.
- Timeline đồng bộ với map.
- Các event media trên map.
- Chi tiết một trip hoặc một khoảng thời gian.

Map không chỉ dùng để hiển thị raw GPS point mà còn hiển thị dữ liệu đã qua xử lý như:

- Trip.
- Stop.
- Photo event.
- Audio event.

---

#### 3.3 Image

Trong các phase sau, thiết bị có thể:

- Chụp ảnh.
- Lưu timestamp.
- Gắn vị trí hiện tại nếu GPS khả dụng.
- Lưu file offline.
- Upload khi có Wi-Fi.
- Hiển thị ảnh trên timeline và map.

Ảnh không được thiết kế như một thuộc tính của GPS point.

Ảnh là một **Timeline Event độc lập** và có thể có hoặc không có location.

---

#### 3.4 Audio

Thiết bị có thể:

- Ghi âm thủ công hoặc theo trigger.
- Lưu timestamp.
- Lưu location nếu có.
- Lưu file cục bộ.
- Đồng bộ lên server.
- Phát lại từ timeline trên web.

Audio cũng là một Timeline Event độc lập.

---

## 3. Nguyên tắc thiết kế

### 4.1 Offline-first

Thiết bị phải hoạt động bình thường khi:

- Không có Wi-Fi.
- Server không khả dụng.
- Upload thất bại.
- Mạng bị ngắt trong quá trình đồng bộ.

Dữ liệu chỉ được xem là đã đồng bộ khi server xác nhận thành công.

---

### 4.2 Device chỉ thu thập, server xử lý

Firmware nên giữ trách nhiệm đơn giản:

```text
Collect
  ↓
Validate
  ↓
Store
  ↓
Sync
```

Các xử lý phức tạp như:

- GPS smoothing.
- Stop detection.
- Trip reconstruction.
- Statistics.
- Timeline aggregation.

nên được thực hiện trên server.

Điều này giúp có thể thay đổi thuật toán mà không cần cập nhật firmware.

---

### 4.3 Giữ raw data

Raw GPS data không nên bị thay thế bởi dữ liệu đã xử lý.

Pipeline:

```text
Raw GPS
   ↓
Filtered trajectory
   ↓
Trip / Stop detection
   ↓
Timeline
```

Việc giữ raw data cho phép:

- Cải tiến thuật toán về sau.
- Reprocess dữ liệu lịch sử.
- Kiểm tra lỗi.
- So sánh thuật toán.
- Khôi phục khi logic xử lý sai.

---

### 4.4 Event-oriented architecture

LifeTrail không nên thiết kế toàn bộ dữ liệu xoay quanh GPS point.

Thay vào đó, dùng khái niệm chung:

```text
TimelineEvent
```

Ví dụ:

```text
TimelineEvent
├── GPS
├── PHOTO
├── AUDIO
├── STOP
├── TRIP
└── CUSTOM
```

Một event có thể có location hoặc không.

---

---

## 4. Roadmap sản phẩm

### Phase 1 — GPS

Mục tiêu:

> Một thiết bị ESP32 có thể ghi GPS liên tục khi offline, lưu dữ liệu an toàn, tự đồng bộ khi có Wi-Fi và người dùng có thể xem lại tuyến đường trên web.

Phạm vi:

- ESP32.
- NEO-6M.
- microSD.
- GPS recording.
- Offline storage.
- Wi-Fi sync.
- Server API.
- Database.
- Map view cơ bản.

Chưa cần:

- Camera.
- Microphone.
- AI.
- Geofence phức tạp.
- Power optimization nâng cao.
- Real-time tracking bắt buộc.

---

### Phase 2 — Timeline & Map

Bổ sung:

- Timeline theo ngày.
- Trip detection.
- Stop detection.
- Duration tại mỗi điểm dừng.
- Hiển thị stop và trip trên map.
- Đồng bộ map với timeline.
- Summary theo ngày.

Ví dụ:

```text
2026-10-04

07:45 — Start trip
08:21 — Stop
08:54 — Continue
09:40 — Stop
11:12 — Return
```

---

### Phase 3 — Image

Bổ sung:

- Camera.
- Capture image.
- Local image storage.
- Metadata.
- Media upload.
- Thumbnail.
- Image timeline event.
- Image marker trên map.

---

### Phase 4 — Audio

Bổ sung:

- Microphone.
- Audio recording.
- Local audio storage.
- Upload.
- Playback trên web.
- Audio timeline event.
- Gắn audio với GPS location nếu có.

---

### Phase 5 — Unified LifeTrail

Hợp nhất:

```text
GPS
Photo
Audio
Trip
Stop
Custom Event
      ↓
Unified Timeline
      ↓
Map + Media + History
```

Có thể mở rộng thêm:

- Search.
- Tags.
- Notes.
- Sensor data.
- Geofence.
- Export.
- Backup.
- AI summary.
- Semantic search.

---

---

## 5. Kiến trúc hệ thống tổng thể

```text
                    ┌──────────────────────┐
                    │      GPS Module      │
                    │       NEO-6M         │
                    └──────────┬───────────┘
                               │ UART
                               ▼
                    ┌──────────────────────┐
                    │        ESP32         │
                    │                      │
                    │ GPS Parser           │
                    │ Local Storage        │
                    │ Sync Manager         │
                    │ Future Media         │
                    └──────┬───────────────┘
                           │
                           │ microSD
                           ▼
                    ┌──────────────────────┐
                    │ Offline Storage      │
                    └──────────────────────┘

                           │ Wi-Fi
                           ▼

                    ┌──────────────────────┐
                    │    LifeTrail API     │
                    │                      │
                    │ Device API           │
                    │ Sync API             │
                    │ Media API            │
                    │ Timeline API         │
                    └──────────┬───────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │      Database        │
                    │                      │
                    │ Raw GPS              │
                    │ Events               │
                    │ Trips                │
                    │ Stops                │
                    │ Media Metadata       │
                    └──────────┬───────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │  LifeTrail Server    │
                    │  (Rust/Axum, API)    │
                    └──────────┬───────────┘
                               │  Docker network
                               │  (`/api/*` reverse proxy)
                               ▼
                    ┌──────────────────────┐
                    │    LifeTrail Web     │
                    │  (Nginx + Vue SPA)   │
                    │                      │
                    │ Timeline             │
                    │ Map                  │
                    │ Photos               │
                    │ Audio                │
                    └──────────────────────┘
```

The `web` service (Nginx) owns the public LAN port `8080` and serves the
SPA; the Rust server is API-only on the internal Compose network and no
longer builds or serves web assets.

---

### 5.1 Boundary chính

```text
LifeTrail Device
    │
    │ collect + durable local store
    ▼
Firmware / ESP32
    │
    │ HTTPS batch sync
    ▼
LifeTrail Server
    │
    ├── raw data storage
    ├── processing
    ├── trip / stop / timeline
    └── API
         │
         ▼
LifeTrail Web
    ├── daily timeline
    ├── map
    ├── trip / stop
    └── media viewer
```

Quy tắc kiến trúc:

- **Device không là nơi xử lý analytics phức tạp.**
- **Server là authoritative backend cho dữ liệu đã đồng bộ.**
- **Raw data phải được giữ lại.**
- **Web là read-oriented client**, không sở hữu business truth.
- Contract giữa các layer phải nằm trong `protocol/` hoặc OpenAPI/schema tương ứng.

---

## 6. Cấu trúc repository

Cấu trúc repo đề xuất:

```text
lifetrail/
├── README.md
├── LICENSE
├── .editorconfig
├── .gitignore
├── .github/
│   └── workflows/
│       ├── firmware-build.yml
│       ├── server-test.yml
│       └── web-test.yml
│
├── firmware/
│   └── esp32/
│       ├── CMakeLists.txt
│       ├── sdkconfig.defaults
│       ├── partitions.csv
│       ├── dependencies.lock
│       │
│       ├── main/
│       │   ├── CMakeLists.txt
│       │   ├── app_main.c
│       │   └── idf_component.yml
│       │
│       ├── components/
│       │   ├── lifetrail_core/
│       │   ├── lifetrail_device/
│       │   ├── lifetrail_gps/
│       │   ├── lifetrail_storage/
│       │   ├── lifetrail_wifi/
│       │   ├── lifetrail_sync/
│       │   ├── lifetrail_health/
│       │   └── third_party/
│       │       └── minmea/
│       │
│       └── test_apps/
│           ├── gps_parser/
│           ├── storage_recovery/
│           └── sync_protocol/
│
├── server/
│   ├── src/
│   ├── tests/
│   ├── migrations/
│   └── README.md
│
├── web/
│   ├── src/
│   ├── public/
│   └── README.md
│
├── protocol/
│   ├── README.md
│   ├── gps-record-v1.md
│   ├── sync-batch-v1.md
│   ├── timeline-event-v1.md
│   └── examples/
│
├── docs/
│   ├── architecture/
│   │   ├── system.md
│   │   ├── firmware.md
│   │   ├── sync.md
│   │   └── data-model.md
│   ├── adr/
│   └── development/
│       ├── esp-idf-setup.md
│       ├── hardware-bringup.md
│       └── testing.md
│
└── tools/
    ├── gps-log-inspector/
    ├── batch-validator/
    └── simulator/
```

Trong Phase 1 có thể chưa cần tạo mọi thư mục ngay, nhưng naming và boundary nên theo cấu trúc này.

---

### 6.1 Ý nghĩa top-level directory

```text
lifetrail/
├── firmware/
│   └── esp32/
│
├── server/
│
├── web/
│
├── protocol/
│
├── docs/
│
└── tools/
```

### firmware/

Firmware của thiết bị.

Trong Phase 1:

```text
firmware/esp32/
├── gps/
├── storage/
├── sync/
├── wifi/
└── app/
```

---

### server/

Backend của LifeTrail.

Trách nhiệm:

- Device authentication.
- Sync.
- Storage.
- GPS processing.
- Timeline.
- Media.
- API cho web.

---

### web/

Web UI:

- Daily timeline.
- Map.
- Route.
- Stop.
- Trip.
- Media viewer.

---

### protocol/

Định nghĩa các format dùng chung:

- Device upload format.
- GPS record format.
- Sync batch.
- API schemas.
- Event types.

---

### 6.2 Boundary giữa các codebase

```text
firmware/
    Device-side code, ESP-IDF, GPS, storage, Wi-Fi, sync.

server/
    Rust backend, ingestion, database, processing, API.

web/
    Vue application, map, timeline, media UI.

protocol/
    Shared contracts giữa firmware/server/web.

docs/
    Architecture, ADR, setup, bring-up, runbook.

tools/
    Host-side tooling, simulator, validators, inspectors.
```

Không import source code trực tiếp giữa `firmware/`, `server/` và `web/`. Chúng giao tiếp qua **protocol/API contract**.

---

## 7. Hệ thống tài liệu

Cấu trúc tài liệu đề xuất:

```text
docs/
├── project/
│   └── codebase.md
│
├── firmware/
│   └── esp32.md
│
├── server/
│   └── architecture.md
│
├── web/
│   └── architecture.md
│
├── architecture/
│   ├── system.md
│   ├── sync.md
│   └── data-model.md
│
├── development/
│   ├── esp-idf-setup.md
│   ├── hardware-bringup.md
│   └── testing.md
│
└── adr/
    ├── 0001-offline-first.md
    ├── 0002-raw-data-immutable.md
    └── ...
```

### 7.1 Tài liệu canonical

| Phạm vi | Tài liệu |
|---|---|
| Project/codebase | [`docs/project/codebase.md`](codebase.md) |
| Firmware ESP32 | [`docs/firmware/esp32.md`](../firmware/esp32.md) |
| Server | [`docs/server/architecture.md`](../server/architecture.md) |
| Web | [`docs/web/architecture.md`](../web/architecture.md) |
| Protocol dùng chung | `protocol/*.md` |
| Quyết định kiến trúc | `docs/adr/*.md` |

### 7.2 README cấp root

`README.md` ở root chỉ nên chứa:

- LifeTrail là gì;
- quick architecture diagram;
- cách bootstrap repository;
- link sang tài liệu chi tiết;
- trạng thái roadmap hiện tại.

Không copy toàn bộ kiến trúc vào `README.md` để tránh drift.

---

## 8. Protocol ownership

Các format chia sẻ giữa device/server không được chỉ nằm trong code firmware.

Nguồn tài liệu canonical:

```text
protocol/
├── gps-record-v1.md
├── sync-batch-v1.md
└── timeline-event-v1.md
```

Firmware/server implementations phải tham chiếu cùng protocol version.

Ví dụ:

```text
GPS schema: gps.v1
Sync protocol: sync.v1
Timeline schema: event.v1
```

Breaking change phải tăng version.

---

### 8.1 Shared protocol directory

```text
protocol/
├── gps-record-v1.md
├── sync-batch-v1.md
├── timeline-event-v1.md
├── media-upload-v1.md          # future
└── examples/
```

Firmware và server **không được tự định nghĩa hai phiên bản khác nhau** cho cùng một payload.

---

## 9. Data ownership

LifeTrail được thiết kế như một hệ thống dữ liệu cá nhân.

Các nguyên tắc nên duy trì:

- User kiểm soát dữ liệu.
- Có khả năng self-host.
- Không phụ thuộc cloud để thiết bị hoạt động.
- Raw data có thể export.
- Media không bị khóa vào một vendor.
- Server có thể backup độc lập.

---

### 9.1 Ownership theo layer

| Dữ liệu | Owner |
|---|---|
| GPS trước khi sync | Device local storage |
| Raw GPS đã ACK | Server database |
| Trip / Stop | Server processing |
| Timeline | Server read model |
| Map viewport / selected event | Web UI state |
| Device config | Device NVS + server-side config khi cần |
| Protocol schema | `protocol/` |

---

## 10. Security ở cấp project

Các phase sau cần bổ sung:

- Device identity.
- Device token.
- HTTPS.
- Authentication cho web.
- Signed media URLs hoặc auth-protected media.
- Encryption at rest nếu cần.
- Backup.

Phase 1 ít nhất cần xác định một `device_id` ổn định cho mỗi thiết bị.

---

Security implementation cụ thể của device nằm trong tài liệu firmware; auth/API security nằm trong tài liệu server.

---

## 11. Khả năng mở rộng thiết bị

Thiết kế không nên khóa vào NEO-6M.

Trong tương lai thiết bị LifeTrail có thể có:

```text
GPS / GNSS
Camera
Microphone
IMU
Temperature
Battery telemetry
BLE
Other sensors
```

Do đó:

```text
LifeTrail Device
```

nên là abstraction tổng quát hơn:

```text
GPS Tracker
```

---

---

## 12. Phase 1 Definition of Done

Phase 1 được xem là hoàn thành khi đạt được toàn bộ các mục sau:

#### Device

- ESP32 đọc được NEO-6M ổn định.
- Parse được GPS.
- Ghi record định kỳ.
- Không mất dữ liệu khi mất Wi-Fi.
- Dữ liệu được lưu trên microSD.
- Device có thể reboot và tiếp tục hoạt động.

#### Sync

- Detect Wi-Fi.
- Upload batch.
- Retry được.
- Không duplicate data.
- Chỉ đánh dấu synced sau ACK.

#### Server

- Nhận batch.
- Validate.
- Lưu raw GPS.
- Query theo device/time.
- Không duplicate batch.

#### Web

- Chọn một ngày.
- Load GPS points.
- Vẽ route trên map.
- Xem start/end.
- Xem summary đơn giản.

---

---

## 13. Không thuộc Phase 1

Các chức năng sau không nên làm quá sớm:

- Camera.
- Audio.
- AI.
- Object recognition.
- Speech recognition.
- Advanced geofencing.
- Automatic place naming.
- Complex trip semantics.
- Aggressive battery optimization.
- Real-time cloud streaming bắt buộc.

Mục tiêu Phase 1 là kiểm chứng:

```text
GPS
 ↓
Offline storage
 ↓
Sync
 ↓
Server
 ↓
Database
 ↓
Map
```

---

---

## 14. Development tools cấp repository

### 61.1 GPS log inspector

Tool host-side:

```text
tools/gps-log-inspector/
```

Chức năng:

- validate NDJSON;
- count points;
- report invalid coordinates;
- inspect timestamps;
- export CSV/GeoJSON;
- visualize quick route nếu cần.

### 61.2 Batch validator

```text
tools/batch-validator/
```

Kiểm tra:

- filename/state;
- schema;
- line parse;
- batch ID;
- timestamp ordering;
- duplicate record suspicion.

### 61.3 Device simulator

Sau Phase 1 có thể tạo simulator gửi batch từ laptop tới server để test backend không cần ESP32.

---

Ngoài các tool trên, repository có thể bổ sung:

```text
tools/
├── gps-log-inspector/
├── batch-validator/
├── simulator/
├── protocol-fixtures/
└── migration-checker/          # future
```

Tool host-side không được trở thành dependency runtime của firmware.

---

## 15. Architectural Decision Records

Trong `docs/adr/`:

```text
0001-use-esp-idf.md
0002-offline-first-device.md
0003-raw-gps-immutable.md
0004-batch-sync-idempotency.md
0005-use-minmea.md
0006-use-fatfs-on-microsd.md
0007-jsonl-phase1-local-format.md
0008-server-side-trip-stop-processing.md
0009-utc-canonical-time.md
```

ADR giúp tránh việc sau vài tháng phải tranh luận lại các quyết định nền tảng mà không biết lý do ban đầu.

---

### 15.1 Quy tắc ADR

Một thay đổi nên có ADR nếu nó làm thay đổi một trong các điểm sau:

- data ownership;
- protocol format;
- persistence model;
- sync semantics;
- framework/runtime chính;
- database/storage technology;
- security boundary;
- backward compatibility.

---

## 16. Quy tắc dependency giữa các subsystem

```text
Firmware ──HTTPS/protocol──► Server ──REST/SSE/OpenAPI──► Web
Browser/ESP32 ──HTTP :8080──► Web (Nginx) ──/api/* proxy──► Server

Firmware ─X─ import Server source
Server   ─X─ import Web source
Server   ─X─ build/serve Web assets (production)
Web      ─X─ access Database trực tiếp
```

Mỗi subsystem có thể phát triển và test độc lập nếu contract không thay đổi.

---

## 17. Quy tắc versioning

Khuyến nghị version độc lập:

```text
Firmware version       0.x / 1.x
Server version         0.x / 1.x
Web version            0.x / 1.x
Protocol version       v1, v2, ...
Database migration     monotonic migration ID
```

Không dùng version firmware làm protocol version.

Một firmware cũ phải có thể sync nếu server vẫn hỗ trợ protocol version mà firmware đó gửi.

---

## 18. Branch / CI boundary

CI cấp repository nên tách job:

```text
firmware-build
firmware-test
server-build
server-test
web-build
web-test
protocol-validation
```

Thay đổi chỉ ở `web/` không cần compile ESP32, và ngược lại, trừ khi thay đổi `protocol/`.

Nếu `protocol/` thay đổi, CI nên chạy ít nhất:

```text
firmware contract test
+
server contract test
+
web generated API/type check nếu liên quan
```

---

## 19. Tầm nhìn dài hạn

LifeTrail không chỉ là một GPS tracker.

Mục tiêu cuối cùng là tạo ra một **personal spatial timeline**:

```text
Where
+
When
+
What happened
+
Media
       ↓
LifeTrail
```

Một ngày có thể được xem lại dưới dạng:

- Đã đi đâu.
- Đi tuyến nào.
- Dừng ở đâu.
- Bao lâu.
- Chụp những ảnh gì.
- Ghi âm những gì.
- Các sự kiện xảy ra ở đâu và vào lúc nào.

GPS là nền tảng đầu tiên để xây dựng toàn bộ hệ thống đó.

---

---

## 20. Tài liệu liên quan

```text
docs/project/codebase.md
    ↓
    ├── docs/firmware/esp32.md
    ├── docs/server/architecture.md
    ├── docs/web/architecture.md
    └── protocol/*.md
```

Khi có xung đột:

1. Protocol spec quyết định format truyền dữ liệu.
2. ADR mới nhất quyết định architectural decision.
3. Tài liệu subsystem quyết định implementation nội bộ subsystem.
4. `docs/project/codebase.md` quyết định boundary ở cấp project.
