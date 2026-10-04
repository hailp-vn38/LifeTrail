# LifeTrail — ESP32 Firmware Architecture & Development Guide

> Tài liệu này là **source of truth cho firmware ESP32** của LifeTrail.  
> Phạm vi chính: ESP32 + NEO-6M + microSD + Wi-Fi sync trong Phase 1.

## 1. Mục tiêu firmware

Firmware có trách nhiệm:

```text
Collect
  ↓
Validate
  ↓
Store locally
  ↓
Sync safely
```

Firmware **không** chịu trách nhiệm cho:

- GPS smoothing phức tạp;
- trip reconstruction;
- stop detection;
- statistics;
- timeline aggregation;
- map rendering.

Các phần trên thuộc server.

---

## 2. Hardware Phase 1

Thiết bị prototype:

```text
ESP32
+
NEO-6M
+
microSD
+
Battery / Power
```

NEO-6M kết nối với ESP32 qua UART.

Luồng:

```text
NEO-6M
   │
   │ NMEA
   ▼
ESP32
   │
   ├── RMC
   └── GGA
        │
        ▼
   GPS Record
        │
        ▼
    microSD
```

---

---

## 3. Development environment, IDE và toolchain

### 3.1 Framework chính

```text
ESP-IDF 6.1.x
```

Không dùng Arduino framework làm runtime chính cho firmware LifeTrail Phase 1.

### 3.2 IDE khuyến nghị

Lựa chọn mặc định:

```text
Visual Studio Code
+
Espressif IDF Extension
```

IDE chỉ là lớp tiện ích. Build chuẩn của project vẫn phải chạy được hoàn toàn bằng CLI:

```bash
idf.py set-target esp32
idf.py build
idf.py flash
idf.py monitor
```

Như vậy:

- developer có thể dùng VS Code;
- CI không phụ thuộc GUI/IDE;
- lỗi build trên local và CI dễ tái hiện;
- project có thể chuyển IDE mà không đổi cấu trúc source.

### 3.3 Công cụ cần cài

Tối thiểu:

```text
ESP-IDF toolchain
Python environment do ESP-IDF quản lý
CMake
Ninja
Git
USB serial driver phù hợp board ESP32
VS Code + ESP-IDF extension          # khuyến nghị, không bắt buộc
```

### 3.4 Workflow developer

```text
clone repo
   ↓
activate ESP-IDF environment
   ↓
cd firmware/esp32
   ↓
idf.py build
   ↓
idf.py flash monitor
```

Configuration:

```bash
idf.py menuconfig
```

Không chỉnh generated `sdkconfig` thủ công nếu setting đó cần được chia sẻ. Setting baseline phải đi qua:

```text
sdkconfig.defaults
Kconfig
```

### 3.5 Serial / debug

Phase 1 cần ít nhất:

- UART monitor qua `idf.py monitor`;
- log levels qua `esp_log`;
- boot reason;
- reset reason;
- heap metrics;
- GPS/SD/Wi-Fi health counters.

JTAG/OpenOCD có thể thêm khi hardware/board hỗ trợ, nhưng không phải dependency bắt buộc của Phase 1.

---

## 4. Baseline kỹ thuật

### 27.1 Firmware framework

Firmware sử dụng:

```text
ESP-IDF 6.1.x
```

Nguyên tắc versioning:

- Pin nhánh stable `6.1.x`, không phát triển production trên `master` / `latest`.
- CI và môi trường developer phải dùng cùng major/minor ESP-IDF.
- Khi nâng ESP-IDF phải có PR riêng, build/test toàn bộ firmware và kiểm tra migration guide.
- `dependencies.lock` của IDF Component Manager phải được commit để build có tính lặp lại.

### 27.2 Ngôn ngữ

Ưu tiên:

```text
C17/C11-compatible C
```

Có thể dùng C++ cho component riêng nếu thực sự cần, nhưng core Phase 1 nên giữ C để:

- dễ kiểm soát allocation;
- dependency nhỏ;
- ABI rõ ràng;
- phù hợp driver/API của ESP-IDF;
- dễ test parser và storage logic trên host.

### 27.3 Nguyên tắc dependency

Thứ tự ưu tiên:

```text
1. ESP-IDF native component
2. Espressif managed component
3. Thư viện C nhỏ, đã kiểm chứng, pin version/commit
4. Tự viết abstraction LifeTrail
```

Không đưa dependency vào chỉ vì tiện. Mỗi dependency bên ngoài phải trả lời được:

- Giải quyết vấn đề gì?
- Có allocation động không?
- Có thread-safe không?
- License gì?
- Có pin version/commit không?
- Có thể thay thế mà không ảnh hưởng business logic không?

---

---

## 5. Library / component stack

### 28.1 Stack chính thức cho Phase 1

| Chức năng | Library / Component | Nguồn | Trạng thái |
|---|---|---|---|
| RTOS | FreeRTOS | ESP-IDF | Bắt buộc |
| GPS UART | `esp_driver_uart` | ESP-IDF | Bắt buộc |
| NMEA parser | `minmea` | third-party C99 | Bắt buộc |
| SD protocol | SDMMC hoặc SDSPI | ESP-IDF | Bắt buộc |
| Filesystem | FatFS + ESP VFS | ESP-IDF | Bắt buộc |
| Wi-Fi | `esp_wifi` | ESP-IDF | Bắt buộc |
| Network interface | `esp_netif` | ESP-IDF | Bắt buộc |
| System events | `esp_event` | ESP-IDF | Bắt buộc |
| HTTP client | `esp_http_client` | ESP-IDF | Bắt buộc |
| TLS | ESP-TLS + mbedTLS | ESP-IDF | Production |
| CA verification | ESP x509 certificate bundle | ESP-IDF | Production |
| Persistent config | NVS (`nvs_flash`) | ESP-IDF | Bắt buộc |
| Timer | `esp_timer` | ESP-IDF | Bắt buộc |
| Watchdog | Task Watchdog (`esp_task_wdt`) | ESP-IDF | Bắt buộc |
| Logging | `esp_log` | ESP-IDF | Bắt buộc |
| JSON control payload | `espressif/cjson` | IDF Component Manager | Có điều kiện |
| Unit test | Unity | ESP-IDF | Bắt buộc |
| Integration test | pytest + ESP-IDF test tooling | ESP-IDF | Bắt buộc |
| OTA | `esp_https_ota` | ESP-IDF | Phase sau |

### 28.2 Vì sao dùng `minmea`

`minmea` phù hợp GPS Phase 1 vì:

- pure C99;
- không dynamic allocation trong core;
- nhỏ;
- hỗ trợ `GGA`, `RMC`, `GSA`, `GSV`, `VTG`, `ZDA`;
- phù hợp embedded/resource-constrained system.

LifeTrail chỉ expose kiểu dữ liệu của riêng mình. Không cho `struct minmea_*` thoát khỏi component GPS.

```text
NEO-6M
  ↓ NMEA
ESP-IDF UART
  ↓
minmea
  ↓
lt_gps_record_t
  ↓
FreeRTOS Queue
```

### 28.3 Cách đưa `minmea` vào repo

Khuyến nghị vendor source đã pin commit:

```text
firmware/esp32/components/third_party/minmea/
├── minmea.c
├── minmea.h
├── LICENSE
└── ORIGIN.md
```

`ORIGIN.md` ghi:

```text
Upstream: https://github.com/kosma/minmea
Pinned commit: <exact commit hash>
License: MIT
Reason: NMEA 0183 parser for GPS component
```

Không track một branch động như `master`.

### 28.4 JSON

ESP-IDF 6.x không còn built-in `json` component. Nếu cần parse ACK/config JSON thì dùng:

```yaml
dependencies:
  espressif/cjson: "^1.7.19"
```

Version resolved thực tế được khóa bởi `dependencies.lock`.

Không dùng cJSON để tạo từng GPS record 1 Hz nếu có thể tránh allocation. GPS high-volume nên dùng serializer fixed-buffer.

```text
High-volume GPS records
        ↓
fixed-buffer serializer

Control / ACK / small config
        ↓
cJSON
```

---

### 5.1 Stack chốt cho Phase 1

```text
ESP-IDF
├── FreeRTOS
├── esp_driver_uart
├── SDMMC / SDSPI
├── FatFS + VFS
├── esp_wifi
├── esp_netif
├── esp_event
├── esp_http_client
├── ESP-TLS + mbedTLS
├── NVS
├── esp_timer
├── esp_task_wdt
├── esp_log
├── Unity
└── pytest tooling

Third party
└── minmea

Managed optional
└── espressif/cjson
```

---

## 6. Kiến trúc firmware tổng thể

## 29.1 Kiến trúc logical

```text
                              ┌──────────────────────┐
                              │       NEO-6M         │
                              │     GPS / NMEA       │
                              └──────────┬───────────┘
                                         │ UART
                                         ▼
                              ┌──────────────────────┐
                              │    lifetrail_gps     │
                              │                      │
                              │ UART RX              │
                              │ Line assembler       │
                              │ NMEA parser          │
                              │ GPS validation       │
                              └──────────┬───────────┘
                                         │ lt_gps_record_t
                                         ▼
                                  FreeRTOS Queue
                                         │
                                         ▼
                              ┌──────────────────────┐
                              │ lifetrail_storage    │
                              │                      │
                              │ Batch writer         │
                              │ Flush / rotate       │
                              │ Boot recovery        │
                              │ SD health            │
                              └──────────┬───────────┘
                                         │ FatFS
                                         ▼
                              ┌──────────────────────┐
                              │       microSD        │
                              │                      │
                              │ *.open               │
                              │ *.ready              │
                              │ *.acked              │
                              └──────────┬───────────┘
                                         │
                         batch available │
                                         ▼
                              ┌──────────────────────┐
                              │  lifetrail_sync      │
                              │                      │
                              │ Batch scan           │
                              │ HTTP streaming       │
                              │ Retry/backoff        │
                              │ ACK verification     │
                              └──────────┬───────────┘
                                         │ HTTPS
                                         ▼
                              ┌──────────────────────┐
                              │    LifeTrail API     │
                              └──────────────────────┘

       ┌──────────────────────┐      ┌──────────────────────┐
       │ lifetrail_wifi       │      │ lifetrail_device     │
       │ esp_wifi/esp_netif   │      │ NVS / device config  │
       └──────────┬───────────┘      └──────────┬───────────┘
                  │                             │
                  └──────────────┬──────────────┘
                                 ▼
                      ┌──────────────────────┐
                      │   lifetrail_core     │
                      │ events / health      │
                      │ shared types         │
                      └──────────────────────┘
```

## 29.2 Quy tắc ownership

Firmware phải có ownership rõ ràng:

- `lifetrail_gps` sở hữu UART GPS và parser state.
- `lifetrail_storage` là component duy nhất sở hữu write lifecycle của GPS batch trên filesystem.
- `lifetrail_wifi` sở hữu Wi-Fi state machine.
- `lifetrail_sync` sở hữu HTTP client và retry state.
- `lifetrail_device` sở hữu persistent device config trong NVS.
- `lifetrail_core` chỉ chứa shared primitives/types; không chứa business logic của từng subsystem.

Không để nhiều task cùng mở/ghi một GPS batch.

---

---

## 7. Cấu trúc thư mục firmware

```text
firmware/esp32/
├── CMakeLists.txt
├── sdkconfig.defaults
├── partitions.csv
├── dependencies.lock
│
├── main/
│   ├── CMakeLists.txt
│   ├── app_main.c
│   └── idf_component.yml
│
├── components/
│   ├── lifetrail_core/
│   ├── lifetrail_device/
│   ├── lifetrail_gps/
│   ├── lifetrail_storage/
│   ├── lifetrail_wifi/
│   ├── lifetrail_sync/
│   ├── lifetrail_health/
│   └── third_party/
│       └── minmea/
│
└── test_apps/
    ├── gps_parser/
    ├── storage_recovery/
    └── sync_protocol/
```

```text
firmware/esp32/components/

lifetrail_core/
├── CMakeLists.txt
├── include/
│   ├── lt_types.h
│   ├── lt_events.h
│   ├── lt_error.h
│   └── lt_version.h
└── src/
    └── lt_events.c

lifetrail_device/
├── CMakeLists.txt
├── include/
│   ├── lt_device.h
│   └── lt_config.h
└── src/
    ├── lt_device.c
    └── lt_config.c

lifetrail_gps/
├── CMakeLists.txt
├── include/
│   └── lt_gps.h
└── src/
    ├── lt_gps.c
    ├── lt_gps_uart.c
    ├── lt_nmea.c
    └── lt_gps_validate.c

lifetrail_storage/
├── CMakeLists.txt
├── include/
│   └── lt_storage.h
└── src/
    ├── lt_storage.c
    ├── lt_sd.c
    ├── lt_batch_writer.c
    ├── lt_batch_recovery.c
    └── lt_batch_scan.c

lifetrail_wifi/
├── CMakeLists.txt
├── include/
│   └── lt_wifi.h
└── src/
    └── lt_wifi.c

lifetrail_sync/
├── CMakeLists.txt
├── include/
│   └── lt_sync.h
└── src/
    ├── lt_sync.c
    ├── lt_sync_http.c
    ├── lt_sync_ack.c
    └── lt_backoff.c

lifetrail_health/
├── CMakeLists.txt
├── include/
│   └── lt_health.h
└── src/
    └── lt_health.c
```

---

---

## 8. Shared data types

### 32.1 GPS record internal

```c
#pragma once

#include <stdint.h>
#include <stdbool.h>

typedef struct {
    int64_t timestamp_ms;      // UTC Unix epoch milliseconds

    double latitude_deg;
    double longitude_deg;

    float altitude_m;
    float speed_mps;
    float course_deg;
    float hdop;

    uint8_t fix_quality;
    uint8_t satellites;

    bool has_altitude;
    bool has_speed;
    bool has_course;
    bool has_hdop;
} lt_gps_record_t;
```

Quy tắc:

- Internal struct không phụ thuộc `minmea`.
- Timestamp luôn UTC.
- Trường không có dữ liệu phải thể hiện bằng `has_*` hoặc một validity mask; không dùng giá trị ma thuật như `-9999`.
- Raw value từ GPS được giữ càng gần nguồn càng tốt.

### 32.2 Không dùng string timestamp trong hot path

Không lưu timestamp nội bộ dưới dạng:

```text
2026-10-04T08:31:42Z
```

trong RAM cho mỗi point.

Nên giữ:

```text
int64 UTC epoch milliseconds
```

Formatting ISO-8601 chỉ làm khi serialize hoặc trên server/web.

---

---

## 9. FreeRTOS task model

### 33.1 Task chính

```text
┌──────────────────┬──────────────────────────────────────┐
│ Task             │ Trách nhiệm                         │
├──────────────────┼──────────────────────────────────────┤
│ gps_task         │ UART receive + NMEA parse           │
│ storage_task     │ append/flush/rotate GPS batch       │
│ sync_task        │ scan + upload + retry + verify ACK  │
│ health_task      │ counters + liveness + diagnostics   │
└──────────────────┴──────────────────────────────────────┘
```

Wi-Fi driver/event loop được ESP-IDF quản lý; `lifetrail_wifi` chủ yếu đăng ký event handler và quản lý state.

### 33.2 Relative priority

Không hard-code kiến trúc theo con số priority trong tài liệu; giữ quan hệ:

```text
GPS acquisition
      ≥
Storage durability
      >
Network sync
      >
Health / housekeeping
```

Network không bao giờ được làm GPS acquisition/starvation.

### 33.3 Không pin core sớm

Phase 1 không pin `gps_task`/`storage_task` vào CPU core cụ thể nếu chưa có profiling chứng minh cần thiết.

Mục tiêu là giảm coupling với SoC và để scheduler xử lý trước.

---

### 34.1 GPS record queue

```text
gps_task
   │
   │ lt_gps_record_t
   ▼
gps_queue
   │
   ▼
storage_task
```

Queue là bounded queue.

Suggested initial configuration:

```text
GPS_QUEUE_LENGTH = 64 records
```

Đây là giá trị khởi đầu, phải đo thực tế.

Nếu queue đầy:

1. log counter;
2. không block vô hạn UART receive;
3. ghi health error;
4. xem đây là durability failure cần điều tra.

Không silently drop GPS point mà không có metric.

### 34.2 EventGroup cho system state

Ví dụ bit:

```text
LT_STATE_SD_READY
LT_STATE_WIFI_CONNECTED
LT_STATE_IP_READY
LT_STATE_TIME_VALID
LT_STATE_SYNC_ACTIVE
LT_STATE_DEGRADED
```

### 34.3 Task notification

Dùng cho signal nhẹ một-một, ví dụ:

```text
storage_task
    │ batch became READY
    ▼
sync_task
```

Sync task vẫn phải scan filesystem; notification chỉ là wake-up hint, không phải source of truth.

---

---

## 10. GPS subsystem

### 10.1 GPS record cần thu thập

Record ban đầu nên chứa tối thiểu:

```json
{
  "timestamp": "2026-10-04T08:31:42Z",
  "latitude": 10.781234,
  "longitude": 106.692345,
  "altitude_m": 12.4,
  "speed_mps": 4.2,
  "course_deg": 127.5,
  "fix_quality": 1,
  "satellites": 8,
  "hdop": 1.3
}
```

Không nên chỉ lưu:

```text
timestamp
latitude
longitude
```

vì server sẽ cần thêm quality metadata để lọc GPS drift và xác định chất lượng fix.

---

### 10.2 Sampling

Cấu hình prototype:

```text
GPS receiver update : 1 Hz
Record interval     : 1 second
```

Đây là cấu hình khởi đầu.

Sau khi có dữ liệu thực tế có thể nghiên cứu:

- Adaptive sampling.
- Giảm sampling khi đứng yên.
- Tăng sampling khi tốc độ cao.
- Power Save Mode.

Phase 1 chưa cần tối ưu các vấn đề này.

---

### 10.3 GPS implementation

### 35.1 Input

NEO-6M gửi NMEA qua UART.

Phase 1 cần tối thiểu:

```text
RMC
GGA
```

Có thể đọc thêm:

```text
GSA
GSV
VTG
ZDA
```

nhưng không bắt buộc để hoàn thành MVP.

### 35.2 Pipeline

```text
UART driver ring buffer
        ↓
UART event queue
        ↓
line assembler
        ↓
checksum validation
        ↓
minmea parse
        ↓
RMC/GGA merge
        ↓
lt_gps_record_t
        ↓
record validation
        ↓
gps_queue
```

### 35.3 Parser state

RMC và GGA không nhất thiết chứa cùng field, nên component GPS có state ngắn hạn để ghép dữ liệu thuộc cùng epoch GPS.

Không để logic merge sentence sang `storage_task`.

### 35.4 Validation trên device

Device chỉ validation cơ bản:

- checksum NMEA;
- latitude range;
- longitude range;
- fix available / fix quality;
- timestamp parse được;
- numeric field hợp lệ.

Không làm server-level filtering như:

- drift correction;
- impossible jump detection phức tạp;
- stop/trip inference;
- map matching.

Raw data phải được giữ để server reprocess.

---

---

## 11. Time model

### 36.1 Canonical time

Canonical timestamp của LifeTrail:

```text
UTC Unix epoch milliseconds
```

Không lưu local timezone vào từng GPS point.

### 36.2 Nguồn thời gian

Ưu tiên:

```text
1. GPS UTC time/date khi valid
2. System clock đã sync trước đó
3. Record được đánh dấu time-quality thấp nếu chưa có UTC tin cậy
```

Không dùng thời điểm upload lên server làm thời gian xảy ra GPS event.

### 36.3 Server/web

Server lưu UTC.

Web convert sang timezone khi hiển thị.

Điều này tránh lỗi khi:

- user đi qua timezone khác;
- DST;
- device reboot;
- sync xảy ra sau nhiều ngày offline.

---

---

## 12. Storage architecture

### 12.1 Local storage baseline

Đề xuất dùng microSD.

Có thể chia file thành batch:

```text
/gps/
├── 2026-10-04/
│   ├── 000001.log
│   ├── 000002.log
│   └── 000003.log
│
└── 2026-10-05/
```

Mỗi file tương ứng với một batch dữ liệu.

Ưu điểm:

- Dễ retry.
- Không cần giữ quá nhiều dữ liệu trong RAM.
- Dễ kiểm tra trạng thái upload.
- Có thể xóa từng batch sau khi server xác nhận.

---

### 12.2 Storage implementation

### 37.1 Filesystem

Phase 1:

```text
microSD
  ↓
SDMMC hoặc SDSPI
  ↓
FatFS
  ↓
ESP VFS / POSIX API
```

Ưu tiên SDMMC nếu phần cứng/pin cho phép.

SDSPI dùng khi module prototype chỉ hỗ trợ SPI hoặc routing board yêu cầu.

### 37.2 Single writer

`storage_task` là writer duy nhất của GPS batches.

```text
gps_task ─┐
          ├──► queue ─► storage_task ─► FatFS
future ───┘
```

Không cho sync task ghi vào file đang được GPS writer sử dụng.

### 37.3 Directory layout

```text
/lifetrail/
├── gps/
│   ├── 2026-10-04/
│   │   ├── gps_000001.open
│   │   ├── gps_000002.ready
│   │   └── gps_000003.acked
│   └── 2026-10-05/
│
├── media/              # future
├── system/
│   └── health/
└── lost+found/         # optional recovery quarantine
```

### 37.4 Batch state machine

```text
                  create
                    │
                    ▼
                 .open
                    │
         rotate + flush + close
                    │
                    ▼
                 .ready
                    │
                 upload
                    │
             ┌──────┴──────┐
             │             │
          failure       valid ACK
             │             │
             └── retry     ▼
                         .acked
                            │
                       retention
                            │
                            ▼
                          delete
```

`.acked` không bắt buộc giữ lâu; có thể xóa sau một retention window ngắn hoặc ngay khi policy cho phép.

### 37.5 Batch rotation

Suggested initial policy:

```text
rotate khi:
- đạt 5 phút dữ liệu
OR
- file đạt 256 KiB
OR
- chuẩn bị shutdown/reboot có kiểm soát
```

Các ngưỡng phải configurable và được benchmark bằng dữ liệu thực tế.

### 37.6 Flush policy

Không `fsync()` mỗi GPS point vì tăng latency và wear/I/O overhead.

Suggested initial policy:

```text
append mỗi record
flush theo interval nhỏ
fsync khi rotate batch
```

Cần test power-loss thực tế để chọn interval.

---

### 38.1 Khuyến nghị: NDJSON/JSONL cho prototype

Mỗi dòng là một raw GPS record.

Ví dụ:

```json
{"ts_ms":1791102702000,"lat":10.7812340,"lon":106.6923450,"alt_m":12.4,"speed_mps":4.2,"course_deg":127.5,"fix_quality":1,"satellites":8,"hdop":1.3}
```

Ưu điểm:

- inspect bằng text tool;
- dễ debug hardware bring-up;
- dễ recover record hoàn chỉnh theo newline;
- server ingest đơn giản;
- không cần giữ toàn batch trong RAM.

### 38.2 Serializer

Không dùng cJSON cho hot path 1 Hz.

Dùng fixed buffer:

```c
char line[256];

int n = snprintf(
    line,
    sizeof(line),
    "{...}\\n",
    ...
);
```

Phải kiểm tra truncation (`n < 0 || n >= sizeof(line)`).

### 38.3 Future binary format

Sau khi Phase 1 ổn định có thể benchmark:

```text
CBOR
custom fixed binary record
protobuf/nanopb
```

Chỉ đổi khi có số liệu về:

- storage footprint;
- CPU;
- upload bandwidth;
- recovery complexity;
- schema evolution.

Không tối ưu format quá sớm.

---

### 39.1 Boot scan

Khi boot:

```text
mount SD
  ↓
scan /lifetrail/gps
  ↓
find *.open
  ↓
validate records
  ↓
truncate/quarantine incomplete tail if needed
  ↓
close recovery
  ↓
rename to *.ready
```

Sau đó mới bắt đầu normal recording.

### 39.2 Power loss

Test case bắt buộc:

```text
power cut:
- đang append record
- đang flush
- đang rotate
- đang upload
- ngay sau server commit nhưng trước device nhận ACK
```

Expected behavior:

- không corrupt toàn bộ lịch sử;
- không duplicate server records;
- incomplete local batch được recover hoặc quarantine;
- batch chưa ACK vẫn có thể retry.

---

---

## 13. Wi-Fi architecture

### 40.1 Native ESP-IDF

Dùng:

```text
esp_wifi
esp_netif
esp_event
```

Không dùng Arduino WiFi wrapper.

### 40.2 State flow

```text
BOOT
  ↓
load Wi-Fi config
  ↓
STA start
  ↓
CONNECTING
  │
  ├── disconnected ─► retry/backoff
  │
  └── connected
          ↓
      got IP
          ↓
      IP_READY
          ↓
      notify sync_task
```

### 40.3 Offline-first rule

Wi-Fi không được là dependency của GPS/storage initialization.

Nếu Wi-Fi lỗi:

```text
GPS continues
Storage continues
Sync paused
```

---

---

## 14. Sync protocol

### 14.1 Protocol nguyên bản

Flow:

```text
ESP32                         LifeTrail Server

  │
  │ Wi-Fi detected
  │
  ├──── POST batch ─────────────►
  │                             │
  │                             ├─ validate
  │                             ├─ insert
  │                             └─ commit
  │
  │◄──── ACK batch_id ──────────┤
  │
  ├─ mark batch synced
  │
  └─ rotate / delete later
```

Nguyên tắc:

> Không xóa local batch chỉ vì HTTP request đã được gửi.

Chỉ đánh dấu synchronized sau khi server trả về ACK hợp lệ.

---

### 14.2 Idempotency

Server cần hỗ trợ upload lại cùng batch.

Ví dụ:

```text
device_id
batch_id
```

tạo thành một khóa idempotency.

Nếu thiết bị gửi lại:

```text
batch_id = gps_000001
```

server không được insert duplicate GPS points.

Điều này rất quan trọng khi:

- Request timeout.
- ESP32 không nhận được response.
- Wi-Fi mất kết nối.
- Device reboot.

---

### 14.3 Implementation

### 41.1 Transport

Production:

```text
HTTPS
  ↓
esp_http_client
  ↓
ESP-TLS / mbedTLS
```

Server certificate phải được verify.

### 41.2 Upload endpoint đề xuất

```http
POST /api/v1/devices/{device_id}/gps-batches/{batch_id}
Content-Type: application/x-ndjson
Idempotency-Key: {device_id}:{batch_id}
X-LifeTrail-Schema: gps.v1
Authorization: Bearer <device_token>
```

Body:

```text
GPS record line 1\n
GPS record line 2\n
GPS record line 3\n
...
```

Endpoint/path/schema cuối cùng phải được khóa trong `protocol/sync-batch-v1.md`.

### 41.3 Streaming upload

Không đọc toàn bộ batch vào RAM.

```text
open file
  ↓
read 4–8 KiB
  ↓
HTTP write
  ↓
read next chunk
  ↓
...
```

Chunk size là benchmark/config value, không phải protocol guarantee.

### 41.4 ACK

Response tối thiểu:

```json
{
  "device_id": "device-123",
  "batch_id": "gps_000123",
  "status": "committed"
}
```

Device chỉ mark `.acked` nếu:

```text
HTTP request success
AND status code thuộc contract thành công
AND response JSON parse được
AND response.device_id == local device_id
AND response.batch_id == local batch_id
AND response.status == committed
```

### 41.5 Idempotency

Server unique constraint hoặc equivalent:

```text
(device_id, batch_id)
```

Nếu device retry batch đã commit:

```text
server phải trả ACK tương đương thành công
không insert duplicate raw GPS point
```

### 41.6 Retry/backoff

Không retry tight loop.

Suggested behavior:

```text
network failure
  ↓
exponential backoff + jitter
  ↓
retry khi Wi-Fi/IP còn valid
```

Backoff reset sau một upload thành công.

---

---

## 15. Device identity và NVS

### 42.1 NVS dùng cho

```text
device_id
wifi_ssid
wifi_credentials
api_base_url
device_token
config_version
firmware_state
```

Không dùng NVS cho GPS log.

### 42.2 Namespace

Ví dụ:

```text
lt_device
lt_wifi
lt_server
lt_runtime
```

### 42.3 Device ID

`device_id` phải:

- ổn định qua reboot;
- không đổi khi Wi-Fi reconnect;
- không dựa trực tiếp vào một field có thể thay đổi;
- được provision hoặc derive một lần rồi persist.

Server không dùng IP/MAC hiện tại làm primary identity của device session.

---

---

## 16. Security baseline

Phase 1 prototype có thể bring-up HTTP trong mạng dev riêng, nhưng architecture production phải hướng tới:

```text
HTTPS
Device identity
Device token / credential
Server certificate verification
Web authentication
Protected media URLs
```

Không ship production với:

```text
skip_cert_common_name_check = true
certificate verification disabled
hard-coded shared token cho mọi device
```

Secrets không commit vào Git.

---

---

## 17. Memory strategy

### 44.1 Hot path

GPS hot path:

```text
UART ring buffer
  ↓
fixed line buffer
  ↓
parser stack/static state
  ↓
fixed-size lt_gps_record_t
  ↓
bounded FreeRTOS queue
  ↓
fixed serialization buffer
```

Tránh:

```text
malloc/free mỗi NMEA sentence
malloc/free mỗi GPS record
String-style dynamic concatenation
whole-batch buffering
```

### 44.2 Heap monitoring

Health metrics nên có:

```text
free_heap
minimum_free_heap
largest_free_block
GPS queue high-water mark
```

Mục tiêu Phase 1 không chỉ "chạy được" mà phải chạy dài ngày mà không có dấu hiệu leak/fragmentation đáng kể.

---

---

## 18. Watchdog và liveness

### 45.1 Task Watchdog

Monitor tối thiểu:

```text
gps_task
storage_task
```

`sync_task` dùng network timeout riêng; chỉ đưa vào watchdog nếu lifecycle được thiết kế rõ.

### 45.2 Liveness counters

Ví dụ:

```text
gps_sentences_rx
gps_records_valid
gps_parse_errors
gps_queue_overflows
sd_write_errors
batches_created
batches_recovered
batches_uploaded
sync_retries
wifi_disconnects
http_failures
```

Counters hữu ích hơn việc ghi quá nhiều verbose log xuống SD.

---

---

## 19. Logging

Dùng `esp_log`:

```c
ESP_LOGI(TAG, "GPS fix acquired");
ESP_LOGW(TAG, "GPS queue high watermark: %u", value);
ESP_LOGE(TAG, "Batch upload failed: %s", esp_err_to_name(err));
```

Tag convention:

```text
LT_GPS
LT_STORAGE
LT_WIFI
LT_SYNC
LT_DEVICE
LT_HEALTH
```

Production giảm log level so với development.

Không log credential/token.

---

---

## 20. Error model

Mỗi component public API trả về `esp_err_t` khi phù hợp.

LifeTrail-specific error cần namespace riêng nếu cần phân biệt semantic error.

Ví dụ:

```text
ESP_OK
ESP_ERR_INVALID_ARG
ESP_ERR_TIMEOUT
ESP_ERR_NO_MEM
LT_ERR_BATCH_CORRUPT
LT_ERR_ACK_MISMATCH
LT_ERR_GPS_NO_FIX
```

Không dùng reboot làm error handling mặc định.

Recovery order:

```text
retry local operation
    ↓
reset subsystem
    ↓
degraded mode
    ↓
controlled restart nếu không recover được
    ↓
hardware watchdog là lớp cuối
```

---

---

## 21. Boot sequence

```text
app_main
  │
  ├── init NVS
  ├── load device config
  ├── init system events
  ├── init health counters
  │
  ├── mount microSD
  ├── recover unfinished batches
  │
  ├── create GPS queue
  ├── start storage_task
  ├── start gps_task
  │
  ├── start Wi-Fi manager
  ├── start sync_task
  │
  └── start health_task
```

Thứ tự quan trọng:

```text
storage ready trước khi GPS producer chạy ổn định
```

Wi-Fi có thể init sau mà không ảnh hưởng recording.

---

---

## 22. Controlled restart / shutdown

Trước controlled restart nếu có thể:

```text
stop accepting new noncritical work
  ↓
flush storage queue
  ↓
flush current batch
  ↓
close/rotate batch
  ↓
restart
```

Tuy nhiên firmware vẫn phải an toàn nếu mất điện mà không có shutdown sequence.

---

---

## 23. `app_main` responsibility

`app_main.c` chỉ làm composition/orchestration.

Không đặt parser/storage/HTTP business logic trực tiếp trong `app_main.c`.

Ví dụ conceptual:

```c
void app_main(void)
{
    ESP_ERROR_CHECK(lt_device_init());
    ESP_ERROR_CHECK(lt_health_init());
    ESP_ERROR_CHECK(lt_storage_init());

    QueueHandle_t gps_queue = lt_storage_gps_queue();

    ESP_ERROR_CHECK(lt_gps_start(gps_queue));
    ESP_ERROR_CHECK(lt_wifi_start());
    ESP_ERROR_CHECK(lt_sync_start());
}
```

API thực tế có thể thay đổi, nhưng dependency direction phải giữ sạch.

---

---

## 24. Component dependency direction

Desired graph:

```text
                     lifetrail_core
                    ▲      ▲      ▲
                   /       │       \
                  /        │        \
       lifetrail_gps  lifetrail_device  lifetrail_health
              │             │
              ▼             │
      lifetrail_storage     │
              │             │
              └──────┬──────┘
                     ▼
               lifetrail_sync
                     ▲
                     │
               lifetrail_wifi
```

Không tạo cycle kiểu:

```text
gps -> storage -> sync -> gps
```

Không để low-level component gọi trực tiếp web/server-specific code nếu không cần.

---

---

## 25. CMake conventions

Mỗi component tự khai báo source/include/dependency.

Ví dụ GPS component:

```cmake
idf_component_register(
    SRCS
        "src/lt_gps.c"
        "src/lt_gps_uart.c"
        "src/lt_nmea.c"
        "src/lt_gps_validate.c"
    INCLUDE_DIRS
        "include"
    PRIV_REQUIRES
        esp_driver_uart
)
```

`minmea` có thể được build như component riêng rồi GPS `PRIV_REQUIRES` vào component đó.

Public dependency chỉ dùng khi header public thật sự expose type/header từ dependency.

---

---

## 26. Configuration strategy

### 53.1 Compile-time config

Dùng Kconfig/sdkconfig cho:

```text
UART port/pins
SD interface
queue length
log level
feature flags
watchdog defaults
```

### 53.2 Runtime config

Dùng NVS cho:

```text
Wi-Fi credentials
API URL
Device token
batch policy override
sampling policy future
```

### 53.3 Không hard-code secrets

`sdkconfig.defaults` có thể commit.

Credential/token thật không commit.

---

```text
CONFIG_LT_GPS_UART_NUM
CONFIG_LT_GPS_UART_BAUD
CONFIG_LT_GPS_RX_BUFFER_SIZE
CONFIG_LT_GPS_QUEUE_LENGTH

CONFIG_LT_SD_USE_SDMMC
CONFIG_LT_SD_USE_SDSPI
CONFIG_LT_SD_MOUNT_POINT

CONFIG_LT_BATCH_MAX_SECONDS
CONFIG_LT_BATCH_MAX_BYTES
CONFIG_LT_STORAGE_FLUSH_SECONDS

CONFIG_LT_SYNC_HTTP_TIMEOUT_MS
CONFIG_LT_SYNC_CHUNK_BYTES
CONFIG_LT_SYNC_BACKOFF_MIN_MS
CONFIG_LT_SYNC_BACKOFF_MAX_MS

CONFIG_LT_HEALTH_INTERVAL_SECONDS
```

Các default phải có lý do và benchmark sau hardware bring-up.

---

---

## 27. Partition table

### 55.1 Phase 1 tối thiểu

Flash ESP32 giữ:

```text
NVS
PHY/config data cần thiết
Application firmware
```

GPS history nằm trên microSD.

### 55.2 OTA-ready production

Khi bật OTA, chuyển sang partition scheme có:

```text
nvs
otadata
ota_0
ota_1
```

Không thêm OTA complexity vào bring-up đầu tiên nếu nó làm chậm validation GPS/storage/sync, nhưng codebase phải tránh quyết định làm OTA khó về sau.

---

---

## 28. Firmware protocol v1

### 28.1 GPS protocol

Required fields:

```text
ts_ms
lat
lon
fix_quality
satellites
```

Optional/nullable-by-contract fields:

```text
alt_m
speed_mps
course_deg
hdop
```

Không dựa vào JSON property order.

Server phải validate range và giữ raw record ngay cả khi sau đó đánh dấu quality kém.

---

### 28.2 Server ingestion contract liên quan firmware

Server khi nhận batch phải thực hiện transaction logic tương đương:

```text
validate device
  ↓
validate batch identity
  ↓
check idempotency
  ↓
parse records
  ↓
insert raw GPS
  ↓
commit transaction
  ↓
return ACK committed
```

Không ACK `committed` trước khi transaction bền vững theo contract backend.

---

Protocol canonical phải nằm trong `protocol/`, không chỉ trong source firmware.

---

## 29. Testing strategy

### 59.1 Unit test

Bắt buộc có:

```text
NMEA parsing
coordinate conversion
RMC/GGA merge
GPS validation
GPS serializer
batch naming/state parsing
ACK parser
backoff calculation
```

### 59.2 Storage recovery tests

```text
empty file
valid file
partial last line
corrupt line
missing directory
SD remount
open batch after reboot
ready batch after reboot
```

### 59.3 Sync tests

```text
200 + valid ACK
200 + wrong batch_id
200 + malformed JSON
4xx
5xx
timeout
connection reset
Wi-Fi disconnect during body upload
server committed but ACK lost
same batch uploaded twice
```

### 59.4 Hardware-in-loop

Test thực tế:

```text
GPS outdoor cold start
GPS indoor/no-fix
SD removed
SD full
Wi-Fi unavailable
Wi-Fi flapping
server unavailable
power cycling
24h+ continuous run
```

### 59.5 Long-run test

Phase 1 nên có soak test ít nhất đủ dài để phát hiện:

- heap leak;
- queue backlog;
- file handle leak;
- gradual SD corruption;
- reconnect/retry instability;
- watchdog false positive.

---

Component phải cho phép dependency substitution ở test nếu hợp lý.

Ví dụ GPS parser test đọc NMEA từ file fixture thay vì UART thật.

```text
fixtures/
├── neo6m_walk.nmea
├── neo6m_stationary.nmea
├── invalid_checksum.nmea
└── no_fix.nmea
```

Storage test dùng temporary/test partition hoặc test filesystem abstraction thay vì yêu cầu GPS thật.

---

---

## 30. Development tools liên quan firmware

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

---

## 31. Observability

### 62.1 Device metrics snapshot

Ví dụ snapshot:

```text
uptime_s
free_heap_bytes
min_free_heap_bytes
sd_free_bytes
gps_sentences_rx
gps_records_written
gps_parse_errors
current_batch_records
ready_batch_count
last_sync_success_ms
sync_retry_count
wifi_disconnect_count
```

### 62.2 Không biến metrics thành cloud dependency

Metrics có thể upload khi online, nhưng device recording không được phụ thuộc việc metrics upload thành công.

---

---

## 32. Performance priorities

Thứ tự tối ưu:

```text
1. Không mất GPS data
2. Recover sau reboot/power loss
3. Không duplicate server data
4. Không memory leak khi chạy dài
5. Wi-Fi/network failure không ảnh hưởng acquisition
6. Storage failure được detect rõ
7. Giảm CPU/RAM
8. Giảm bandwidth/storage
9. Tối ưu pin
```

Phase 1 GPS 1 Hz không cần micro-optimize parser nếu durability chưa được chứng minh.

---

---

## 33. Power optimization roadmap

Không tối ưu aggressive power trong Phase 1.

Sau khi pipeline ổn định mới benchmark:

```text
adaptive GPS sampling
Wi-Fi duty cycle
batch upload windows
CPU frequency/power management
light sleep
deep sleep theo use case
GPS module power modes
```

Mọi power optimization phải chứng minh không làm tăng data loss hoặc recovery complexity quá mức.

---

---

## 34. Future Image / Audio integration

Kiến trúc hiện tại phải cho phép thêm producer mới:

```text
GPS ───────┐
PHOTO ─────┼──► event/storage layer
AUDIO ─────┤
SENSOR ────┘
```

Không gắn image/audio vào `lt_gps_record_t`.

Về lâu dài:

```text
TimelineEvent
├── GPS
├── PHOTO
├── AUDIO
├── STOP
├── TRIP
└── CUSTOM
```

Raw GPS vẫn là stream riêng hiệu quả; unified timeline là abstraction server/domain layer.

---

---

## 35. Coding conventions

### 66.1 Naming

LifeTrail prefix:

```text
lt_
```

Ví dụ:

```text
lt_gps_init
lt_storage_rotate_batch
lt_sync_upload_batch
lt_device_get_id
```

Type:

```text
lt_gps_record_t
lt_batch_info_t
lt_device_config_t
```

### 66.2 Static scope

Function không cần export phải `static`.

Không expose internal struct nếu opaque handle đủ dùng.

### 66.3 Error checks

Không bỏ qua return code của:

```text
file I/O
NVS
UART init
Wi-Fi init
HTTP
filesystem mount
queue send/receive quan trọng
```

### 66.4 No magic numbers

Các threshold/config phải có tên hoặc Kconfig.

---

---

## 36. Third-party policy

Mỗi third-party dependency phải có:

```text
source URL
license
pinned version/commit
reason for use
local modifications nếu có
```

Không sửa source vendor trực tiếp mà không ghi patch/change note.

Nếu fork, phải ghi rõ upstream và divergence.

---

---

## 37. CI cho firmware

Mỗi PR firmware tối thiểu chạy:

```text
format/lint policy
idf.py build
unit tests có thể chạy host/target
size report
```

Branch release chạy thêm:

```text
hardware integration tests nếu infrastructure cho phép
artifact checksum
firmware version metadata
```

Build phải reproducible ở mức dependency/version đã lock.

---

---

## 38. Firmware version metadata

Firmware expose:

```text
firmware_version
git_commit
build_type
protocol_version
ESP-IDF version
```

Log một lần khi boot.

Server có thể lưu firmware version đi kèm sync metadata để debug field deployment.

---

---

## 39. Definition of Done — Firmware Phase 1

### GPS

- UART hoạt động ổn định với NEO-6M.
- Parse RMC/GGA.
- Checksum được validate.
- `lt_gps_record_t` không phụ thuộc parser library.
- GPS 1 Hz chạy dài không leak memory.

### Storage

- microSD mount/reconnect policy rõ.
- GPS records ghi được khi offline.
- Batch rotate đúng.
- Boot recover `.open` batch.
- Power-loss test không làm mất toàn bộ history.

### Wi-Fi

- Connect/reconnect event-driven.
- Wi-Fi failure không dừng GPS/storage.

### Sync

- Upload streaming từ SD.
- Retry có backoff.
- ACK được verify semantic.
- Retry cùng batch không tạo duplicate server data.
- ACK lost case hoạt động đúng.

### Device

- Stable `device_id`.
- NVS config hoạt động qua reboot.

### Health

- Watchdog cho critical task.
- Có counters cho parse/storage/sync failures.
- Có heap/min-heap visibility.

### Testing

- Parser unit tests.
- Storage recovery tests.
- Sync failure matrix tests.
- Hardware power-cycle tests.
- Long-run/soak test.

---

---

## 40. Implementation milestones

### Milestone F0 — Toolchain

```text
ESP-IDF pinned
repo builds
CI builds
serial logging works
```

### Milestone F1 — GPS bring-up

```text
UART
 ↓
NMEA line
 ↓
minmea
 ↓
lt_gps_record_t
 ↓
serial log
```

### Milestone F2 — Durable local recording

```text
GPS
 ↓
queue
 ↓
storage task
 ↓
JSONL batch
 ↓
microSD
```

Acceptance: Wi-Fi hoàn toàn tắt vẫn ghi nhiều giờ.

### Milestone F3 — Recovery

```text
power cut
 ↓
reboot
 ↓
recover .open
 ↓
continue recording
```

### Milestone F4 — Wi-Fi

```text
STA connect
reconnect
IP events
sync wake-up
```

### Milestone F5 — Sync

```text
ready batch
 ↓
HTTPS POST stream
 ↓
server commit
 ↓
ACK
 ↓
acked/delete policy
```

### Milestone F6 — End-to-end

```text
walk with device
 ↓
offline GPS log
 ↓
return to Wi-Fi
 ↓
auto sync
 ↓
server raw GPS
 ↓
web route
```

Đây là Phase 1 milestone quan trọng nhất.

---

---

## 41. Chưa nên thêm vào firmware Phase 1

Không thêm sớm:

```text
MQTT chỉ vì "IoT"
Bluetooth provisioning phức tạp
Camera
Audio
AI
On-device trip detection
On-device stop detection
Map matching
Complex geofencing
Custom RTOS
Custom TCP protocol
Custom TLS layer
Aggressive sleep modes
Heavy C++ framework
Large dependency graph
```

Chỉ thêm khi use case cụ thể yêu cầu.

---

---

## 42. Tài liệu tham chiếu kỹ thuật

ESP-IDF:

- Stable documentation: https://docs.espressif.com/projects/esp-idf/en/stable/
- UART: https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/peripherals/uart.html
- SD/MMC: https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/storage/sdmmc.html
- FatFS: https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/storage/fatfs.html
- NVS: https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/storage/nvs_flash.html
- HTTP client: https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/protocols/esp_http_client.html
- Wi-Fi: https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/network/esp_wifi.html
- Task Watchdog: https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/wdts.html
- x509 Certificate Bundle: https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/protocols/esp_crt_bundle.html

Third-party:

- minmea: https://github.com/kosma/minmea
- ESP Component Registry: https://components.espressif.com/
- cJSON component: https://components.espressif.com/components/espressif/cjson

---

---

## 43. Kiến trúc Phase 1 được chốt

```text
                    ┌──────────────────────┐
                    │       NEO-6M         │
                    └──────────┬───────────┘
                               │ NMEA / UART
                               ▼
                    ┌──────────────────────┐
                    │   lifetrail_gps      │
                    │ UART + minmea        │
                    └──────────┬───────────┘
                               │ lt_gps_record_t
                               ▼
                         FreeRTOS Queue
                               │
                               ▼
                    ┌──────────────────────┐
                    │ lifetrail_storage    │
                    │ FatFS + SDMMC/SDSPI │
                    └──────────┬───────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │      microSD         │
                    │ .open/.ready/.acked │
                    └──────────┬───────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │  lifetrail_sync      │
                    │ esp_http_client      │
                    │ ESP-TLS              │
                    └──────────┬───────────┘
                               │ HTTPS
                               ▼
                    ┌──────────────────────┐
                    │    LifeTrail API     │
                    └──────────┬───────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │ Raw GPS Database     │
                    └──────────┬───────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │ Processing / Map     │
                    └──────────────────────┘
```

Các nguyên tắc không được phá vỡ:

```text
Device collects; server interprets.

Raw GPS is immutable.

Local storage is the durability boundary.

Network sync is asynchronous and retryable.

Server ACK is required before local data is considered synchronized.

(device_id, batch_id) provides idempotency.

Wi-Fi failure must not stop GPS recording.

Library implementation details must not leak into LifeTrail domain types.
```

Đây là baseline để bắt đầu code firmware Phase 1.

---

## 44. Quick start checklist

### Môi trường

- [ ] ESP-IDF đúng version baseline.
- [ ] `idf.py --version` hoạt động.
- [ ] ESP32 được nhận qua serial.
- [ ] `idf.py build` thành công.
- [ ] `idf.py flash monitor` thành công.

### Hardware

- [ ] NEO-6M có NMEA trên UART.
- [ ] microSD mount được.
- [ ] Wi-Fi station kết nối được.

### Firmware

- [ ] GPS record vào queue.
- [ ] Storage tạo `.open` và rotate `.ready`.
- [ ] Reboot recover batch dang dở.
- [ ] Sync upload `.ready`.
- [ ] Chỉ mark `.acked` sau ACK hợp lệ.
- [ ] Upload lại không tạo duplicate server-side.

### Test

- [ ] Parser test.
- [ ] Invalid NMEA test.
- [ ] SD recovery test.
- [ ] Network timeout/retry test.
- [ ] Long-run/soak test.
