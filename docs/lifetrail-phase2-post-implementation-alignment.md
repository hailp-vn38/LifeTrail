# LifeTrail Phase 2 — Post-Implementation Alignment Guide

## Mục đích

Tài liệu này dùng sau khi implementation Phase 2 đã hoàn thành nhưng chưa commit.

Mục tiêu là giúp agent hiểu **baseline Phase 2 mới** và thực hiện cleanup cuối cùng trước khi commit.

Quyết định quan trọng nhất:

> **Phase 2 không còn phụ thuộc routing/map-matching engine.**

OSRM, Valhalla và mọi logic map matching được đưa ra khỏi Definition of Done Phase 2.

Phase 2 tập trung vào:

```text
Raw GPS
  ↓
quality processing
  ↓
outlier / unusable observation handling
  ↓
Trip / Stop / Gap / Evidence Hole
  ↓
Movement Segment
  ↓
Processed GPS Route Parts
  ↓
Activity Revision
  ↓
Daily Projection
  ↓
Timeline / Daily View API
  ↓
Web Daily Map / Timeline / Playback
```

## 1. Scope chính thức của Phase 2

Phase 2 phải hoàn thiện được flow sau mà **không cần routing service bên ngoài**:

```text
Device Batch
    ↓
Raw GPS immutable storage
    ↓
dirty processing range
    ↓
processing worker
    ↓
GPS quality evaluation
    ↓
activity segmentation
    ↓
movement classification
    ↓
processed GPS geometry
    ↓
activity revision
    ↓
atomic daily publication
    ↓
Daily View API
    ↓
Vue Web
```

Các output chính:

- Trip
- Stop
- Gap
- Evidence Hole
- Movement Segment
- Processed Route Part
- Historical progress anchors
- Daily projection
- Timeline
- Summary
- Playback-ready geometry
- Revision/publication state

## 2. Raw GPS vẫn là source of truth

Không sửa hoặc xóa Raw GPS để làm route đẹp hơn.

```text
gps_points = immutable observations
```

Processor chỉ tạo derived data.

Ví dụ một GPS point bị jump:

```text
Raw:

● ● ● ● ---------------- X
          impossible jump
● ● ●

Derived route:

●──●──●──●     ●──●──●
```

Point `X` vẫn tồn tại trong Raw GPS nhưng có thể bị loại khỏi geometry đã publish.

Không được:
- update tọa độ Raw GPS;
- xóa Raw GPS vì quality kém;
- snap Raw GPS sang road;
- overwrite Raw GPS bằng processed coordinates.

## 3. Phân biệt Gap và Evidence Hole

### Gap

Gap tồn tại khi thực sự không có Raw GPS observations giữa hai observation boundaries.

```text
Raw observation
      ●

      [no records]

                     ●
              Raw observation
```

Expected:

```text
Activity
   ↓
  Gap
   ↓
Activity
```

Không nối geometry qua Gap.

Không cộng distance connector qua Gap.

Không interpolate playback qua Gap.

Gap có thể xuất hiện trong Timeline.

### Evidence Hole

Evidence Hole xảy ra khi Raw GPS records vẫn tồn tại nhưng processor không đủ bằng chứng để xác định activity/geometry đáng tin cậy.

```text
raw raw raw raw raw
 x   x   x   x   x
```

Ví dụ:
- fix quality rất thấp;
- HDOP quá cao;
- nhiều observations không usable;
- activity semantics không thể xác định chắc chắn.

Evidence Hole:
- không phải Gap;
- không tự tạo Stop;
- không tự tạo Trip;
- không được matcher/routing error tạo ra;
- chưa cần là Timeline event trong Phase 2.

API có thể expose:

```json
{
  "evidence_holes": [
    {
      "from": "...",
      "until": "...",
      "reason": "insufficient_quality"
    }
  ]
}
```

## 4. Isolated bad points không tự tạo Evidence Hole

Một số record bị loại không có nghĩa toàn interval trở thành Evidence Hole.

```text
usable ●
       x
       x
usable ●
usable ●
```

Nếu neighboring evidence đủ tốt, processor vẫn có thể tạo activity/route liên tục.

Rule:

```text
excluded observation != evidence hole
```

Evidence Hole chỉ nên tạo khi **một khoảng liên tục thực sự không còn đủ evidence**.

Agent cần đặc biệt kiểm tra các output kiểu:

```text
18:35:01 - 18:35:04
18:35:05 - 18:35:08
18:35:09 - 18:35:12
...
```

Nếu đây chỉ là các nhóm quality yếu ngắn, không nên biến thành hàng chục Evidence Hole nhỏ.

## 5. Trip và Movement Segment

Trip là continuous journey.

```text
Trip
├── MovementSegment WALK
├── MovementSegment CAR
└── MovementSegment UNKNOWN
```

Mode transition không tự split Trip.

Ví dụ:

```text
Home
  ↓ WALK
Parking
  ↓ CAR
Office
```

Nếu không có Stop hoặc Gap đủ điều kiện ở Parking:

```text
ONE Trip
├── WALK
└── CAR
```

## 6. Transport mode trong Phase 2

Phase 2 vẫn có thể classify:

```text
WALK
BIKE
CAR
UNKNOWN
```

Nhưng transport mode hiện chỉ phục vụ:
- Timeline;
- UI/icon;
- analytics;
- summary;
- future routing integration.

Transport mode **không quyết định geometry source** trong Phase 2.

```text
WALK    ┐
BIKE    ├──> processed GPS geometry
CAR     │
UNKNOWN ┘
```

Không còn flow:

```text
CAR  -> osrm-car
BIKE -> osrm-bike
WALK -> osrm-foot
```

trong Phase 2.

## 7. Route geometry của Phase 2

Canonical geometry source:

```text
route_source = processed_gps
```

Route được tạo từ usable GPS observations sau:
- quality filtering;
- outlier handling;
- deterministic reduction nếu cần;
- evidence-boundary handling.

Một Movement Segment có thể có nhiều Route Parts:

```text
MovementSegment
├── RoutePart A
├── RoutePart B
└── RoutePart C
```

Ví dụ bị Evidence Hole:

```text
RoutePart A

Evidence Hole

RoutePart B
```

Không nối giả A → B.

## 8. Progress anchors vẫn giữ

Routing bị loại khỏi Phase 2 nhưng Route Parts + historical progress anchors vẫn cần giữ.

```json
{
  "id": "route-part-1",
  "source": "processed_gps",
  "geometry": {
    "type": "LineString",
    "coordinates": []
  },
  "anchors": [
    {
      "recorded_at": "...",
      "distance_m": 0.0
    },
    {
      "recorded_at": "...",
      "distance_m": 318.2
    }
  ]
}
```

Anchors dùng cho:
- playback;
- seeking;
- clipping theo ngày;
- distance progress;
- moving puck;
- Course-Up/Heading-Up camera.

## 9. Distance authority nằm ở server

Server phải sở hữu metric distance/progress.

RoutePart có thể expose:

```text
vertex_distance_m
```

với invariant:

```text
vertex_distance_m[0] = 0
vertex_distance_m[i+1] >= vertex_distance_m[i]
```

Daily clipping dùng progress difference.

Không để Web tự tính lại một metric distance khác.

Không cộng connector giữa Route Parts nếu connector đó không tồn tại trong published geometry.

## 10. OPEN activity boundaries

Activity không được coi là kết thúc chỉ vì dataset hiện tại kết thúc.

Có thể có:

```text
start_boundary = open
end_boundary   = open
```

Dùng:

```text
observed_from_at
observed_until_at
```

để mô tả phần đã quan sát.

Không ngoại suy:

```text
OPEN activity -> now()
OPEN activity -> end of day
```

Nếu boundary chưa confirmed:

```text
full_duration = unknown/null
```

nhưng observed duration vẫn tính được.

## 11. Cross-midnight activity

Trip/Stop/Gap không thuộc riêng một calendar day.

```text
23:50 ────────────── 00:20
           STOP
```

Đây là một Stop, không phải hai Stop.

Daily View chỉ là projection:

```text
Day 1 -> visible overlap
Day 2 -> visible overlap
```

Daily summary chỉ tính phần giao với Owner-local day.

## 12. Activity Revision và Daily Publication

ActivityRevision chỉ chứa continuous range được processing.

Không copy toàn bộ history của Device mỗi lần late data tới.

```text
ActivityRevision
    ↓
continuous replacement range
```

Revision là immutable.

Daily publication là immutable snapshot/read model.

Tách:

```text
activity_revision_id
```

khỏi:

```text
published_revision
```

Publication có thể tái sử dụng activity data ngoài replacement range.

## 13. Late data

Late Batch:

```text
commit Raw GPS
    ↓
input_generation++
    ↓
mark dirty
    ↓
worker reprocess
```

Snapshot cũ tiếp tục được phục vụ trong lúc processing.

```text
R7 published
    ↓
late data arrives
    ↓
R7 remains visible
processing = queued/running
    ↓
R8 success
    ↓
atomic publication
```

Nếu R8 fail:

```text
R7 remains published
```

Không fallback về Raw Daily View nếu đã có good processed snapshot.

## 14. input_generation

`input_generation` tăng đúng một lần khi một Batch mới commit thành công.

Tăng với:
- late Batch;
- overlapping timestamps;
- poor-quality Raw GPS;
- new Batch có timestamps trùng Batch cũ.

Không tăng với:
- identical replay;
- batch conflict;
- validation failure;
- transaction rollback;
- config change;
- processing version change.

## 15. Worker protocol

Phase 2 dùng PostgreSQL durable queue.

Scope hiện tại:

```text
one processing worker
```

nhưng protocol vẫn phải bảo vệ race.

Worker cần:

```text
lease + fencing token
```

Publication phải verify:

```text
input_generation
processing target
fencing authority
```

trước khi activate candidate.

Flow:

```text
claim job
   ↓
capture generation/target
   ↓
release transaction
   ↓
process
   ↓
short publication transaction
   ↓
verify
   ↓
publish or reject candidate
```

## 16. Atomic multiday publication

Nếu một revision thay activity xuyên midnight:

```text
2026-10-05
2026-10-06
```

thì cả hai affected Daily Views phải được publish trong cùng publication transaction.

Không để:

```text
Day A = R8
Day B = R7
```

nếu cùng một activity revision làm thay đổi cả hai.

## 17. Web publication behavior

Web có thể poll processing status mỗi 5 giây khi:

```text
queued
running
```

Không cần SSE trong Phase 2.

Khi `published_revision` đổi:

```text
pause playback
reset playback
clear selected event
refetch full Daily View
replace route/timeline/summary atomically
```

Không merge data giữa hai revision.

Khi tab hidden:

```text
stop polling
```

Khi focus lại:

```text
immediate status refetch
```

## 18. Timezone

Activity được lưu theo UTC.

Timezone chỉ thuộc Daily Projection.

```text
UTC Activities
      ↓
Owner timezone
      ↓
Daily View
```

Owner đổi timezone:
- không rerun GPS quality processing;
- không rerun segmentation;
- không rerun classification;
- không rebuild activity revisions;
- chỉ rebuild Daily Projections.

## 19. Summary semantics

Summary cần phân biệt:

```text
point_count
usable_point_count

trip_count
stop_count
gap_count

trip_duration_s
stop_duration_s
gap_duration_s
```

Không dùng `moving_duration_s` nếu nó thực chất là toàn duration của Trip.

Trip duration có thể bao gồm short pauses chưa đủ điều kiện Stop.

## 20. Evidence state

Tách evidence khỏi processing state.

Ví dụ:

```text
processing_state = ready
evidence_state   = insufficient
```

là hợp lệ.

Suggested values:

```text
sufficient
partial
insufficient
```

Một ngày chỉ có 1 GPS record vẫn có thể:

```text
processing_state = ready
timeline         = []
route_parts      = []
evidence_state   = insufficient
```

Không tạo activity giả để lấp ngày.

# 21. Những thứ PHẢI loại khỏi Phase 2 hiện tại

Agent phải tìm và loại bỏ/de-scope mọi implementation hoặc config chỉ phục vụ routing/map matching.

## 21.1 OSRM infrastructure

Loại khỏi Phase 2:

```text
osrm-car
osrm-bike
osrm-foot
```

và các:

```text
docker-compose OSRM services
OSRM ports
OSRM volumes
OSRM health checks
OSRM startup dependencies
```

Không cần OSRM container để Phase 2 chạy.

## 21.2 Valhalla

Nếu đã thử/thêm Valhalla:

```text
valhalla service
valhalla tiles
valhalla environment variables
trace_route / trace_attributes clients
```

cũng loại khỏi Phase 2.

Routing engine sẽ được xem lại ở Phase sau.

## 21.3 Matcher client

Loại/defer:

```text
OsrmClient
ValhallaClient
MapMatcher
MatcherProvider
```

nếu abstraction hiện chỉ tồn tại để gọi external routing engine.

Không để worker require matcher client.

## 21.4 Profile URLs/config

Loại/defer:

```text
LT_OSRM_CAR_URL
LT_OSRM_BIKE_URL
LT_OSRM_FOOT_URL

LT_VALHALLA_URL
LT_MATCHER_ENGINE
```

## 21.5 Matcher retry

Loại/defer:

```text
MATCH_RETRY_MAX_ATTEMPTS
MATCH_RETRY_TOTAL_BUDGET_MS
MATCH_RETRY_BASE_DELAY_MS
MATCH_REQUEST_TIMEOUT_MS
```

Phase 2 không còn external matcher dependency.

## 21.6 Matcher confidence

Loại/defer:

```text
MATCH_CONFIDENCE_HIGH
MATCH_CONFIDENCE_MEDIUM
MATCH_CONFIDENCE_LOW
```

hoặc policy:

```text
confidence -> matched/raw fallback
```

Không còn matcher confidence trong Phase 2.

## 21.7 Match chunking

Loại/defer:

```text
MATCH_CHUNK_POINTS
MATCH_OVERLAP_POINTS
chunk matching
chunk stitch
chunk seam
```

Q27 trở thành future matcher scope.

## 21.8 Matcher evidence

Nếu schema migration đã tạo các bảng/columns chỉ phục vụ external matcher, review và loại nếu chưa cần cho processed GPS.

Ví dụ:

```text
matcher_input
matcher_input_hash
matcher_engine
matcher_engine_version
matcher_dataset_version
matcher_profile
matcher_confidence
matcher_raw_response
matcher_normalized_result
matcher_attempt
```

Nếu một phần evidence model hữu ích generic cho processing audit thì rename/generalize thành processing evidence, không giữ terminology matcher.

## 21.9 OSRM-specific same-second handling

Rule generic deterministic ordering có thể giữ.

Nhưng logic chỉ tồn tại vì:

```text
OSRM requires timestamps in whole seconds
```

thì loại khỏi Phase 2.

Raw GPS giữ millisecond timestamps.

Không cần collapse points chỉ để chiều OSRM.

Nếu processor cần reduction cho performance/outlier handling, đó phải là generic processing rule với lý do riêng.

## 21.10 Matched geometry terminology

Đổi:

```text
matched_route
matched_geometry
matched_part
```

thành:

```text
processed_route
processed_geometry
route_part
```

nếu output hiện thực tế chỉ dựa trên GPS.

Default source:

```text
processed_gps
```

# 22. Những thứ KHÔNG được xóa chỉ vì bỏ routing

Các phần sau vẫn thuộc Phase 2:

```text
Route Parts
progress anchors
vertex distance
Gap handling
Evidence Hole handling
Movement Segments
Trip / Stop
OPEN boundaries
Activity Revision
Daily Projection
atomic publication
worker queue
generation
fencing
late-data processing
timeline
playback
```

Đây là domain architecture của LifeTrail, không phụ thuộc OSRM.

# 23. Future routing extension point

Không cần implement routing bây giờ, nhưng có thể giữ schema extensible.

Ví dụ:

```text
route_source
```

có thể hiện tại chỉ có:

```text
processed_gps
```

về sau thêm:

```text
map_matched
```

Không để future capability làm phức tạp Phase 2 hiện tại.

# 24. Docker topology sau cleanup

Phase 2 deployment phải quay về:

```text
┌───────────────┐
│      web      │
│ Vue + Nginx   │
└───────┬───────┘
        │
        │ /api/*
        ▼
┌───────────────┐
│    server     │
│ Rust + Axum   │
│ + worker      │
└───────┬───────┘
        │
        ▼
┌───────────────┐
│ PostgreSQL    │
│ + PostGIS     │
└───────────────┘
```

Không routing container.

# 25. Web expected result

Daily Map nên có:

```text
Daily Header
Map
Route Parts
Trip / Stop presentation
Gap presentation
Evidence quality indication
Timeline
Summary
Playback controls
Course-Up / Heading-Up playback
```

Playback không phụ thuộc routing engine.

MapLibre tiếp tục render processed GPS Route Parts.

MapTiler tiếp tục chỉ cung cấp basemap/style/terrain.

# 26. Acceptance sau cleanup

Agent phải đảm bảo Phase 2 pass ít nhất:

### Data processing

- Raw GPS immutable.
- Bad point được exclude mà không mất Raw.
- Gap được phát hiện đúng.
- Evidence Hole khác Gap.
- Isolated low-quality records không tạo hole vô lý.
- Stop/Trip segmentation ổn định.
- Mode transitions không split Trip sai.
- OPEN boundaries không extrapolate.

### Revision/publication

- Late data tăng input generation.
- Replay không tăng generation.
- Candidate stale không publish.
- Worker fencing hoạt động.
- Cross-midnight revision publish atomically.
- Old good snapshot được giữ khi processing fail.

### Geometry

- Route Parts chỉ từ processed GPS.
- Không connector qua Gap/Evidence Hole.
- Server-owned progress monotonic.
- Daily clipping bảo toàn distance.

### API

- Daily View trả revision hoàn chỉnh.
- Evidence state rõ.
- Route có thể empty.
- Timeline có thể empty.
- Poor evidence không phải processing failure.

### Web

- Không render `undefined` cho evidence reason.
- Không hiện hàng chục tiny Evidence Holes nếu processor vẫn đủ evidence.
- Revision đổi thì reset playback/selection.
- Hidden tab dừng polling.
- Focus lại refetch status.

### Scale

- 1 Owner.
- 1 Device.
- 30,000 Raw GPS records/day.
- Đo processing wall time, memory, DB read latency và Daily View payload.

# 27. Cleanup order trước commit

Agent nên làm theo thứ tự này:

```text
1. Inspect git diff/status.

2. Tìm toàn bộ references:
   OSRM
   Valhalla
   matcher
   map_match
   matched_
   MATCH_
   profile URL
   routing service

3. Phân loại:
   remove now
   generalize
   keep as future-neutral extension

4. Cleanup Docker Compose.

5. Cleanup server config.

6. Cleanup database migrations/schema
   chưa commit và chỉ phục vụ matcher.

7. Cleanup worker dependencies.

8. Rename matcher-specific domain names
   sang processed GPS terminology.

9. Verify API/OpenAPI.

10. Verify Web mappings.

11. Run unit tests.

12. Run integration tests.

13. Run Phase 2 fixture suite.

14. Run 30k acceptance dataset.

15. Update Phase 2 spec/CONTEXT/ADR.

16. Review git diff lần cuối.

17. Commit.
```

# 28. Search checklist

Agent có thể bắt đầu bằng các search terms:

```text
osrm
OSRM
valhalla
Valhalla
matcher
matching
map_match
map-match
matched_
route_profile
matcher_profile
dataset_version
MATCH_
LT_OSRM
LT_VALHALLA
```

Nhưng không xóa mù quáng.

Ví dụ:

```text
historical progress anchors
route parts
route source
processing evidence
```

vẫn cần giữ nếu chúng không phụ thuộc matcher.

# 29. Phase 2 final mental model

```text
                     Raw GPS
                        │
                        ▼
               Quality Processing
                        │
             ┌──────────┴──────────┐
             │                     │
             ▼                     ▼
        usable evidence       unusable evidence
             │                     │
             │                Evidence Hole
             ▼
       Activity Segmentation
             │
       ┌─────┼─────┐
       │     │     │
      Trip  Stop   Gap
       │
       ▼
 Movement Segments
       │
       ▼
Processed GPS Route Parts
       │
       ▼
Activity Revision
       │
       ▼
Daily Projection
       │
       ▼
Timeline + Summary + Playback
       │
       ▼
     Web UI
```

Không có routing engine trong critical path.

# 30. Definition of Done

Phase 2 được coi là hoàn thiện khi:

> LifeTrail có thể ingest Raw GPS, xử lý quality/outlier, xác định Trip/Stop/Gap/Evidence Hole, tạo Movement Segments và processed GPS Route Parts, quản lý late data bằng Activity Revision + atomic Daily Publication, expose Daily View/Timeline API, và Web có thể render/playback lịch sử hoàn chỉnh mà không phụ thuộc OSRM, Valhalla hay bất kỳ routing service nào.

Routing/map matching được chuyển thành scope của phase sau và phải là **optional enhancement**, không làm thay đổi Raw GPS source of truth hoặc activity/publication architecture đã hoàn thành trong Phase 2.
