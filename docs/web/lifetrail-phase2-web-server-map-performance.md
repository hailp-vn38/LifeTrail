# LifeTrail Phase 02 — Web/Server Daily Map Payload & Rendering Optimization

## rev 2 (2026-10-07)

### Changelog rev 1 → rev 2

Rev 2 tích hợp toàn bộ 10 findings từ vòng review 2026-10-07 (đối chiếu trực tiếp với repo `hailp-vn38/LifeTrail`, branch `integration/phase-2-timeline-osrm` @ `81fb320`):

1. **§14 — chốt migration:** reproject toàn bộ historical snapshots lên schema v2; web chỉ hỗ trợ v2, gặp v1 thì fail-fast thay vì silent fallback.
2. **§9.3 — fallback khi chạm max tolerance mà vẫn vượt budget:** chấp nhận vượt + ghi log/metric + provenance, cấm drop part.
3. **§20/§21/§31/§41 — triple-check** `device_id`/`date`/`manifest_version` trước khi khởi tạo PlaybackController (chống race khi user đổi ngày giữa fetch).
4. **§34 — provenance ghi effective tolerance** sau escalation, không phải base config.
5. **§9.6 (mới) — display coordinates làm tròn 6 decimals** (~0.1 m, visually lossless).
6. **§6.1/§18 — làm rõ semantics:** `distance_m` = full part distance (canonical, không đổi); `visible_distance_m` = phần clipped của ngày.
7. **§8 — pinned manifest contract 200/410, bỏ 409** cho playback + retention policy; **bounded retry** (3 vòng refresh) cho refetch loop.
8. **§11 — giữ nguyên `project_part()` hiện tại làm playback path** (rename, không rewrite); display là wrapper strip + simplify → clipping math không thể drift.
9. **§28 — thêm time-based acceptance target.**
10. **Tests bổ sung:** S7 (rounding), I5 (historical reprojection), W7 (triple-check race), W8 (bounded manifest retry).
11. **§19.0 (mới) — layout guardrail:** update web không được phá layout Daily page; state mới của playback/export phải non-reflowing.

---

## 0. Status / baseline

Tài liệu này được viết sau khi kiểm tra repository `hailp-vn38/LifeTrail` trên GitHub.

Branch người dùng yêu cầu:

```text
integration/phase2
```

không tồn tại trên remote GitHub tại thời điểm kiểm tra.

Integration branch Phase 02 khả dụng hiện tại là:

```text
integration/phase-2-timeline-osrm
HEAD: 81fb320732f2041058fa9a63d5e8906865d394a3
```

(Đã kiểm chứng lại 2026-10-07 ~12:20 qua public `git ls-remote`: branch tồn tại, hash khớp chính xác.)

Nếu developer/agent đang làm trên branch local `integration/phase2` chưa push, phải chạy diff với `integration/phase-2-timeline-osrm` trước khi áp dụng tài liệu này.

Không áp patch trực tiếp lên `main` nếu chưa xác nhận branch làm việc.

---

# 1. Mục tiêu

Tối ưu Daily Map của LifeTrail để:

- giảm thời gian tải Daily View;
- giảm kích thước JSON response;
- giảm JSON parse/memory trên browser;
- giảm số vertex MapLibre phải xử lý ở initial render;
- giữ nguyên Raw GPS immutable;
- giữ nguyên canonical processed route/progress trên server;
- không làm sai `distance_m`, Timeline, Stop/Trip/Gap/Evidence Hole;
- không làm playback mất tính chính xác về thời gian;
- không làm Web recompute distance;
- không đưa map-matching/OSRM/Valhalla trở lại Phase 02.

Mục tiêu kiến trúc:

```text
Raw GPS
   ↓
Canonical processing
   ↓
Activity Revision
   ├── canonical route/progress      ← server authority
   ├── Daily display projection      ← lightweight
   └── Playback projection           ← lazy/on demand
```

---

# 2. Tình trạng code hiện tại

## 2.1 Server RoutePart đang giữ dữ liệu canonical đầy đủ

File:

```text
server/src/processing/route_parts.rs
```

`RoutePart` hiện chứa:

```text
geometry.coordinates
vertex_distance_m
progress_anchors
```

`geometry.coordinates` được tạo trực tiếp từ usable GPS observations.

`vertex_distance_m` có số phần tử tương ứng với geometry vertices.

`progress_anchors` được build 1:1 theo từng observation (zip với `vertex_distance_m`), nên cũng ~N entries.

Kết quả: một chuỗi GPS được biểu diễn nhiều lần trong cùng object.

Ví dụ với N observations:

```text
coordinates          ~ N entries
vertex_distance_m    ~ N entries
progress_anchors     ~ N entries
```

Đây là dữ liệu đúng về mặt domain nhưng không phù hợp để gửi toàn bộ trong initial Daily Map response.

## 2.2 Daily projection hiện copy toàn bộ Route Part vào snapshot

File:

```text
server/src/processing/progress_projection.rs
```

`project_part()` hiện clip part theo ngày, reconstruct boundary coordinate bằng nội suy tuyến tính giữa hai anchors, rebase progress, rồi emit Daily Route Part gồm:

```text
geometry
vertex_distance_m
progress_anchors
visible_from_at
visible_until_at
visible_distance_m
...
```

Sau đó:

```text
server/src/processing/reprojection.rs
```

đưa toàn bộ `route_parts` vào body của Daily Snapshot. Daily View mặc định (`GET` không kèm `?view=`) serve trực tiếp snapshot JSON từ DB.

Do đó browser phải tải playback/canonical data ngay cả khi user chỉ muốn nhìn overview map.

## 2.3 Timeline còn chứa provenance ID arrays không cần cho UI

OpenAPI `DailyActivityBoundaries` hiện yêu cầu:

```text
source_record_ids
```

Trong processing, Trip/Stop giữ danh sách Raw GPS record IDs (`Vec<i64>`).

Thông tin này hữu ích cho audit/debug server nhưng không cần cho normal Daily Map UI.

Đã kiểm chứng: web không dùng `source_record_ids` trong bất kỳ logic nào — chỉ xuất hiện trong test fixtures và generated types. Việc bỏ nó khỏi normal response không vỡ UI.

Với dataset lớn, `source_record_ids` có thể tạo payload đáng kể dù MapLibre không dùng trường này.

Normal Web UI đã có:

```text
source_record_count
```

nên không cần toàn bộ ID list.

## 2.4 Web render Route Parts bằng toàn bộ geometry

File:

```text
web/src/map/route-parts.ts
```

Hiện tại:

```text
DailyView.route_parts
      ↓
part.geometry.coordinates
      ↓
GeoJSON FeatureCollection
      ↓
MapLibre source
```

MapLibre không tạo DOM marker cho từng point — điều này đúng.

Tuy nhiên MapLibre vẫn phải:

- nhận toàn bộ coordinates;
- parse GeoJSON;
- upload geometry;
- tessellate line;
- giữ source data trong memory.

Do đó không render DOM marker không đồng nghĩa payload/render đã nhẹ.

## 2.5 Playback hiện phụ thuộc cùng RoutePart object

File:

```text
web/src/map/route-playback/parts.ts
```

Playback dùng:

```text
geometry.coordinates
vertex_distance_m
progress_anchors
```

để map historical timestamp → progress → coordinate, và **validate** `vertex_distance_m.length === coordinates.length`.

Vì vậy không được chỉ xóa `progress_anchors` hoặc simplify `geometry` trong object hiện tại mà không đổi contract playback. Display geometry không bao giờ được đưa vào playback path — validation 1:1 sẽ fail loudly nếu ai đó làm nhầm, đó là guardrail có sẵn cần giữ.

## 2.6 Daily View đã có Raw mode riêng

File:

```text
web/src/api/queries/daily-view.query.ts
```

Hiện có:

```ts
getDailyView(deviceId, date, raw = false)
```

và server hỗ trợ (`server/src/daily_view.rs`, `ViewMode::Raw`):

```text
?view=raw
```

Normal processed Daily View không cần dùng Raw mode.

UI production không được load `view=raw` mặc định.

---

# 3. Root cause

Bottleneck không nên được giải quyết bằng một thay đổi duy nhất ở MapLibre.

Hiện pipeline đang dùng một representation cho nhiều mục đích:

```text
Canonical processing data
        =
Daily API data
        =
Map rendering data
        =
Playback data
```

Đây là coupling cần loại bỏ.

Thiết kế mới:

```text
Canonical Route Part
        │
        ├── Display Route Part
        │      lightweight/simplified
        │
        └── Playback Route Part
               canonical temporal progress
```

---

# 4. Invariants không được phá

## 4.1 Raw GPS vẫn immutable

Không update/xóa/simplify `gps_points` để tối ưu UI.

```text
gps_points = source of truth observations
```

## 4.2 Activity Revision vẫn canonical

Không thay canonical activity geometry bằng display-simplified geometry.

Canonical Route Parts vẫn phục vụ:

- historical audit;
- distance/progress authority;
- playback generation;
- re-projection;
- future algorithm changes.

## 4.3 Display simplification không thay distance

Không làm:

```text
display_distance = haversine(simplified_geometry)
```

Daily/Trip distance vẫn lấy từ server-owned canonical progress:

```text
visible_distance_m
```

## 4.4 Không simplify xuyên boundary

Không simplify nối qua:

- Route Part boundary;
- GPS Gap;
- Evidence Hole;
- disconnected geometry.

Mỗi Route Part được simplify độc lập.

## 4.5 Playback không được suy ra time từ vertex index

Playback vẫn phải dùng historical temporal evidence.

Không làm:

```text
time = vertex_index / vertex_count
```

Không làm constant-speed playback nếu canonical anchors cho biết tốc độ thay đổi.

---

# 5. Target architecture

```text
                    PostgreSQL
                        │
                        ▼
               Immutable Raw GPS
                        │
                        ▼
                Phase 02 processor
                        │
                        ▼
               Activity Revision
              canonical route data
                        │
             ┌──────────┴──────────┐
             │                     │
             ▼                     ▼
     Daily display projection   Playback projection
             │                     │
       simplify geometry         canonical anchors
       round 6 decimals          canonical timestamps
             │                     │
             ▼                     ▼
     GET Daily View             GET Playback
             │                     │
             ▼                     ▼
       initial MapLibre        lazy on Play
```

Normal page load chỉ cần nhánh bên trái.

---

# 6. API contract đề xuất

## 6.1 Daily View mặc định phải lightweight

Giữ endpoint:

```text
GET /api/v1/devices/{deviceId}/days/{date}
```

Nhưng `route_parts` public cho Daily View phải trở thành display-oriented model.

Không gửi mặc định:

```text
canonical geometry
vertex_distance_m
progress_anchors
source_record_ids
```

Đề xuất schema:

```json
{
  "id": "...",
  "kind": "route_part",
  "trip_id": "...",
  "movement_segment_id": "...",
  "source": "processed_gps",
  "mode": "car",
  "classification_confidence": 0.91,

  "observed_from_at": "...",
  "observed_until_at": "...",
  "visible_from_at": "...",
  "visible_until_at": "...",

  "distance_m": 12034.2,
  "visible_distance_m": 11820.4,

  "source_record_count": 4200,

  "display_geometry": {
    "type": "LineString",
    "coordinates": []
  }
}
```

Semantics bắt buộc ghi trong OpenAPI:

- `distance_m`: **full part distance** (canonical, không đổi, lấy từ Activity Revision).
- `visible_distance_m`: phần distance thuộc về ngày đang xem (day-clipped, server-owned).
- `display_geometry`: visualization-only, coordinates đã simplify + làm tròn 6 decimals. Không dùng cho distance/playback.

Có thể giữ tên `geometry` nếu muốn giảm số thay đổi Web, nhưng phải ghi rõ contract rằng geometry trong Daily View là **display geometry**, không phải canonical progress geometry.

Khuyến nghị rõ ràng hơn:

```text
display_geometry
```

để tránh agent hoặc future code dùng nhầm nó cho distance/playback. Tên tường minh cũng ngăn lỗi nguy hiểm: nếu ai đó đưa nhầm display geometry vào playback path, validation 1:1 ở `web/src/map/route-playback/parts.ts` sẽ fail — nhưng với tên `geometry` thì khả năng nhầm lẫn cao hơn ngay từ đầu.

---

# 7. Playback endpoint riêng

Thêm endpoint:

```text
GET /api/v1/devices/{deviceId}/days/{date}/playback
```

Response chỉ chứa dữ liệu cần cho playback.

Có hai phương án hợp lệ.

## Option A — giữ model playback hiện tại

Response:

```text
geometry
vertex_distance_m
progress_anchors
```

Ưu điểm:

- ít thay đổi thuật toán Web;
- reuse `buildRoutePartPlaybackInput()`;
- canonical semantics giữ nguyên.

Nhược điểm:

- payload playback vẫn lớn, nhưng chỉ tải khi user Play.

Đây là option khuyến nghị cho Phase 02 hiện tại.

## Option B — playback anchors có coordinate

Ví dụ:

```json
{
  "at": "...",
  "distance_m": 123.4,
  "coordinate": [106.7, 10.8]
}
```

Có thể bỏ `vertex_distance_m` + canonical geometry khỏi API playback nếu server đã project coordinate cho từng anchor.

Option này giảm coupling hơn nhưng thay đổi nhiều code.

Không cần thực hiện trong patch đầu tiên.

---

# 8. Publication consistency giữa Daily View và Playback

Không được mix playback của revision cũ với Daily View mới.

Daily View hiện có provenance:

```text
provenance.manifest_version
```

Playback response phải có ít nhất:

```json
{
  "manifest_version": "uuid",
  "route_parts": []
}
```

Production Web luôn gọi playback ở dạng pinned theo manifest của Daily View:

```text
Daily View M1
    ↓
GET playback?manifest_version=M1
```

Nếu response trả về manifest khác M1 (khi không pin) thì vẫn là mismatch và phải
xử lý như dưới. Không merge dữ liệu giữa hai manifest.

Endpoint playback nhận optional pinned query:

```text
?manifest_version=<uuid>
```

Server behavior (contract đã chốt, **không có `409`**):

- không truyền `manifest_version` → `200` với current publication;
- pinned manifest còn tồn tại (dù đã bị supersede) → `200` với đúng pinned manifest;
- pinned manifest không còn tồn tại → `410 Gone`.

Server không có đủ thông tin để kết luận `publication_changed` cho request
unpinned, nên **không trả `409` cho unpinned playback**. `409` bị loại bỏ khỏi
contract playback.

Web xử lý `410` (pinned manifest đã mất):

```text
pause playback
clear playback cache
refetch Daily View
→ nhận manifest M2
request playback lại pinned M2 (bounded retry, tối đa 3 vòng refresh)
```

Nếu sau 3 vòng refresh vẫn `410` (publication đổi liên tục), dừng và hiển thị
lỗi "dữ liệu đang cập nhật, thử lại sau" — không loop vô hạn.

Retention policy Phase 02: Daily Snapshots không bị xóa → pinned manifest luôn servable. Nếu sau này có pruning policy, phải version chính sách đó; `410` là contract cho trường hợp manifest đã mất.

Phase 02 nên ưu tiên behavior đơn giản và deterministic.

---

# 9. Display geometry simplification

## 9.1 Algorithm

Dùng deterministic polyline simplification, ví dụ:

```text
Ramer-Douglas-Peucker (RDP)
```

hoặc thuật toán equivalent có tolerance theo mét.

Không dùng trực tiếp degree delta như meter.

Tính distance/error trong metric space.

Implementation có thể:

- dùng local projection/equirectangular approximation phù hợp route nhỏ (center theo mean latitude của từng part);
- hoặc distance-to-geodesic segment helper;
- hoặc PostGIS nếu canonical data được đưa vào geometry column trong tương lai.

Với code Phase 02 hiện tại đang giữ geometry trong Rust/JSON, implementation Rust thuần là lựa chọn đơn giản hơn.

## 9.2 Default tolerance

Baseline:

```text
Daily overview tolerance = 10 m
```

Cho phép config:

```text
LT_DAILY_DISPLAY_SIMPLIFY_TOLERANCE_M=10
LT_DAILY_DISPLAY_MAX_VERTICES=3000
```

Không coi `10 m` là protocol invariant.

## 9.3 Vertex budget

Ngoài tolerance, có target budget:

```text
<= 3000 display vertices / Daily View
```

Không cắt ngẫu nhiên hoặc `every Nth point`.

Nếu tổng vertices sau 10 m vẫn quá lớn:

```text
increase tolerance deterministically
```

ví dụ:

```text
10 m
15 m
20 m
30 m
...
```

cho tới khi đạt budget hoặc đạt configured maximum tolerance.

Không merge Route Parts để đạt budget.

**Fallback khi đã chạm maximum tolerance mà vẫn vượt budget:** chấp nhận vượt budget, ghi `display_vertices_over_budget: true` vào snapshot provenance, và emit server log/metric. Không drop part, không merge parts (nhất quán với test S5).

## 9.4 Endpoint preservation

Mỗi simplified part bắt buộc giữ:

```text
first coordinate
last coordinate
```

(RDP giữ endpoints theo construction; assert显式 trong test.)

Nếu part không thể giữ >= 2 distinct coordinates:

- không fabricate point;
- không duplicate coordinate giả;
- review upstream part validity.

## 9.5 Boundary preservation

Không simplify qua:

```text
Part A
GPS Gap
Part B
```

thành:

```text
A ---------------- B
```

Mỗi Part simplify riêng.

## 9.6 Coordinate rounding (mới trong rev 2)

Sau simplify, làm tròn display coordinates về **6 decimals** (~0.11 m tại equator, ~0.10 m tại Việt Nam — visually lossless).

Sai số làm tròn ≤ ~0.06 m/point, không đáng kể so với tolerance 10 m.

An toàn vì:

- distance lấy từ canonical progress, không từ display geometry;
- playback dùng canonical geometry riêng.

Rounding giảm thêm ~30–40% bytes của phần coordinates (f64 đầy đủ serialize tới ~17 significant digits).

---

# 10. Server implementation plan

## 10.1 Không thay `route_parts::build()` canonical semantics

File:

```text
server/src/processing/route_parts.rs
```

Canonical Route Part hiện tại có thể giữ nguyên trong Activity Revision.

Không đưa display simplification vào reducer nếu không cần.

Lý do:

- display là projection concern;
- thay display tolerance không nên làm thay Activity Revision;
- không nên bump activity reducer chỉ vì UI optimization.

## 10.2 Thêm module display projection

Suggested file:

```text
server/src/processing/display_geometry.rs
```

Responsibility:

```text
canonical daily-clipped geometry
        ↓
metric simplify (RDP)
        ↓
round 6 decimals
        ↓
display geometry
```

Suggested API:

```rust
pub(super) fn simplify_part(
    coordinates: &[[f64; 2]],
    tolerance_m: f64,
) -> Vec<[f64; 2]>
```

Nếu cần global Daily vertex budget, thêm coordinator ở reprojection layer.

Không mutate canonical source.

## 10.3 Thay Daily Snapshot representation

File chính:

```text
server/src/processing/reprojection.rs
```

Hiện tại:

```text
project_part(part, day)
    ↓
full daily RoutePart
```

Thay thành:

```text
project_part_for_playback(part, day)   # canonical, giữ nguyên logic
        ↓
strip vertex_distance_m, progress_anchors
        ↓
simplify display geometry (RDP + round 6 decimals)
        ↓
store lightweight Daily RoutePart (display_geometry)
```

Daily snapshot không chứa:

```text
vertex_distance_m
progress_anchors
```

vì playback đã có endpoint riêng.

---

# 11. `progress_projection.rs` refactor

`project_part()` hiện tại vừa:

- clip part theo ngày;
- reconstruct boundary coordinate;
- rebase progress;
- emit full playback-ready part.

Refactor theo hướng **không rewrite clipping math**:

```text
project_part_for_playback(value, day)
    = project_part() hiện tại, chỉ rename — giữ nguyên logic từng dòng
```

và:

```text
project_part_for_display(value, day)
    1. let canonical = project_part_for_playback(value, day)?;   # reuse, không duplicate
    2. strip value["vertex_distance_m"], value["progress_anchors"]
    3. simplify value["geometry"]["coordinates"] → rename thành "display_geometry"
    4. round 6 decimals
```

Không duplicate clipping math ở hai nơi — display path là wrapper trên canonical path, nên hai path không thể drift khỏi nhau.

Canonical clipping vẫn phải giữ:

```text
visible_from_at
visible_until_at
visible_distance_m
```

Display projection reuse scalar metadata nhưng simplify geometry sau clipping (trên coordinates đã gồm boundary reconstructed points).

---

# 12. Không public `source_record_ids` trong Daily View mặc định

Server vẫn giữ `source_record_ids` trong immutable Activity Revision.

Nhưng Daily View normal response chỉ cần:

```text
source_record_count
```

OpenAPI DailyActivity model phải bỏ requirement:

```text
source_record_ids
```

khỏi normal response. Đã kiểm chứng web không dùng trường này trong logic (chỉ fixtures + generated types) nên thay đổi an toàn.

Nếu cần debug/audit sau này:

```text
GET .../activities/{id}/evidence
```

hoặc debug-only endpoint có thể trả IDs.

Không gửi hàng nghìn/hàng chục nghìn integer IDs cho mỗi page load chỉ vì provenance nội bộ.

---

# 13. Projection schema version

Hiện snapshot provenance có:

```text
projection_schema_version = 1
```

Sau thay đổi response/snapshot shape:

```text
projection_schema_version = 2
```

Không bump `REDUCER_VERSION` chỉ vì display payload thay đổi nếu canonical activity semantics không đổi.

Nếu reducer logic không đổi:

```text
Activity Revision: giữ nguyên
Daily Projection schema: bump
```

Đây là distinction quan trọng.

---

# 14. Existing snapshots / migration (chốt trong rev 2)

Daily Snapshots là immutable.

Không update JSON body của snapshot cũ.

**Quyết định migration (rev 2, đã sửa): reproject toàn bộ historical publications lên schema v2**, thay vì để v1/v2 lẫn lộn:

- Raw GPS immutable → reproject từ canonical Activity Revisions là an toàn, không mất dữ liệu;
- personal app, không lo scale của reproject toàn bộ;
- web chỉ cần hỗ trợ một schema → giảm complexity và nguy cơ bug dual-shape.

**Ràng buộc immutable (quan trọng):** reproject **không** update JSON body của
snapshot v1 và **không** xóa row v1. Mỗi day được reproject bằng cách tạo một
**Daily Snapshot v2 mới** rồi switch `daily_publications` sang v2 atomically.
Do đó acceptance của migration là **"không còn `daily_publications` trỏ tới
snapshot v1"**, **không phải** "không còn row snapshot v1 trong DB". Row v1 cũ
vẫn tồn tại và đọc nguyên vẹn.

Thực hiện qua **projection-only requeue seam** (không tăng `input_generation` vì Raw GPS không đổi; không bắt buộc rerun quality/Trip/Stop reducer; reuse `active_manifest_id`). Seam này **tách khỏi timezone-generation seam**.

Nếu current worker chưa có projection-version requeue seam, agent phải thêm seam projection-only thay vì abuse timezone generation hoặc Raw generation. Migration mới dùng số **`0016_daily_display_projection.sql`**; **không sửa `0015`** đã nằm trong history.

**Web behavior:** web yêu cầu `projection_schema_version == 2`. Nếu gặp snapshot v1 (migration chưa hoàn tất hoặc lỗi), **fail-fast với error state rõ ràng** ("dữ liệu ngày này cần reproject"), không silent fallback về canonical geometry — silent fallback sẽ âm thầm tải lại payload nặng, đúng thứ tài liệu này muốn loại bỏ.

---

# 15. Playback endpoint implementation

Suggested server location:

```text
server/src/daily_playback.rs
```

hoặc dưới processing read model:

```text
server/src/processing/playback.rs
```

Endpoint flow:

```text
request device/date (+ optional ?manifest_version=)
   ↓
resolve current/pinned Daily publication
   ↓
read manifest_version
   ↓
read immutable Activity Revisions referenced by manifest
   ↓
collect canonical Route Parts overlapping day
   ↓
reuse project_part_for_playback (canonical clipping/progress projection)
   ↓
return playback route parts
```

Không rerun:

```text
quality
stop detection
trip segmentation
mode classification
```

Playback là projection của published immutable activity.

Lưu ý: playback endpoint đọc từ **Activity Revisions** (qua manifest), không bao giờ đọc từ Daily Snapshot v2 — vì snapshot v2 không còn chứa canonical data.

---

# 16. Playback response example

Phase 02 recommended initial contract:

```json
{
  "device_id": "uuid",
  "date": "2026-10-05",
  "timezone": "Asia/Ho_Chi_Minh",
  "manifest_version": "uuid",
  "projection_schema_version": 2,
  "route_parts": [
    {
      "id": "...",
      "trip_id": "...",
      "movement_segment_id": "...",
      "visible_from_at": "...",
      "visible_until_at": "...",
      "geometry": {
        "type": "LineString",
        "coordinates": []
      },
      "vertex_distance_m": [],
      "progress_anchors": []
    }
  ]
}
```

> Ghi chú rev 2: granularity của playback payload (`vertex_distance_m` +
> `progress_anchors` hay anchor-projected coordinates) chốt ở Ticket 01 —
> xem §11. Bất kể chọn dạng nào, playback payload **luôn là canonical**, không
> bao giờ chứa `display_geometry`.

Không cần duplicate toàn bộ Timeline trong playback response.

Gap/Evidence Hole interruption data:

- giữ trong Daily View lightweight payload (rất nhỏ và UI Timeline cũng cần).

---

# 17. Raw GPS mode

Normal Daily Map không request:

```text
?view=raw
```

`Raw GPS` nên là debug/developer feature.

Nếu UI hiện có badge/toggle `Raw GPS`, review behavior để đảm bảo nó không phải default source cho normal Daily Map.

Recommended future bounded debug API:

```text
GET /api/v1/devices/{deviceId}/gps
    ?from=...
    &until=...
    &limit=5000
```

Hoặc mở rộng raw view với range/limit.

Không cần block initial optimization nếu debug endpoint chưa làm ngay.

---

# 18. OpenAPI changes

File:

```text
protocol/openapi/lifetrail-v1.yaml
```

Agent phải:

1. tạo schema display Route Part (`display_geometry`);
2. bỏ playback-only arrays khỏi default Daily View Route Part;
3. bỏ `source_record_ids` khỏi Daily View public activity schema;
4. thêm Playback response schemas;
5. thêm playback endpoint (+ `?manifest_version`, document 200/410, không có 409);
6. giữ `visible_distance_m` server-owned;
7. document `display_geometry` là visualization-only, coordinates rounded 6 decimals;
8. document Web không được tính distance từ simplified geometry;
9. document semantics: `distance_m` = full part canonical distance; `visible_distance_m` = day-clipped portion.

Sau đó regenerate:

```text
web/src/api/generated/lifetrail-v1.d.ts
```

Không sửa generated types bằng tay.

---

# 19. Web implementation plan

## 19.0 Layout guardrail — không phá layout Daily page

Mọi thay đổi web trong phase này **không được phá layout UI hiện có của Daily
page**. Cụ thể phải giữ:

- workspace grid 2 cột (`map column` + `timeline column`) và breakpoint hiện có;
- header grid 3 vùng (`DailyDateContext` | `DailyMapToolbar` | `DailyMapActions`)
  và các breakpoint 1399px/767px;
- vị trí toggle `Raw GPS` / `Xem hoạt động` (đây là điều kiện review §17, không
  phải chỗ để đổi layout);
- các state `loading` / `empty` / `not-found` / `error` hiện có render trong cùng
  khung layout, không đẩy cột hay đổi chiều cao đột ngột.

State mới do lazy playback / export thêm vào phải **non-reflowing**: hiển thị
inline hoặc overlay trong không gian đã dành sẵn, không thêm block full-width
làm xô lệch grid. `DailyMapPage.test.ts` hiện có phải tiếp tục pass.

Đổi layout là hành vi bị cấm trong scope phase này; muốn đổi phải là một quyết
định riêng, ngoài các ticket dưới đây.

## 19.1 Daily Map chỉ dùng display geometry

File:

```text
web/src/map/route-parts.ts
```

Thay:

```text
part.geometry.coordinates
```

bằng:

```text
part.display_geometry.coordinates
```

MapLibre source chỉ nhận fields cần render/select:

```json
{
  "eventId": "...",
  "partId": "..."
}
```

Không copy full activity object vào GeoJSON properties.

## 19.2 `fitRoute()` dùng display geometry

File:

```text
web/src/components/RouteMap.vue
```

Initial bounds phải được tính từ:

```text
display_geometry
```

Không cần canonical playback geometry để fit initial map. Sai số bounds do simplify (~10 m) không đáng kể cho fit.

## 19.3 Playback data phải lazy load

Thêm query:

```text
web/src/api/queries/daily-playback.query.ts
```

Query phải **disabled mặc định** (`enabled: false`).

Không request playback data trong initial Daily Map render.

Fetch khi:

```text
user presses Play
```

Có thể thêm prefetch sau này khi:

```text
user hover Play
```

nhưng không prefetch ngay trong `onMounted()`.

Mục tiêu là initial map interactive trước khi tải heavy playback payload.

---

# 20. Playback UX khi data chưa tải

Khi user nhấn Play lần đầu:

```text
Play click
   ↓
playback query loading (enabled)
   ↓
show small loading state on PlaybackBar
   ↓
data validated against manifest_version
   ↓
triple-check: response.device_id/date/manifest_version
   còn khớp selection hiện tại?
   ↓ no → discard response, không init controller
   ↓ yes
initialize PlaybackController
   ↓
play
```

Không freeze toàn map trong lúc fetch.

Map vẫn pan/zoom được.

Triple-check là bắt buộc vì user có thể đổi date/device trong lúc fetch — init controller với data của ngày cũ là bug khó phát hiện.

---

# 21. Refactor `RouteMap.vue`

Hiện component initialize controller trong `onMounted()` từ `dailyView.route_parts`.

Sau update:

```text
onMounted
   ↓
create MapLibre
   ↓
render display route
   ↓
NO playback controller yet
```

Khi playback query success **và triple-check pass**:

```text
buildRoutePartPlaybackInput(playback.route_parts, ...)
   ↓
create/reset PlaybackController
```

Phải dispose controller cũ trước khi replace.

---

# 22. Publication change behavior

Khi Daily View publication/manifest thay đổi:

```text
pause playback
reset playback
clear selected event if required by existing Phase 02 contract
remove cached playback for old manifest
refetch Daily View
```

Không tiếp tục chạy old playback path trên new Timeline.

Query key playback nên gồm:

```text
deviceId
date
manifestVersion
```

Ví dụ:

```ts
queryKeys.dailyPlayback(deviceId, date, manifestVersion)
```

---

# 23. Map rendering rules

Normal mode:

```text
Display Route Parts
Stops
Timeline event dots
selection highlight
```

Không render:

```text
one CircleLayer feature per Raw GPS observation
```

trừ debug mode explicit.

Không tạo DOM markers cho raw GPS points.

---

# 24. Selected Trip

Ban đầu selected Trip vẫn có thể dùng display geometry để:

```text
highlight
fit bounds
```

Không cần fetch high-resolution route chỉ vì user click Timeline item.

Nếu sau này cần inspection mức đường nhỏ:

```text
selected Trip detail endpoint
```

có thể trả geometry tolerance thấp hơn.

Không cần thêm vào patch đầu tiên.

---

# 25. Recommended display levels

Phase 02 patch đầu tiên chỉ cần một Daily display level:

```text
~10 m tolerance
```

Không cần implement dynamic zoom-dependent server requests ngay.

Future:

```text
overview       10–20 m
selected trip   3–5 m
raw/debug       0 m
```

Nhưng tránh premature complexity.

---

# 26. Web memory rules

Không deep-clone Daily View lớn.

Không đưa raw route arrays vào Pinia.

TanStack Query giữ server state.

MapLibre source chỉ nhận prepared FeatureCollection nhỏ.

Playback payload chỉ tồn tại khi playback feature được dùng/cached.

Khi đổi device/date:

- dispose controller;
- clear playback points;
- let query cache lifecycle release stale heavy data theo configured cache policy (giữ `gcTime` bounded cho playback queries).

---

# 27. Performance instrumentation

Server test/benchmark phải đo:

```text
raw_point_count
canonical_route_vertex_count
display_route_vertex_count
daily_json_bytes
playback_json_bytes
display_vertices_over_budget (bool)
effective_tolerance_m
```

Web development measurement:

```text
Daily View transfer size
JSON parse duration
Map interactive time (MapLibre ready + route rendered)
Route source creation duration
Playback lazy fetch duration
```

Không cần gửi telemetry location ra third-party analytics.

---

# 28. Acceptance targets

Các giá trị sau là engineering target, không phải protocol invariant.

## Initial Daily View

Dataset realistic 24h:

```text
display vertices <= 3000/day target
```

và:

```text
Daily View JSON giảm >= 70% so với payload hiện tại
```

trên fixture dense tương đương.

Nếu baseline hiện tại rất lớn, target tốt hơn là >= 80%.

(Ước tính sanity check: mỗi point hiện tại ~105–115 bytes (coordinates ~38 + vertex_distance_m ~13 + progress_anchor ~60). Ngày dense 50k points ≈ 5–6 MB chỉ route parts. Sau tối ưu ≈ 150–200 KB → giảm ~95%+ phần route. Target 70% là conservative.)

## Render time (mới trong rev 2)

Đo trên cùng máy tham chiếu, fixture dense 24h:

```text
JSON parse duration:      giảm >= 70% so baseline
Map interactive time:     giảm >= 50% so baseline
                          (MapLibre ready + route rendered)
```

Calibrate số tuyệt đối từ baseline ở Step 1 (§39) — không hard-code ms vì phụ thuộc máy đo.

## Raw/canonical data

```text
Raw GPS record count: unchanged
Activity Revision canonical geometry: unchanged
canonical distance: unchanged
```

## Route quality

Visual Daily overview phải giữ:

- major turns;
- route topology theo từng Part;
- start/end;
- no connector across Gap/Hole.

Không yêu cầu display line đi qua mọi GPS jitter point.

## Distance

Sau simplification:

```text
summary.distance_m
trip distance
segment distance
visible_distance_m
```

phải giữ canonical value trước optimization.

Sai khác do simplified geometry không được propagate vào metric.

## Playback

Playback phải:

- dùng canonical temporal data;
- preserve timestamp ordering;
- preserve Gap/Hole interruptions;
- Course-Up/Heading-Up vẫn hoạt động;
- seek vẫn binary-search theo timestamps;
- không phụ thuộc display vertex count.

---

# 29. Server unit tests

## Test S1 — simplify straight line

Input nhiều collinear points.

Expected:

```text
first + last preserved
intermediate points largely removed
```

## Test S2 — preserve turn

Input có góc rẽ lớn.

Expected:

```text
turn vertex preserved within tolerance
```

## Test S3 — independent Route Parts

Input:

```text
Part A
Gap
Part B
```

Expected:

- A simplify độc lập;
- B simplify độc lập;
- không có connector A → B.

## Test S4 — canonical distance unchanged

Sau display simplification:

```text
visible_distance_m_before == visible_distance_m_after
```

trong numerical tolerance.

## Test S5 — vertex budget

Dense route > configured max vertices.

Expected:

```text
display vertices <= configured budget
```

mà không drop entire part.

## Test S6 — projection schema

New Daily Snapshot:

```text
projection_schema_version == 2
```

và không mutate snapshot v1.

## Test S7 — coordinate rounding (mới trong rev 2)

Sau display projection:

```text
mỗi coordinate có <= 6 decimals
sai số vs canonical coordinate <= 0.15 m/point
```

---

# 30. Server integration tests

## Test I1 — dense realistic day

Dùng Phase 02 realistic fixture.

Compare:

```text
before JSON bytes
after JSON bytes
```

Assert reduction target.

## Test I2 — timeline provenance payload

Daily response không chứa massive:

```text
source_record_ids
```

Normal UI vẫn có:

```text
source_record_count
```

## Test I3 — playback consistency

Daily View manifest `M1`.

Playback response phải:

```text
manifest_version = M1
```

## Test I4 — late data race

Flow:

```text
Daily View M1
late batch
publish M2
playback request pinned M1/M2
```

Expected behavior phải deterministic theo contract:

- pinned M1 (còn tồn tại) → 200 với M1; hoặc
- không pin → 200 với current publication M2. Server **không** trả `409`; việc
  playback M2 lệch Timeline M1 là mismatch phải xử lý ở web (§8), không encode
  thành HTTP status ở server.

Không mix route from M1 với Timeline M2.

## Test I5 — historical reprojection (mới trong rev 2)

Sau migration:

```text
mọi Daily Snapshot mà daily_publications trỏ tới đều projection_schema_version == 2
canonical distance_m / visible_distance_m không đổi vs trước reproject
không còn daily_publications trỏ tới snapshot v1 (row v1 cũ vẫn giữ)
```

---

# 31. Web tests

## Test W1 — initial page does not fetch playback

Mount Daily Map.

Assert:

```text
Daily View request = yes
Playback request = no
```

## Test W2 — map uses display geometry

Assert MapLibre route source coordinates đến từ:

```text
display_geometry
```

không phải playback geometry.

## Test W3 — Play triggers lazy request

First click Play:

```text
one playback fetch
controller initializes after success
```

## Test W4 — manifest mismatch

Nếu playback response manifest khác current Daily View:

```text
do not play
invalidate/refetch
```

## Test W5 — route highlight

Timeline Trip selection vẫn highlight/fit display geometry mà không cần playback fetch.

## Test W6 — processing publication update

Khi published revision/manifest thay đổi:

```text
pause
reset
clear old playback
refetch daily
```

## Test W7 — triple-check race (mới trong rev 2)

User bấm Play ngày A, chuyển sang ngày B trước khi playback response về.

Assert:

```text
controller KHÔNG init với data ngày A
không có playback path của A hiển thị trên map ngày B
```

## Test W8 — bounded manifest refresh (mới trong rev 2)

Giả lập publication đổi liên tục (mỗi refetch lại ra manifest mới, playback pinned trả `410`).

Assert:

```text
dừng sau 3 vòng refresh
hiển thị error state rõ ràng
không loop vô hạn
```

---

# 32. Files expected to change

Server likely:

```text
server/src/processing/reprojection.rs
server/src/processing/progress_projection.rs
server/src/processing/display_geometry.rs        # new
server/src/processing/read.rs                    # depending endpoint design
server/src/daily_view.rs
server/src/daily_playback.rs                     # suggested new
server/src/app.rs                                # route registration if needed
protocol/openapi/lifetrail-v1.yaml
server/tests/*
```

Web likely:

```text
web/src/api/queries/daily-view.query.ts
web/src/api/queries/daily-playback.query.ts       # new
web/src/api/query-keys.ts
web/src/api/generated/lifetrail-v1.d.ts           # generated
web/src/components/RouteMap.vue
web/src/map/route-parts.ts
web/src/map/route-playback/parts.ts                # minimal changes if Option A
web/src/features/activity/model.ts
web/src/**/*.test.ts
```

Agent phải verify actual paths trên branch local trước khi edit.

---

# 33. Migration / data model

Không bắt buộc migration database nếu:

- canonical Activity Revision JSON giữ nguyên;
- new Daily Snapshot schema được tạo bằng existing tables;
- playback được project từ existing immutable revisions.

Có thể cần migration/config nếu muốn persist:

```text
display tolerance
projection schema target/version
```

Không thêm DB column chỉ để lưu config nếu config deployment-level đủ dùng cho Phase 02.

Nếu config ảnh hưởng immutable Daily Snapshot output, provenance nên ghi lại ít nhất:

```text
display_simplification_algorithm
display_tolerance_m          # effective tolerance sau escalation (rev 2)
display_coordinate_decimals  # = 6 (rev 2)
display_vertices_over_budget # bool (rev 2)
```

hoặc `projection_schema_version` phải đủ xác định deterministic default.

---

# 34. Recommended provenance

Daily Snapshot provenance v2:

```json
{
  "projection_schema_version": 2,
  "display_geometry": {
    "algorithm": "rdp-v1",
    "tolerance_m": 10,
    "coordinate_decimals": 6,
    "max_vertices": 3000,
    "vertices_over_budget": false
  }
}
```

Trong đó:

- `tolerance_m` là **effective tolerance sau escalation** (§9.3), không phải base config — vì snapshot là per-day và escalation có thể khác nhau mỗi ngày;
- `vertices_over_budget: true` khi đã chạm max tolerance mà vẫn vượt budget (§9.3 fallback).

Điều này giúp debug vì sao historical snapshot có số vertex khác snapshot được tạo sau config change.

Nếu config thay đổi làm output khác, phải có versioning/reprojection policy rõ ràng.

---

# 35. Không dùng client-side simplification làm solution chính

Có thể simplify ở browser nhưng không khuyến nghị làm primary fix.

Nếu server vẫn gửi 50k points:

```text
network cost already paid
JSON parse cost already paid
memory cost already paid
```

Client simplification chỉ giảm phần MapLibre render sau đó.

Do đó simplification phải xảy ra trước network boundary.

---

# 36. Không dùng `every Nth point`

Sai approach:

```text
points.filter((_, i) => i % 10 === 0)
```

Nó có thể bỏ:

- góc rẽ;
- short movement;
- important shape points.

Phải dùng geometry-aware simplification.

---

# 37. Không thay firmware để giải quyết Web payload

Firmware adaptive GPS persistence là optimization riêng.

Nó giúp:

```text
SD
network ingest
DB
processing
```

nhưng Web vẫn phải có display projection đúng kiến trúc.

Không phụ thuộc vào giả định future firmware luôn sparse.

Server/Web phải chịu được legacy dense datasets.

---

# 38. Compatibility với adaptive GPS persistence

Sau khi firmware được tối ưu:

```text
MOVING        sparse adaptive
STATIONARY    heartbeat
```

canonical server data tự nhiên nhỏ hơn.

Display simplification vẫn giữ nguyên vai trò:

```text
legacy dense data        → lightweight Web
new sparse data          → lightweight Web
```

---

# 39. Rollout order

Khuyến nghị thực hiện theo thứ tự sau.

## Step 1 — baseline benchmark

Trước khi sửa:

- chọn realistic dense day;
- ghi lại Daily View JSON bytes;
- count route vertices;
- count progress anchors;
- count source_record_ids;
- đo browser load/MapLibre ready (làm baseline cho time-based targets ở §28).

Không tối ưu mà không có baseline.

## Step 2 — OpenAPI contract design

Chốt:

```text
DisplayRoutePart
PlaybackDailyView/PlaybackRoutePart
```

và publication consistency (bao gồm pinned `200` / `410` semantics ở §8, **không có `409`**).

Viết contract trước implementation.

## Step 3 — server display simplifier

Implement deterministic metric simplification + rounding + tests (S1–S3, S7).

## Step 4 — playback endpoint

Reuse `project_part_for_playback` (canonical clipping/progress code giữ nguyên).

Playback endpoint **không phụ thuộc snapshot v2** nên đi trước cutover: nó đọc
canonical qua manifest và có thể ship/verify độc lập.

## Step 5 — projection schema v2

Daily Snapshot chỉ chứa lightweight display data.

Không public source record ID arrays.

## Step 6 — regenerate API client

Không edit generated types manually.

## Step 7 — Web display migration

MapLibre dùng `display_geometry`.

Daily View page phải hoạt động hoàn chỉnh mà chưa fetch playback.

Web assert `projection_schema_version == 2`, fail-fast nếu gặp v1.

## Step 8 — lazy playback

Fetch on Play, validate manifest, triple-check, initialize controller.

## Step 9 — performance acceptance

Chạy realistic fixture + browser tests.

So sánh trước/sau (payload + render time).

## Step 10 — historical reprojection (mới trong rev 2)

Chạy projection-only requeue cho toàn bộ historical publications lên v2 (test I5). Verify **không còn `daily_publications` trỏ tới snapshot v1** — row snapshot v1 cũ vẫn tồn tại (immutable), chỉ publication được switch sang v2.

---

# 40. Definition of Done

Task chỉ hoàn thành khi tất cả điều sau đúng:

- [ ] Raw GPS không bị sửa/xóa vì UI optimization.
- [ ] Canonical Activity Revision giữ route/progress authority.
- [ ] Daily View không gửi canonical playback arrays mặc định.
- [ ] Daily View không gửi `source_record_ids` mặc định.
- [ ] Daily route có server-simplified display geometry.
- [ ] Display coordinates rounded 6 decimals.
- [ ] Display simplification không vượt Part/Gap/Hole boundaries.
- [ ] `visible_distance_m` không tính lại từ simplified geometry.
- [ ] `distance_m` giữ full part canonical distance trong OpenAPI docs.
- [ ] Initial Web load không fetch playback endpoint.
- [ ] MapLibre render bằng display geometry.
- [ ] Playback fetch lazy khi user Play.
- [ ] Playback sử dụng canonical timestamp/progress data.
- [ ] Controller chỉ init sau triple-check device/date/manifest.
- [ ] Daily/Playback manifest consistency được verify.
- [ ] Pinned playback contract: manifest tồn tại → `200`, manifest mất → `410`; **không có `409`**.
- [ ] Manifest refetch loop bounded (3 vòng refresh).
- [ ] `410` reset playback atomically (pause + clear cache + refetch Daily View).
- [ ] Không còn `daily_publications` trỏ tới snapshot v1 (row snapshot v1 cũ vẫn giữ nguyên, immutable).
- [ ] Provenance ghi effective tolerance + vertices_over_budget.
- [ ] OpenAPI + generated TypeScript types đồng bộ.
- [ ] Existing tests pass.
- [ ] New server/web tests pass (bao gồm S7, I5, W7, W8).
- [ ] Dense-day payload giảm ít nhất 70% so baseline.
- [ ] Render-time targets đạt (§28).
- [ ] Visual route vẫn giữ major shape/turns.
- [ ] Layout Daily page không đổi: workspace grid, header grid, vị trí toggle
      Raw GPS, và các state loading/empty/error vẫn render trong cùng khung;
      state mới của playback/export non-reflowing (§19.0).
- [ ] Trip/Stop/Gap/Evidence Hole semantics không đổi.

---

# 41. Agent guardrails

Agent không được:

```text
- simplify gps_points
- mutate immutable Activity Revision
- calculate distance from display geometry
- restore OSRM/Valhalla
- interpolate across GPS Gap
- merge Route Parts across Evidence Hole
- eager-load playback on Daily Map mount
- expose full Raw GPS in normal UI
- edit generated OpenAPI TypeScript file manually
- overwrite immutable old Daily Snapshots
- init playback controller khi triple-check (device/date/manifest) fail
- silent-fallback về canonical geometry khi gặp snapshot v1
- drop part hoặc merge parts để đạt vertex budget
- retry manifest refetch vô hạn (tối đa 3 vòng refresh)
- fallback sang display geometry cho export/video khi playback fetch fail
- phá layout Daily page (workspace/header grid, vị trí Raw GPS toggle, khung
  loading/empty/error); state mới của playback/export phải non-reflowing
```

Nếu implementation yêu cầu một trong các hành vi trên, dừng và review architecture trước.

---

# 42. Target final flow

```text
Device
  ↓
Raw GPS immutable
  ↓
Phase 02 processing
  ↓
Canonical Activity Revision
  │
  ├─────────────────────────────────────┐
  │                                     │
  ▼                                     ▼
Daily Projection                    Playback Projection
  │                                     │
  │ RDP ~10m + round 6 decimals         │ canonical progress
  │ remove UI-unneeded arrays           │ canonical timestamps
  ▼                                     ▼
Lightweight Daily View              Lazy Playback API
  │                                     │
  ▼                                     │ only when Play
MapLibre initial render                 ▼
  │                              Playback Controller
  ▼                              (triple-checked)
fast interactive map
```

Kết quả mong muốn:

```text
Server keeps evidence.
Web receives only what the current interaction needs.
```

Đây là boundary nên giữ lâu dài cho LifeTrail.
