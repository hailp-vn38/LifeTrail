# LifeTrail GPS Persistence Optimization — No Accelerometer

**Rev 2 — 2026-10-07** (cập nhật sau vòng review rev 1)

## Thay đổi so với rev 1

1. **§6.1 + §23**: định nghĩa rõ hành vi khi **mất fix GPS** (trong nhà, không có epoch hợp lệ): không persist gì cả; gap lúc này là gap thật, server phân loại theo logic hiện có. Heartbeat chống gap giả chỉ áp dụng khi còn fix.
2. **§22**: mapping đầy đủ firmware interval ↔ mọi ngưỡng server (`observation_gap_s`, `short_failure_max_s`, `stop_*`); yêu cầu verify `short_failure_max_s = 10` không phân loại sai các khoảng 30s/120s ở trạng thái không di chuyển.
3. **§14**: thêm **backfill từ ring buffer** khi chuyển vào MOVING (persist 5s buffer trước transition) để trip start không trễ 2–5s.
4. **§8/§9**: heading trigger được **bypass `minimum persist interval` 2s** (guard riêng 1s) để point rẽ không lệch ~28m khỏi góc.
5. **§9**: ghi rõ đi bộ (< 2.0 m/s) không kích heading trigger — by design, time trigger 3s (~4.2m) đã đủ cho geometry đi bộ.
6. **§11/§14/§15**: định nghĩa chính xác "mostly" (≥80% epoch trong window), khởi tạo/cập nhật `stationary_center`, quy tắc drift-absorb (≤10m), gắn cờ jump (implied speed > 70 m/s).
7. **§4/§5/§12/§26**: thống nhất tên `candidate_detect_window` (30s) vs `stationary_confirm` (120s); tổng thời gian đến STATIONARY ≈ 150s sau khi dừng thật.
8. **§8**: khẳng định **mọi** state transition đều persist ngay, không riêng MOVING.
9. **§16**: heartbeat = epoch gần nhất với thời điểm heartbeat (lưới thời gian đều); loại phương án chọn best-HDOP.
10. **§17**: thêm quy tắc timestamp đơn điệu tăng trong batch.
11. **§19**: chính sách khi storage queue đầy (drop-oldest + counter, không block pipeline).
12. **§21**: batch adaptive theo state (MOVING/CANDIDATE 60s, STATIONARY 300s).
13. **§33/§34**: định lượng AC-05 (cross-track error < 15m thẳng / < 30m cua; stop boundary ±30s) và thêm AC cho no-fix.
14. **§35**: ghi rõ non-goal về pin (GPS vẫn 1 Hz 24/7).
15. **§37**: đưa viết unit test + fixture lên sớm, song song với policy.

---

## 1. Mục tiêu

Tài liệu này định nghĩa cách tối ưu việc ghi GPS trên firmware LifeTrail khi **chưa có accelerometer**.

Baseline áp dụng:

```text
branch:
integration/phase-2-timeline-osrm
```

Phase 02 hiện sử dụng flow:

```text
Device GPS
   ↓
Raw GPS immutable
   ↓
gps_points
   ↓
quality processing
   ↓
Stop / Trip / GPS Gap / Evidence Hole
   ↓
Movement Segment
   ↓
Processed GPS Route Parts
   ↓
Activity Revision
   ↓
Daily View / Timeline / Playback
```

Phase 02 không còn phụ thuộc OSRM/Valhalla.

Do đó GPS persisted trên Device là nguồn geometry quan trọng cho server.

Mục tiêu của thay đổi này:

- giảm mạnh số GPS record được ghi xuống SD;
- giảm số lần write/flush/fsync;
- giảm dung lượng Raw GPS;
- vẫn giữ đủ point để dựng route;
- vẫn phát hiện Stop chính xác;
- không tạo GPS Gap giả khi người dùng đứng yên lâu;
- không thay đổi Raw GPS thành dữ liệu nội suy;
- không làm firmware tự quyết định Trip/Stop semantic của server.

Non-goal về năng lượng (xem §35): tài liệu này tối ưu **write/SD/dung lượng**,
không tối ưu pin. GPS receiver vẫn chạy 1 Hz 24/7.

---

# 2. Nguyên tắc kiến trúc

Phải tách riêng:

```text
GPS acquisition
```

và:

```text
GPS persistence
```

GPS receiver vẫn được đọc ở:

```text
1 Hz
```

nhưng không phải tất cả Navigation Epoch hợp lệ đều được persist.

Flow mới:

```text
GPS receiver
     │
     │ 1 Hz
     ▼
GPS parser
     │
     ▼
valid Navigation Epoch
     │
     ▼
quality pre-check
     │
     ▼
movement estimator
     │
     ▼
persistence policy
     │
     ├── DROP
     │
     └── ACCEPT
            │
            ▼
        storage queue
            │
            ▼
          microSD
```

Server vẫn nhận:

```text
Raw GPS
```

nhưng Raw GPS lúc này là:

> tập hợp các GPS observations đã được Device chấp nhận để persist.

Không được tạo tọa độ giả bằng interpolation hay averaging rồi lưu như Raw GPS.

---

# 3. Không thay đổi GPS acquisition rate

Giữ:

```text
GPS acquisition = 1 Hz
```

Lý do:

- cần phát hiện movement sớm;
- cần theo dõi GPS jitter;
- cần chọn observation tốt;
- cần phát hiện transition moving/stationary;
- cần giữ khả năng tăng mật độ GPS khi người dùng di chuyển;
- không cần đợi GPS receiver wake lại từ trạng thái power-down.

Trong Phase hiện tại:

```text
1 Hz acquisition
```

không đồng nghĩa:

```text
1 Hz SD write
```

---

# 4. Các trạng thái firmware

Firmware chỉ cần ba trạng thái:

```text
MOVING
CANDIDATE_STOP
STATIONARY
```

Không triển khai semantic:

```text
Trip
Stop
Place
Walking
Cycling
Driving
```

trên firmware.

Các semantic đó vẫn thuộc server.

State machine (tên window đã thống nhất, xem §5):

```text
                 movement evidence
       ┌────────────────────────────┐
       │                            │
       ▼                            │
    MOVING                          │
       │                            │
       │ candidate condition đúng   │
       │ trong candidate_detect_    │
       │ window (30 s)              │
       ▼                            │
 CANDIDATE_STOP                     │
       │                            │
       │ stable >= stationary_      │
       │ confirm (120 s)            │
       ▼                            │
   STATIONARY                       │
       │                            │
       └──── movement evidence ─────┘
```

Tổng thời gian từ lúc người dùng dừng thật đến khi vào STATIONARY ≈
30 s (detect) + 120 s (confirm) = **~150 s** với default config.

---

# 5. Default persistence policy

Recommended defaults:

```text
GPS acquisition:
    1 Hz

minimum persist interval:
    2 s
    (áp dụng cho distance/time trigger trong MOVING;
     heading trigger dùng guard riêng 1 s — xem §9;
     transition và backfill luôn bypass — xem §8, §14)

MOVING max persist interval:
    3 s

MOVING distance trigger:
    15 m

MOVING heading trigger:
    25 degrees
    (chỉ khi speed >= 2.0 m/s; bypass min interval 2 s)

CANDIDATE_STOP persist interval:
    30 s

STATIONARY persist interval (heartbeat):
    120 s
    (đo từ lần persist gần nhất, bất kể reason)

candidate_detect_window:
    30 s

stationary_confirm:
    120 s

candidate speed ratio:
    >= 80% epoch trong detect window có speed < 0.6 m/s
    AND displacement trong window < 10 m

moving evidence (từ STATIONARY/CANDIDATE_STOP):
    >= 3 epoch liên tiếp speed >= 1.0 m/s
    OR (distance từ stationary_center >= 20 m
        AND >= 2 epoch liên tiếp speed >= 1.0 m/s)

stationary drift absorb:
    heartbeat epoch cách center <= 10 m -> cập nhật center
```

Các giá trị phải configurable.

Không hard-code trực tiếp vào processing function.

---

# 6. Vì sao Stationary vẫn phải ghi GPS

Không được dừng ghi hoàn toàn khi người dùng đứng yên.

Phase 02 hiện có:

```text
observation_gap_s = 300
```

Trong server:

```text
time_between_raw_points > 300 s
```

sẽ được hiểu là:

```text
GPS Gap
```

Do đó trường hợp:

```text
22:00 GPS
...
08:00 GPS
```

không có heartbeat ở giữa sẽ làm server hiểu thành nhiều giờ mất GPS.

Đây là sai semantic nếu thiết bị thực tế vẫn hoạt động và người dùng chỉ nằm yên.

Vì vậy phải có:

```text
stationary heartbeat
```

Recommended:

```text
120 s
```

Heartbeat interval được đo từ **lần persist gần nhất** (bất kể reason:
heartbeat, transition hay trigger trong MOVING), không phải từ heartbeat trước đó.

## 6.1. Heartbeat chỉ áp dụng khi còn fix — hành vi khi mất fix

Invariant chống gap giả (§23) **chỉ áp dụng khi firmware còn nhận được
valid Navigation Epoch** (GPS có fix).

Khi mất fix hoàn toàn (điển hình: người dùng ở trong nhà nhiều giờ, receiver
không ra epoch hợp lệ):

```text
- firmware không có gì để persist -> không persist gì cả
- KHÔNG được "kéo dài" heartbeat cuối bằng cách lặp lại tọa độ cũ
- KHÔNG được tạo epoch giả để lấp khoảng trống
- server sẽ thấy khoảng trống > observation_gap_s và phân loại thành
  GPS Gap / Evidence Hole theo logic hiện có của server
```

Đây là **gap thật** (thiết bị thật sự không có GPS evidence), không phải gap giả
do optimization. Việc phân loại đúng là trách nhiệm của server Phase 02,
firmware không được bóp méo dữ liệu để "giúp" server.

Firmware có thể đếm `no_fix_duration_s` cho telemetry nội bộ, nhưng không được
persist nó thành GPS record.

Trường hợp fix chập chờn (vài epoch rời rạc cách nhau > 300 s): xử lý như mất fix
từng đoạn; mỗi epoch hợp lệ vẫn đi qua pipeline bình thường (có thể trigger
transition nếu đủ evidence).

---

# 7. Vì sao chọn 120 giây

Server hiện có:

```text
observation_gap_s = 300 s
```

Nếu heartbeat:

```text
120 s
```

thì:

```text
normal:
120 s < 300 s
```

Nếu một heartbeat bị mất:

```text
240 s < 300 s
```

vẫn chưa tạo Gap.

Nếu dùng:

```text
180 s
```

và mất một point:

```text
360 s > 300 s
```

server sẽ có nguy cơ tạo GPS Gap giả.

Do đó:

```text
120 s
```

là giá trị an toàn hơn.

---

# 8. Persist policy theo trạng thái

## Quy tắc chung: mọi transition đều persist ngay

```text
MỌI state transition:
    MOVING -> CANDIDATE_STOP
    CANDIDATE_STOP -> STATIONARY
    CANDIDATE_STOP -> MOVING
    STATIONARY -> MOVING
đều persist ngay epoch hiện tại, bypass mọi interval.
Riêng transition vào MOVING kèm backfill theo §14.
```

## 8.1 MOVING

Trong `MOVING`, persist point khi ít nhất một điều kiện sau đúng:

```text
state transition vừa xảy ra
    -> persist ngay (bypass mọi interval)

OR

heading trigger (speed >= 2.0 m/s, delta >= 25° so với last persisted,
                  now - last_heading_persist >= 1 s)
    -> persist (bypass min interval 2 s, xem §9)

OR

distance_from_last_persisted >= 15 m
    (và thỏa min interval 2 s)

OR

elapsed_since_last_persist >= 3 s
```

Pseudo rule:

```text
if state == MOVING:
    if transition_just_happened:
        persist(reason=PERSIST_STATE_TRANSITION)   # bypass mọi interval

    else if heading_trigger_armed():               # §9
        persist(reason=PERSIST_HEADING)            # bypass min interval 2 s,
                                                  # guard riêng 1 s

    else if now - last_persist < 2s:
        drop

    else if distance >= 15m:
        persist(reason=PERSIST_DISTANCE)

    else if elapsed >= 3s:
        persist(reason=PERSIST_PERIODIC)

    else:
        drop
```

Thứ tự đánh giá là một phần của spec (transition > heading > min-interval gate
> distance > time).

---

# 9. Heading trigger

Không dùng heading trigger khi thiết bị gần như đứng yên.

GPS course/heading ở tốc độ thấp thường không ổn định.

Chỉ đánh giá:

```text
heading_change
```

khi:

```text
speed >= 2.0 m/s
```

hoặc receiver báo course đáng tin cậy.

Recommended:

```text
if speed >= 2.0 m/s
AND
heading_delta >= 25 degrees (so với course của last persisted epoch)
AND
now - last_heading_persist >= 1 s
    -> persist (PERSIST_HEADING)
```

Heading trigger **bypass** `minimum persist interval = 2 s` vì rẽ là sự kiện
hiếm, không gây write burst. Guard riêng 1 s đủ để chống trigger dồn dập từ
heading nhiễu. Lý do bypass: nếu giữ gate 2 s, ở 50 km/h point rẽ có thể lệch
tới ~28 m khỏi góc thật — đúng cái mà heading trigger sinh ra để chống.

Heading delta phải xử lý wrap-around:

```text
359° -> 2°
```

phải được hiểu là:

```text
3°
```

không phải:

```text
357°
```

**Ghi nhận by design:** ở tốc độ đi bộ (~1.4 m/s < 2.0 m/s), heading trigger
không bao giờ kích hoạt. Rẽ khi đi bộ dựa vào time trigger 3 s, tương đương
~4.2 m/point ở 1.4 m/s — đủ dày để giữ geometry khúc cua đi bộ. Không cần hạ
ngưỡng heading cho đi bộ.

---

# 10. CANDIDATE_STOP

Không chuyển thẳng:

```text
MOVING -> STATIONARY
```

chỉ vì một GPS sample báo tốc độ thấp.

GPS jitter hoặc tín hiệu kém có thể gây false stop.

Khi movement evidence suy yếu:

```text
MOVING
    ↓
CANDIDATE_STOP
```

Trong `CANDIDATE_STOP`:

```text
GPS acquisition = 1 Hz
persist = 30 s
```

Mục tiêu:

- có đủ evidence quanh transition;
- server vẫn nhìn thấy nhiều GPS point;
- tránh ghi 1 Hz trong vài phút đứng yên;
- có thể quay lại MOVING ngay nếu thiết bị tiếp tục di chuyển
  (transition vào MOVING: persist ngay + backfill §14).

---

# 11. Xác định Candidate Stop

Không sử dụng một signal đơn lẻ.

Không làm:

```text
speed < 0.6
=> stationary
```

Định nghĩa chính xác (một phần của spec, không để implementation tự diễn giải):

```text
candidate_condition =
    trong candidate_detect_window (30 s):
        >= 80% số valid epoch có speed < 0.6 m/s
        AND
        displacement (first -> last epoch trong window) < 10 m
```

Khi `candidate_condition` đúng:

```text
MOVING
    ↓
CANDIDATE_STOP
```

Có thể giữ sliding window khoảng:

```text
30 s
```

trong RAM (trùng với ring buffer §18).

Epoch bị gắn cờ jump (§15) không được tính vào 80% (loại khỏi cả tử và mẫu).

---

# 12. Chuyển CANDIDATE_STOP -> STATIONARY

Phase 02 server hiện có:

```text
stop_min_duration_s = 180 s
stop_radius_m = 30 m
```

Firmware không cần replicate chính xác Stop detection của server.

Firmware chỉ cần xác định:

```text
device appears stationary
```

Recommended:

```text
CANDIDATE_STOP duration >= stationary_confirm (120 s)
AND
candidate_condition (§11) vẫn đúng trong suốt thời gian đó
```

Sau đó:

```text
CANDIDATE_STOP
    ↓
STATIONARY
```

(persist ngay epoch transition theo §8.)

Không cần dùng chính xác 180 s vì firmware state này chỉ phục vụ persistence
optimization. Server vẫn là authority cho Stop.

Hệ quả đã biết trước: với candidate persist 30 s, biên stop mà server suy ra có
độ chính xác khoảng ±30 s so với baseline 1 Hz — được chấp nhận và định lượng
trong AC-14 (§33).

---

# 13. STATIONARY

Trong `STATIONARY`:

```text
GPS acquisition = 1 Hz
GPS persistence = 120 s (heartbeat, đo từ lần persist gần nhất)
```

Firmware vẫn quan sát toàn bộ sample 1 Hz trong RAM.

Không cần ghi tất cả xuống SD.

Ví dụ người dùng ở nhà 8 giờ:

```text
1 Hz:
8 * 3600 = 28,800 records
```

Với 120 s:

```text
8 * 3600 / 120 = 240 records
```

Giảm:

```text
~99.2%
```

so với persist 1 Hz.

---

# 14. Phát hiện quay lại MOVING

Định nghĩa chính xác `moving_evidence` (áp dụng khi ở STATIONARY hoặc
CANDIDATE_STOP):

```text
moving_evidence =
    (a) >= 3 valid epoch LIÊN TIẾP, mỗi epoch speed >= 1.0 m/s
    OR
    (b) distance(latest_epoch, stationary_center) >= 20 m
        AND >= 2 valid epoch liên tiếp speed >= 1.0 m/s
```

Không kết luận MOVING từ một sample đơn lẻ.

Khi `moving_evidence` confirmed tại thời điểm T:

```text
STATIONARY / CANDIDATE_STOP
    ↓
MOVING
```

và thực hiện **cả hai** bước sau, theo thứ tự thời gian:

```text
1. backfill: từ ring buffer (§18), lấy các valid epoch trong [T - 5 s, T)
   chưa được persist (timestamp > last_persist_ms), tối đa 6 records,
   persist theo thứ tự thời gian, reason = PERSIST_TRANSITION_BACKFILL,
   bypass min interval.

2. persist ngay epoch T, reason = PERSIST_STATE_TRANSITION.
```

Mục tiêu của backfill: confirm cần 2–5 s evidence, nếu chỉ persist từ T thì
trip start trên server trễ 2–5 s, tương đương mất ~3 m (đi bộ) đến ~70 m
(xe 50 km/h) đầu trip. Backfill 5 s lấy lại đúng đoạn đường đã đi trong lúc
confirm, với chi phí tối đa ~6 records cho mỗi lần rời stationary.

---

# 15. GPS jitter khi stationary và stationary center

Khi người dùng đứng yên, GPS có thể dao động:

```text
3 m
6 m
12 m
20 m
```

Không được coi từng displacement nhỏ là movement thật.

## 15.1. stationary_center

```text
- Khởi tạo khi vào STATIONARY: mean lat/lon của tối đa 30 valid epoch gần nhất
  trong ring buffer.
  (Đây là state nội bộ — được phép averaging. KHÔNG bao giờ persist giá trị
  này thành Raw GPS.)
- Cập nhật (drift absorb): ở mỗi stationary heartbeat, nếu
  distance(heartbeat_epoch, center) <= 10 m thì center = vị trí heartbeat_epoch.
  Quy tắc này hấp thụ drift/jitter chậm của GPS trong nhiều giờ.
- Nếu distance > 10 m: giữ nguyên center, đánh giá moving_evidence (§14).
```

## 15.2. Gắn cờ jump

```text
Một valid epoch có implied speed so với valid epoch gần nhất > 70 m/s
(= max_implied_speed_mps của server) được gắn cờ "jump":
    - không dùng làm movement evidence (§11, §14);
    - heartbeat selection (§16) bỏ qua epoch bị cờ, chọn epoch gần deadline
      nhất không bị cờ trong cửa sổ ±5 s;
    - trong MOVING, epoch bị cờ vẫn có thể được persist nếu trúng trigger
      (distance/time/heading) — để server phân loại theo quality processor
      của nó (đúng tinh thần §17: firmware không loại aggressive quá mức).
```

Với quy tắc này, test case "one bad GPS jump" (§31 case 6) được xử lý: jump đơn
lẻ không đủ consecutive evidence nên không gây transition sai; trong STATIONARY
nó cũng không bao giờ được chọn làm heartbeat.

---

# 16. Chọn epoch cho stationary heartbeat (và không averaging tọa độ Raw GPS)

Không làm:

```text
lat_avg
lon_avg
```

và persist nó như một GPS Record thật.

Raw GPS phải là observation thật từ receiver.

Quy tắc chọn epoch cho heartbeat tại deadline D:

```text
- chọn valid epoch gần nhất tại hoặc trước D;
- tìm trong cửa sổ [D - 5 s, D];
- bỏ qua epoch bị gắn cờ jump (§15.2);
- nếu không có epoch nào (mất fix quanh deadline) -> bỏ qua heartbeat lần này,
  KHÔNG persist bù, thử lại ở heartbeat tiếp theo (xem §6.1).
```

**Phương án "chọn best HDOP trong vài giây" bị loại** khỏi spec vì: làm timestamp
heartbeat không đều (khó debug, khó đối chiếu), code phức tạp hơn, trong khi
epoch gần deadline nhất vẫn là observation thật và đã qua quality pre-check.

---

# 17. Quality pre-check trên firmware

Firmware không được cố replicate toàn bộ Phase 02 quality processor.

Server hiện vẫn là authority cho:

```text
Usable
LowQuality
Excluded
Evidence Hole
```

Firmware chỉ reject các record không đủ điều kiện protocol cơ bản.

Ví dụ:

```text
invalid checksum
RMC != status A
missing matched GGA
invalid coordinates
```

Các trường hợp như:

```text
HDOP cao nhưng vẫn là GPS Record hợp lệ
```

nên cân nhắc vẫn persist tùy protocol hiện tại để server có thể phân loại.

Không được loại aggressive quá mức ở firmware khiến server mất evidence.

Quy tắc bổ sung cho timestamp:

```text
persisted timestamp = GPS epoch time của observation.
Trong một batch, timestamp phải đơn điệu tăng; epoch nào có timestamp đi ngược
(do receiver glitch) thì drop (không persist, không dùng làm evidence).
```

---

# 18. Buffer gần transition

Giữ RAM ring buffer:

```text
30–60 seconds
```

các Navigation Epoch gần nhất (valid và invalid đều lưu, kèm cờ jump).

Mục tiêu:

- hỗ trợ movement estimation (§11, §14);
- phát hiện displacement;
- phân tích stationary candidate;
- chọn heartbeat (§16);
- **backfill khi transition vào MOVING (§14)** — lý do chính khiến buffer
  phải giữ cả epoch đã bị DROP khỏi persistence.

Ví dụ:

```text
ring buffer:
60 samples @ 1 Hz
```

là rất nhỏ đối với ESP32.

---

# 19. Storage write phải tối ưu cùng persistence

Hiện behavior cũ kiểu:

```text
fopen
fwrite
fflush
fsync
fclose
```

cho từng GPS point là quá aggressive.

Không nên tiếp tục thiết kế:

```text
1 accepted record
=
1 fsync
```

Đề xuất:

```text
Persistence Filter
       ↓
RAM storage queue (bounded, ví dụ 256 records)
       ↓
storage task
       ↓
persistent/open batch file
       ↓
append buffered
```

Recommended:

```text
append:
    theo accepted record

fflush:
    khoảng 5 s

fsync:
    khoảng 10–15 s

force fsync:
    batch rotation
    graceful shutdown
    reboot transition
    storage lifecycle event
```

## 19.1. Chính sách khi storage queue đầy

Queue là bounded và storage (SD) có thể chậm/kẹt bất ngờ. Quy tắc:

```text
- khi queue đầy: drop OLDEST record, tăng storage_queue_overflow_count,
  tiếp tục nhận record mới.
- KHÔNG BAO GIỜ block GPS acquisition / persistence pipeline vì storage chậm.
- log warn khi overflow, rate-limited tối đa 1 lần/phút (xem §30).
- overflow counter được expose qua telemetry để phát hiện SD có vấn đề.
```

Mất record cũ nhất khi SD kẹt là trade-off có chủ đích: pipeline GPS realtime
quan trọng hơn việc giữ một record đã trễ hàng phút.

---

# 20. Trade-off durability

Nếu:

```text
fsync every 15 s
```

thì khi mất điện đột ngột có thể mất một phần GPS mới nhất chưa sync.

Đây là trade-off có chủ đích:

```text
lower SD wear / lower latency
vs
latest data durability
```

Recommended ban đầu:

```text
fsync = 10–15 s
```

Sau này nếu hardware có capacitor/UPS behavior có thể tune tiếp.

Không cần fsync mỗi GPS point.

---

# 21. Batch policy (adaptive theo state)

Batch cố định 60 s cho mọi state sẽ tạo ~1440 file/ngày khi stationary
(0–1 record/phút) — overhead open/upload/ingest phía server không đáng.

Recommended — rotate khi:

```text
(state in {MOVING, CANDIDATE_STOP} AND age >= 60 s)

OR

(state == STATIONARY AND age >= 300 s)

OR

size >= configured max bytes
```

Sparse GPS đồng nghĩa batch stationary 300 s vẫn chỉ chứa ~2–3 records,
nhưng số file/ngày giảm ~5 lần so với batch 60 s cố định.

Ví dụ stationary:

```text
0–1 record / phút
```

Moving:

```text
~20–30 record / phút
```

---

# 22. Server compatibility

Các config Phase 02 cần lưu ý:

```text
stop_radius_m = 30
stop_min_duration_s = 180
observation_gap_s = 300

short_failure_max_s = 10

max_hdop = 5
max_implied_speed_mps = 70
jump_distance_floor_m = 100
```

## 22.1. Mapping firmware interval ↔ ngưỡng server

| Firmware pattern | Interval | Ngưỡng server | Kết luận |
|---|---|---|---|
| MOVING persist | ≤ 3 s | `short_failure_max_s = 10` | 3 < 10 ✓ không tạo short failure giả |
| CANDIDATE_STOP persist | 30 s | `short_failure_max_s = 10` | 30 > 10 → **CẦN VERIFY** (§22.2) |
| STATIONARY heartbeat | 120 s | `short_failure_max_s = 10` | 120 > 10 → **CẦN VERIFY** (§22.2) |
| STATIONARY heartbeat | 120 s (mất 1 → 240 s) | `observation_gap_s = 300` | 240 < 300 ✓ không tạo Gap giả |
| Candidate (4 points/120 s, 30 s interval) + heartbeat | — | `stop_min_duration_s = 180`, `stop_radius_m = 30` | đủ point cho cluster; biên stop ±30 s (AC-14) |
| Jump flag | implied speed > 70 m/s | `max_implied_speed_mps = 70` | khớp ngưỡng server |

Persistence policy firmware phải đảm bảo:

```text
stationary heartbeat << observation_gap_s
```

Recommended:

```text
120 s < 300 s
```

và có margin cho một lost heartbeat.

## 22.2. Verify bắt buộc: short_failure_max_s

Các khoảng 30 s (CANDIDATE_STOP) và 120 s (STATIONARY) **vượt xa**
`short_failure_max_s = 10`. Trước khi coi implementation là đạt, phải xác nhận
với server Phase 02 rằng các khoảng này ở trạng thái không di chuyển **không**
bị phân loại sai thành short failure / Evidence Hole.

Nếu server hiện tại coi mọi khoảng > 10 s là failure bất kể ngữ cảnh, các lựa
chọn (theo thứ tự ưu tiên):

```text
a) sửa server: chỉ đánh giá short failure trong vùng MOVING-dense
   (khoảng cách giữa các point liên tiếp > 10 s nhưng < 300 s mà motion
   context là stationary thì không phải failure);

b) firmware đính kèm motion state (MOVING/CANDIDATE_STOP/STATIONARY) vào batch
   metadata để server miễn trừ đúng chỗ;

c) ghi nhận là hành vi đã biết và chấp nhận phân loại hiện tại của server
   (chỉ chọn nếu (a) và (b) đều không khả thi).
```

Mục này là một phần bắt buộc của integration test §32, không được bỏ qua.

---

# 23. Không được tạo Gap giả do optimization

Invariant quan trọng (áp dụng **khi thiết bị còn GPS fix**, xem §6.1):

```text
Device vẫn đang hoạt động
+
GPS vẫn có fix
+
người dùng đứng yên
```

không được biến thành:

```text
GPS Gap
```

trên server.

Do đó:

```text
stationary persisted interval
```

không được vượt quá:

```text
observation_gap_s
```

và nên có safety margin.

Khi **mất fix** (§6.1), firmware không persist gì và server được phép thấy Gap —
đó là gap thật, không thuộc phạm vi của invariant này.

---

# 24. Geometry quality

Phase 02 dựng geometry từ GPS observations:

```text
processed_gps
```

không có map matching.

Vì vậy không được giảm moving GPS quá mạnh.

Không khuyến nghị:

```text
moving = 10 s
```

hoặc:

```text
moving = 30 s
```

Ví dụ xe 60 km/h:

```text
10 s
≈ 167 m
```

một point.

Route sẽ cắt góc và mất chi tiết.

Recommended:

```text
moving max interval = 3 s
distance trigger = 15 m
heading trigger = 25° (bypass min interval 2 s, guard 1 s — §9)
```

---

# 25. Adaptive behavior kỳ vọng

Ví dụ đi bộ:

```text
speed ~1.4 m/s
```

3 giây:

```text
~4.2 m / point
```

Time trigger thường quyết định. (Heading trigger không kích ở tốc độ đi bộ —
by design, xem §9.)

---

Ví dụ xe chạy:

```text
speed = 50 km/h
≈ 13.9 m/s
```

Distance trigger:

```text
15 m
```

sẽ xảy ra khoảng:

```text
~1.1 s
```

nhưng minimum interval:

```text
2 s
```

sẽ giới hạn write rate.

Kết quả:

```text
~28 m / persisted point
```

ở 50 km/h.

Khi rẽ ở tốc độ này, heading trigger bypass min interval 2 s (guard 1 s) nên
point rẽ lệch khỏi góc thật tối đa ~15 m thay vì ~28 m.

---

# 26. Một số config đề xuất

Có thể thêm Kconfig tương tự:

```text
CONFIG_LT_GPS_ACQUISITION_HZ=1

CONFIG_LT_GPS_PERSIST_MIN_INTERVAL_MS=2000
CONFIG_LT_GPS_PERSIST_MOVING_MAX_INTERVAL_MS=3000

CONFIG_LT_GPS_PERSIST_DISTANCE_M=15
CONFIG_LT_GPS_PERSIST_HEADING_DEG=25
CONFIG_LT_GPS_HEADING_MIN_SPEED_MPS=2.0
CONFIG_LT_GPS_HEADING_MIN_INTERVAL_MS=1000

CONFIG_LT_GPS_CANDIDATE_STOP_INTERVAL_MS=30000
CONFIG_LT_GPS_STATIONARY_INTERVAL_MS=120000

CONFIG_LT_GPS_CANDIDATE_DETECT_WINDOW_MS=30000
CONFIG_LT_GPS_STATIONARY_CONFIRM_MS=120000
CONFIG_LT_GPS_CANDIDATE_SPEED_RATIO_PCT=80

CONFIG_LT_GPS_MOVING_SPEED_MPS=1.0
CONFIG_LT_GPS_STATIONARY_SPEED_MPS=0.6
CONFIG_LT_GPS_STATIONARY_DRIFT_ABSORB_M=10
CONFIG_LT_GPS_STATIONARY_MOVE_DIST_M=20
CONFIG_LT_GPS_JUMP_SPEED_MPS=70

CONFIG_LT_GPS_BACKFILL_WINDOW_MS=5000
CONFIG_LT_GPS_BACKFILL_MAX_RECORDS=6

CONFIG_LT_GPS_STORAGE_QUEUE_LEN=256

CONFIG_LT_GPS_BATCH_MAX_AGE_MOVING_S=60
CONFIG_LT_GPS_BATCH_MAX_AGE_STATIONARY_S=300
```

Tên cụ thể có thể điều chỉnh theo convention hiện tại của firmware.

Không cần expose tất cả config ra user-facing UI.

---

# 27. Suggested internal model

Ví dụ:

```c
typedef enum {
    LT_GPS_MOTION_MOVING,
    LT_GPS_MOTION_CANDIDATE_STOP,
    LT_GPS_MOTION_STATIONARY,
} lt_gps_motion_state_t;
```

Runtime state:

```c
typedef struct {
    lt_gps_motion_state_t state;

    uint64_t state_since_ms;

    uint64_t last_persist_ms;
    uint64_t last_heading_persist_ms;

    double last_persist_lat;
    double last_persist_lon;
    double last_persist_course_deg;

    double stationary_center_lat;
    double stationary_center_lon;
    bool has_stationary_center;

    bool has_last_persist;

    uint64_t storage_queue_overflow_count;

    // recent GPS observations for movement estimation
    // (giữ cả epoch bị DROP — phục vụ backfill §14)
    lt_gps_sample_t recent_samples[...];
} lt_gps_persistence_state_t;
```

Tách persistence policy thành component/function riêng.

Không nhét logic trực tiếp vào GPS parser.

---

# 28. Component boundary đề xuất

Nên có flow:

```text
lifetrail_gps
    ↓
Navigation Epoch
    ↓
lifetrail_gps_policy
    ↓
PersistDecision
    ↓
lifetrail_storage
```

Ví dụ:

```text
lifetrail_gps/
    parser
    navigation epoch

lifetrail_gps_policy/
    movement estimator
    persistence decision

lifetrail_storage/
    queue
    batch writer
```

Parser không được biết:

```text
SD
batch
server
```

Persistence policy không được biết:

```text
PostgreSQL
Trip
Stop
Daily View
```

---

# 29. PersistDecision

Recommended abstraction:

```text
PersistDecision
```

có thể gồm:

```text
DROP
PERSIST_PERIODIC
PERSIST_DISTANCE
PERSIST_HEADING
PERSIST_STATE_TRANSITION
PERSIST_TRANSITION_BACKFILL
PERSIST_HEARTBEAT
```

Reason có ích cho:

- unit test;
- telemetry;
- debug logs;
- tuning threshold.

Không nhất thiết upload reason lên server trong Phase hiện tại.
(Ngoại lệ: nếu cần phương án (b) ở §22.2 thì motion state phải đi kèm batch
metadata — quyết định ở integration test.)

---

# 30. Logging

Không log mỗi GPS sample trong production.

Log transition:

```text
GPS motion MOVING -> CANDIDATE_STOP
GPS motion CANDIDATE_STOP -> STATIONARY
GPS motion STATIONARY -> MOVING
```

Và persistence decision ở debug level:

```text
persist reason=distance
persist reason=heartbeat
persist reason=transition
persist reason=backfill
```

Log warn khi storage queue overflow (§19.1), rate-limited tối đa 1 lần/phút:

```text
storage queue overflow, dropped oldest, total=<count>
```

Không spam console 1 Hz.

---

# 31. Unit tests bắt buộc

## Case 1 — stationary 8 giờ

Input:

```text
GPS observations 1 Hz
position jitter <= approximately 10 m
```

Expected:

```text
state -> STATIONARY

persist approximately every 120 s

no interval > 300 s
```

Không được persist 28,800 points.

---

## Case 2 — đi bộ

Input:

```text
speed approximately 1.4 m/s
continuous path
```

Expected:

```text
MOVING
persist <= approximately 3 s interval
heading trigger không kích (speed < 2.0 m/s) — expected, không phải bug
```

Geometry server vẫn đủ mượt.

---

## Case 3 — chạy xe

Input:

```text
speed 10–20 m/s
```

Expected:

```text
distance trigger activates
minimum interval prevents excessive writes
```

---

## Case 4 — rẽ góc

Input:

```text
heading change > 25 degrees
speed > 2 m/s
```

Expected:

```text
persist around turn
heading trigger bypass min interval 2 s (guard 1 s)
point rẽ cách góc thật < 20 m ở 15 m/s
```

Không để route nối thẳng qua góc quá lớn.

---

## Case 5 — GPS jitter

Input:

```text
device stationary
random displacement 3–12 m
speed mostly low
```

Expected:

```text
does not repeatedly switch to MOVING
drift absorb (§15.1) giữ center ổn định, không trigger evidence sai
```

---

## Case 6 — one bad GPS jump

Input:

```text
stationary
one point jumps 30–80 m
then returns
```

Expected:

```text
no permanent transition to MOVING (không đủ consecutive evidence)
epoch jump bị gắn cờ, loại khỏi movement evidence
heartbeat (nếu rơi đúng lúc jump) bỏ qua epoch bị cờ, chọn epoch khác
trong MOVING, epoch jump trúng trigger thì vẫn persist để server phân loại
```

---

## Case 7 — short stop

Input:

```text
moving
pause 20 s
moving
```

Expected:

```text
may enter CANDIDATE_STOP
must return to MOVING (transition persist ngay + backfill §14)
must not require firmware Stop semantic
```

Server có quyền giữ đây là cùng một Trip.

---

## Case 8 — long stop

Input:

```text
moving
stationary > 10 min
moving
```

Expected:

```text
MOVING
-> CANDIDATE_STOP
-> STATIONARY
-> MOVING
```

Persist transition points, backfill khi vào MOVING, và periodic stationary
heartbeat.

---

## Case 9 — lost heartbeat

Simulate:

```text
stationary interval = 120 s
one persistence event lost
```

Expected interval:

```text
~240 s
```

vẫn dưới:

```text
observation_gap_s = 300 s
```

Server không tạo Gap.

---

## Case 10 — mất fix 8 giờ (mới)

Input:

```text
không có valid Navigation Epoch trong 8 giờ
(hoặc fix chập chờn, các epoch cách nhau > 300 s)
```

Expected:

```text
firmware không persist bất kỳ record nào trong khoảng mất fix
không tạo tọa độ giả, không lặp lại heartbeat cuối
server được phép phân loại thành GPS Gap / Evidence Hole (gap thật)
```

---

## Case 11 — backfill khi rời STATIONARY (mới)

Input:

```text
stationary 10 phút, sau đó di chuyển ở 5 m/s
```

Expected:

```text
STATIONARY -> MOVING confirmed trong 2–5 s
persisted records bao gồm backfill 5 s trước transition (tối đa 6 records)
trip start trên server lệch < 5 s so với thời điểm di chuyển thật
```

---

## Case 12 — rẽ ở tốc độ cao, kiểm tra bypass (mới)

Input:

```text
chạy 15 m/s, rẽ 90° tại một góc
```

Expected:

```text
heading trigger persist point rẽ mà không chờ min interval 2 s
point rẽ cách góc thật < 20 m
route không nối thẳng qua góc
```

---

# 32. Integration tests với server Phase 02

Dùng simulated data giống firmware output mới.

Phải kiểm tra:

```text
gps_points
```

ít hơn đáng kể so với fixture 1 Hz.

Nhưng output vẫn phải giữ đúng:

```text
Stop
Trip
Gap
Evidence Hole
Route Parts
Daily Snapshot
Timeline
```

Đặc biệt verify:

```text
long stationary period
```

không tạo:

```text
GPS Gap
```

(gap giả — phân biệt với gap thật khi mất fix ở §6.1/case 10).

Các mục verify bắt buộc bổ sung (rev 2):

```text
- short_failure_max_s = 10: các khoảng 30 s (candidate) và 120 s (stationary)
  không bị phân loại sai thành short failure / Evidence Hole (§22.2).
  Nếu server phân loại sai, xử lý theo một trong ba phương án ở §22.2
  trước khi coi là đạt.

- stop boundary: stop start/end lệch không quá ±30 s so với baseline 1 Hz.

- trip continuity: số lượng và tính liên tục của Trip giống baseline 1 Hz;
  trip start sau stationary lệch < 5 s nhờ backfill.

- long no-fix period: server phân loại thành Gap/Evidence Hole, và đây được
  ghi nhận là hành vi đúng (không phải regression do optimization).
```

---

# 33. Acceptance criteria

Implementation đạt yêu cầu khi:

### AC-01

GPS receiver vẫn được xử lý ở:

```text
1 Hz
```

---

### AC-02

Stationary lâu không ghi 1 GPS record/s.

---

### AC-03

Stationary heartbeat default:

```text
120 s
```

(đo từ lần persist gần nhất, chỉ khi còn fix — §6.1).

---

### AC-04

Không có stationary interval bình thường:

```text
> observation_gap_s
```

(trong điều kiện còn fix).

---

### AC-05

Moving geometry vẫn đủ chi tiết cho:

```text
processed_gps
```

không cần external routing engine.

Định lượng so với baseline persist 1 Hz trên cùng dữ liệu mô phỏng
(xem AC-13).

---

### AC-06

Mọi state transition persist ngay (bypass mọi interval); transition vào MOVING
kèm backfill §14.

---

### AC-07

GPS jitter không gây write burst.

---

### AC-08

Không averaging/interpolate coordinates thành Raw GPS giả.

---

### AC-09

Storage không:

```text
fsync
```

mỗi accepted GPS point.

---

### AC-10

Mất một stationary heartbeat không tạo Gap với default config.

---

### AC-11

Phase 02 Stop detection vẫn hoạt động với:

```text
stop_min_duration_s = 180
stop_radius_m = 30
```

---

### AC-12

Trip/Stop semantic vẫn thuộc server.

Firmware chỉ quản lý:

```text
MOVING
CANDIDATE_STOP
STATIONARY
```

cho mục đích persistence.

---

### AC-13 (mới)

So với baseline 1 Hz trên cùng dữ liệu mô phỏng:

```text
max cross-track error < 15 m trên đoạn thẳng
max cross-track error < 30 m tại khúc cua
```

(tính trên `processed_gps` route).

---

### AC-14 (mới)

Stop start/end trên server lệch không quá **±30 s** so với baseline 1 Hz
(giới hạn này đến từ candidate persist interval 30 s — đã biết trước).

---

### AC-15 (mới)

Mất fix hoàn toàn trong thời gian dài: firmware không persist gì thêm;
server phân loại thành Gap/Evidence Hole — được ghi nhận là hành vi đúng,
không phải regression.

---

### AC-16 (mới)

Storage queue đầy không bao giờ block pipeline GPS; overflow được đếm
(`storage_queue_overflow_count`) và log warn rate-limited.

---

# 34. Metrics cần đo

Trước và sau implementation cần đo:

```text
raw_navigation_epochs_per_day

persisted_gps_records_per_day

persist_ratio

sd_append_count

sd_flush_count

sd_fsync_count

average_persist_interval_moving

average_persist_interval_stationary

max_stationary_persist_gap

motion_state_transition_count

storage_queue_overflow_count          (mới)

max_crosstrack_error_vs_baseline_m    (mới: tách straight / turn)

stop_boundary_error_s                 (mới: lệch biên stop vs baseline 1 Hz)

no_fix_periods_total_s                (mới: để đối chiếu gap thật trên server)
```

Target kỳ vọng:

```text
stationary record reduction:
> 95%

fsync reduction:
> 95%

moving route quality:
max cross-track error < 15 m (thẳng) / < 30 m (cua) — AC-13

stop boundary:
±30 s — AC-14
```

---

# 35. Không làm trong thay đổi này

Không thêm:

```text
accelerometer
gyroscope
magnetometer
```

Không power-off GPS khi stationary.

Không thay đổi:

```text
Trip semantic
Stop semantic
Place detection
routing
map matching
```

Không đưa movement classification của server xuống firmware.

Không thay Raw GPS bằng processed coordinates.

**Non-goal về năng lượng:** GPS receiver vẫn chạy 1 Hz 24/7 trong thay đổi này.
Tài liệu tối ưu write/SD/dung lượng, không tối ưu pin. Power management cho
stationary (duty-cycle GPS, power-off có điều kiện) là thay đổi riêng trong
tương lai, có thể kết hợp với accelerometer (§36).

---

# 36. Future extension với accelerometer

Thiết kế state machine phải cho phép sau này thêm:

```text
accelerometer motion interrupt
```

như một evidence source.

Future:

```text
GPS evidence
      +
accelerometer evidence
      ↓
movement estimator
```

Nhưng public interface của persistence policy không nên cần thay đổi lớn.

Do đó từ bây giờ nên tránh implementation kiểu:

```text
if gps_speed ...
```

rải rác nhiều nơi.

Tất cả movement logic nên tập trung trong:

```text
lifetrail_gps_policy
```

hoặc component tương đương.

---

# 37. Recommended implementation order

Thực hiện theo thứ tự:

```text
1. Viết simulated GPS fixtures + unit test skeleton cho các case §31
   (định nghĩa expected TRƯỚC khi code policy — test và fixture đi song song
   với các bước 2–7, không để cuối)

2. Tách GPS acquisition khỏi persistence

3. Thêm GPS persistence policy

4. Thêm MOVING / CANDIDATE_STOP / STATIONARY (với định nghĩa chính xác §11, §14)

5. Thêm time trigger

6. Thêm distance trigger

7. Thêm heading trigger (bypass min interval, guard 1 s — §9)

8. Thêm stationary heartbeat (chọn epoch theo §16)

9. Thêm RAM recent-observation buffer (giữ cả epoch bị DROP — phục vụ backfill)

10. Thêm backfill khi transition vào MOVING (§14)

11. Thay per-record fsync bằng buffered storage + bounded queue (§19, §19.1)

12. Chạy unit tests, tune threshold trên fixture

13. Update simulated GPS generator (output theo policy mới)

14. Chạy Phase 02 integration/acceptance tests, BAO GỒM verify §22.2
    (short_failure_max_s) và stop boundary (AC-14)

15. So sánh record count + route quality (cross-track error, AC-13) trước/sau
```

---

# 38. Target architecture cuối cùng

```text
                     ┌─────────────────────┐
                     │     GPS Receiver    │
                     │        1 Hz         │
                     └──────────┬──────────┘
                                │
                                ▼
                     ┌─────────────────────┐
                     │   Navigation Epoch  │
                     │ validation / parse  │
                     └──────────┬──────────┘
                                │
                                ▼
                     ┌─────────────────────┐
                     │ Recent GPS Buffer   │
                     │     30–60 sec       │
                     │ (giữ cả epoch DROP) │
                     └──────────┬──────────┘
                                │
                                ▼
                     ┌─────────────────────┐
                     │ Movement Estimator  │
                     │                     │
                     │ MOVING              │
                     │ CANDIDATE_STOP      │
                     │ STATIONARY          │
                     └──────────┬──────────┘
                                │
                                ▼
                     ┌─────────────────────┐
                     │ Persistence Policy  │
                     │                     │
                     │ time                │
                     │ distance            │
                     │ heading (bypass)    │
                     │ transition (mọi     │
                     │   transition)       │
                     │ backfill (→MOVING)  │
                     │ heartbeat           │
                     └──────┬────────┬─────┘
                            │        │
                         DROP     ACCEPT
                                     │
                                     ▼
                           ┌──────────────────┐
                           │  Storage Queue   │
                           │  bounded 256     │
                           │  drop-oldest     │
                           └────────┬─────────┘
                                    │
                                    ▼
                           ┌──────────────────┐
                           │ Buffered SD File │
                           │                  │
                           │ append           │
                           │ flush ~5 s       │
                           │ fsync ~10–15 s   │
                           └────────┬─────────┘
                                    │
                                    ▼
                           ┌──────────────────┐
                           │      Batch       │
                           │  60 s (moving /  │
                           │   candidate)     │
                           │  300 s (station.)│
                           └────────┬─────────┘
                                    │
                                    ▼
                           ┌──────────────────┐
                           │      Server      │
                           │                  │
                           │ Raw GPS          │
                           │ Quality          │
                           │ Stop / Trip      │
                           │ Processed GPS    │
                           └──────────────────┘
```

---

# 39. Default configuration cuối cùng

Baseline được khuyến nghị cho implementation đầu tiên:

```text
GPS acquisition:
    1 Hz

minimum persisted interval:
    2 s (distance/time trigger; heading dùng guard riêng 1 s)

MOVING:
    max interval = 3 s
    distance = 15 m
    heading = 25° khi speed >= 2 m/s (bypass min interval)

CANDIDATE_STOP:
    persist every 30 s

STATIONARY:
    persist every 120 s (heartbeat, từ lần persist gần nhất, chỉ khi còn fix)

candidate_detect_window = 30 s  (>= 80% epoch speed < 0.6 m/s, displacement < 10 m)
stationary_confirm      = 120 s

moving evidence:
    >= 3 epoch liên tiếp speed >= 1.0 m/s,
    hoặc distance từ center >= 20 m + >= 2 epoch liên tiếp >= 1.0 m/s

stationary drift absorb: <= 10 m
jump flag: implied speed > 70 m/s

backfill vào MOVING: 5 s window, tối đa 6 records

storage:
    bounded queue 256, drop-oldest + counter
    buffered append
    flush approximately 5 s
    fsync approximately 10–15 s

batch:
    60 s khi MOVING/CANDIDATE_STOP
    300 s khi STATIONARY
```

Các threshold trên là baseline tuning ban đầu.

Chúng phải được xác nhận lại bằng realistic test data (bước 12–15 ở §37)
trước khi coi là protocol invariant.

---

# 40. Kết luận

LifeTrail không cần giảm GPS receiver xuống tần suất thấp.

Thiết kế đúng là:

```text
read GPS frequently
persist intelligently
```

Cụ thể:

```text
GPS acquisition = 1 Hz

MOVING
    sparse nhưng đủ dày cho geometry
    (heading trigger bypass min interval để giữ góc rẽ)

CANDIDATE_STOP
    giảm nhanh write rate, giữ evidence quanh transition

STATIONARY
    chỉ heartbeat 120 s (khi còn fix)
    mất fix -> không persist gì, để server thấy gap thật
```

Mọi transition persist ngay; transition vào MOVING kèm backfill 5 s để trip
không bắt đầu trễ.

Điều này giảm rất mạnh:

```text
SD writes
Raw GPS volume
server point count
database storage
processing cost
```

nhưng vẫn bảo toàn những thuộc tính Phase 02 cần:

```text
Raw GPS provenance
Stop detection (±30 s biên)
Trip continuity (backfill giữ trip start)
GPS Gap semantics (phân biệt gap giả / gap thật khi mất fix)
Processed GPS geometry (cross-track < 15 m / < 30 m)
Timeline
Playback
```

Cho đến khi LifeTrail có accelerometer, đây là persistence strategy được khuyến
nghị cho firmware hiện tại.
