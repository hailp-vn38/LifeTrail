# LifeTrail — Web Architecture Specification

> **Status:** Proposed implementation baseline  
> **Project:** LifeTrail  
> **Scope:** `web/`  
> **Framework:** Vue 3 + TypeScript  
> **Map:** MapLibre GL JS  
> **Last reviewed:** 2026-10-04

---

# 1. Mục tiêu và nguyên tắc Web

Tài liệu này là specification độc lập cho `web/` của LifeTrail. Web **không phải source of truth**; dữ liệu chuẩn nằm ở LifeTrail Server/PostgreSQL. Web tập trung vào read model, spatial visualization và interaction của người dùng.

Nguyên tắc bắt buộc:

- Vue 3 + TypeScript;
- SPA authenticated; Phase 1 không cần SSR;
- OpenAPI từ Rust server là API contract chuẩn;
- server-state dùng TanStack Vue Query, không dùng Pinia làm API cache;
- Pinia chỉ dùng cho client/UI state thực sự cần chia sẻ;
- MapLibre GL JS là renderer;
- GeoJSON cho daily/detail view, vector tiles là đường nâng cấp cho history lớn;
- API timestamp dùng UTC, UI mới chuyển sang timezone người dùng;
- SSE chỉ phát invalidation signal, REST vẫn là data transport chính;
- không render DOM marker cho từng raw GPS point;
- basemap/style URL phải configurable để giữ khả năng self-host.

---

# 2. System architecture — Web view

```text
LifeTrail Server
      │
      ├── REST / OpenAPI
      ├── GeoJSON
      └── SSE invalidation
      │
      ▼
┌─────────────────────────────────┐
│ LifeTrail Web                   │
│ Vue 3 + TypeScript              │
│ Vue Router                      │
│ TanStack Vue Query              │
│ Pinia (UI/client state only)    │
│ MapLibre GL JS                  │
└───────────────┬─────────────────┘
                │
       ┌────────┴────────┐
       ▼                 ▼
    Timeline             Map
                          │
                  configurable basemap
```

Daily view nên đi theo một read-model duy nhất:

```text
URL device/date
      ↓
TanStack Query
      ↓
GET daily read model
      ↓
summary + route + events + stops/trips
      ↓
┌──────────────┬──────────────┐
▼              ▼              ▼
Summary     Timeline       MapLibre
```

---

# 3. Vai trò của Web

Web không phải source of truth.

Source of truth là server/database.

Web có trách nhiệm:

```text
query server data
      ↓
cache server state
      ↓
render timeline + map
      ↓
manage user interaction
```

Web không thực hiện:

- authoritative trip detection;
- authoritative stop detection;
- sửa raw GPS;
- tính batch sync state của device;
- giữ database local đầy đủ của lịch sử.

---

---

# 4. Web library stack

## Core

```text
vue
@vitejs/plugin-vue
vite
typescript
```

### Vue

Vue là UI framework chính.

Convention:

```text
Composition API
<script setup lang="ts">
```

Không trộn Options API và Composition API trong code mới nếu không có lý do.

---

## Routing

```text
vue-router
```

Routes dự kiến:

```text
/login
/
/devices
/devices/:deviceId
/devices/:deviceId/day/:date
/devices/:deviceId/trips/:tripId
/settings
```

URL phải đủ thông tin để reload trang mà vẫn quay lại đúng ngày/device.

Ví dụ:

```text
/devices/01K.../day/2026-10-04
```

Không để ngày hiện tại chỉ tồn tại trong Pinia mà không phản ánh trong URL.

---

## UI state

```text
pinia
```

Pinia chỉ giữ **client state**.

Ví dụ:

```text
selectedEventId
selectedTripId
mapViewport
sidebarCollapsed
mapLayerVisibility
userPreferences
```

Không để Pinia trở thành cache API chính.

---

## Server state

```text
@tanstack/vue-query
```

TanStack Query quản lý:

```text
GET /devices
GET /days/:date
GET /timeline
GET /trips
GET /stops
GET /media
```

Nó chịu trách nhiệm:

- caching;
- stale state;
- retry;
- invalidation;
- background refetch;
- loading/error state.

Quy tắc:

```text
remote state  → Vue Query
UI state      → Pinia / component state
URL state     → Vue Router
```

---

---

# 5. Contract Web ↔ Rust Server

Web giả định Rust server cung cấp API versioned dưới `/api/v1`.

Phase 1:

```text
GET /api/v1/devices
GET /api/v1/devices/:deviceId/days/:date
GET /api/v1/devices/:deviceId/gps?from=&to=
```

Phase 2:

```text
GET /api/v1/devices/:deviceId/timeline?date=
GET /api/v1/devices/:deviceId/trips?date=
GET /api/v1/devices/:deviceId/stops?date=
```

Daily read model là contract quan trọng nhất cho Web. Nó nên cung cấp đủ dữ liệu render một ngày mà không buộc browser tải toàn bộ raw GPS:

```json
{
  "date": "2026-10-04",
  "processing_state": "ready",
  "summary": {
    "gps_points": 13428,
    "distance_m": 12840,
    "duration_s": 13320
  },
  "route": {
    "type": "Feature",
    "geometry": {
      "type": "LineString",
      "coordinates": []
    }
  },
  "start": null,
  "end": null,
  "trips": [],
  "stops": [],
  "events": []
}
```

Typed client pipeline:

```text
Rust DTO / utoipa
       ↓
OpenAPI JSON
       ↓
openapi-typescript
       ↓
src/api/generated/schema.d.ts
       ↓
openapi-fetch
```

Browser authentication dùng secure server-side session cookie. Web không giữ device token. Mutation có side effect phải tuân thủ CSRF contract của server.

---

# 6. Typed API client

Một mục tiêu quan trọng là không định nghĩa DTO bằng tay hai lần.

Pipeline:

```text
Rust structs + utoipa
        │
        ▼
OpenAPI schema
        │
        ▼
openapi-typescript
        │
        ▼
TypeScript API types
        │
        ▼
openapi-fetch
        │
        ▼
Vue Query composables
```

Server là chủ sở hữu API contract.

Web generate type từ OpenAPI.

---

## Package

```text
openapi-typescript
openapi-fetch
```

Không cần Axios mặc định.

`openapi-fetch` dùng native fetch và type từ OpenAPI.

---

## Generated file

```text
web/src/api/generated/
└── lifetrail-v1.d.ts
```

Không sửa file generated bằng tay.

Script:

```json
{
  "scripts": {
    "api:generate": "openapi-typescript ../protocol/openapi/lifetrail-v1.json -o src/api/generated/lifetrail-v1.d.ts"
  }
}
```

---

## API client

```ts
// src/api/client.ts

import createClient from 'openapi-fetch'
import type { paths } from './generated/lifetrail-v1'

export const api = createClient<paths>({
  baseUrl: '/api/v1',
  credentials: 'include',
})
```

Production ưu tiên cùng origin:

```text
https://lifetrail.example.com/
https://lifetrail.example.com/api/v1/...
```

Thay vì:

```text
app.example.com
api.example.com
```

ở Phase 1.

Cùng origin giúp đơn giản:

- cookies;
- CORS;
- CSRF policy;
- self-host deployment.

---

---

# 7. Query key convention

Query key phải thống nhất.

Ví dụ:

```ts
export const queryKeys = {
  devices: ['devices'] as const,

  day: (deviceId: string, date: string) =>
    ['device', deviceId, 'day', date] as const,

  timeline: (deviceId: string, date: string) =>
    ['device', deviceId, 'timeline', date] as const,

  trips: (deviceId: string, date: string) =>
    ['device', deviceId, 'trips', date] as const,
}
```

Không tự viết query key rải rác trong component.

---

---

# 8. Daily view là read model chính

Web Phase 1 nên ưu tiên một endpoint tổng hợp:

```text
GET /api/v1/devices/:deviceId/days/:date
```

Response:

```json
{
  "device_id": "019...",
  "date": "2026-10-04",
  "timezone": "Asia/Ho_Chi_Minh",
  "processing_state": "ready",
  "summary": {
    "point_count": 13428,
    "distance_m": 12840.2,
    "moving_duration_s": 9234,
    "first_fix_at": "2026-10-04T00:15:00Z",
    "last_fix_at": "2026-10-04T11:28:00Z"
  },
  "route": {
    "type": "Feature",
    "geometry": {
      "type": "LineString",
      "coordinates": []
    },
    "properties": {}
  },
  "stops": [],
  "trips": [],
  "events": []
}
```

Ưu điểm:

```text
1 HTTP query
   │
   ├── Summary
   ├── Timeline
   └── Map
```

thay vì 3 component tự fetch trùng dữ liệu.

---

---

# 9. Web directory structure

```text
web/src/
├── main.ts
├── App.vue
│
├── app/
│   ├── router.ts
│   ├── query-client.ts
│   └── bootstrap.ts
│
├── api/
│   ├── client.ts
│   ├── generated/
│   ├── queries/
│   │   ├── device.queries.ts
│   │   ├── day.queries.ts
│   │   └── timeline.queries.ts
│   └── mutations/
│       └── settings.mutations.ts
│
├── features/
│   ├── auth/
│   ├── devices/
│   ├── daily-view/
│   ├── map/
│   ├── timeline/
│   ├── trips/
│   ├── stops/
│   └── media/
│
├── components/
│   ├── base/
│   └── layout/
│
├── stores/
│   ├── ui.store.ts
│   └── preferences.store.ts
│
├── composables/
│   ├── useSse.ts
│   ├── useSelectedEvent.ts
│   └── useTimezone.ts
│
├── lib/
│   ├── date.ts
│   ├── geo.ts
│   └── errors.ts
│
├── styles/
│   ├── tokens.css
│   └── main.css
│
└── types/
```

Feature folder sở hữu UI và logic theo use case.

Không tạo một thư mục `utils/` khổng lồ chứa mọi thứ.

---

---

# 10. Map stack

## Library

```text
maplibre-gl
```

Không cần wrapper Vue cho MapLibre ở core architecture.

Lý do:

- MapLibre API imperative;
- wrapper có thể che mất lifecycle quan trọng;
- direct integration giúp kiểm soát source/layer/event rõ hơn.

Vue component chỉ quản lý lifecycle của map instance.

---

## Component structure

```text
features/map/
├── components/
│   ├── LifeTrailMap.vue
│   ├── MapControls.vue
│   └── MapStatus.vue
│
├── map/
│   ├── create-map.ts
│   ├── sources.ts
│   ├── layers.ts
│   ├── interactions.ts
│   └── camera.ts
│
└── types.ts
```

---

---

# 11. Map source/layer architecture

Không dùng một source duy nhất cho tất cả dữ liệu.

```text
MapLibre
│
├── source: daily-route
│   └── layer: route-line
│
├── source: trip-route
│   └── layer: trip-line
│
├── source: stops
│   ├── layer: stop-circle
│   └── layer: stop-label
│
├── source: media-events
│   ├── layer: photo-symbol
│   └── layer: audio-symbol
│
└── source: selection
    └── layer: selected-event
```

Mỗi source có ownership rõ ràng.

---

---

# 12. Không render mỗi GPS point bằng DOM marker

Một ngày 1 Hz có thể đạt:

```text
86,400 GPS points/day
```

Không tạo 86,400 `Marker` DOM objects.

Route nên là:

```text
GeoJSON LineString
```

và render bằng MapLibre line layer.

Point raw chỉ hiển thị khi user zoom/debug hoặc khi cần inspection đặc biệt.

---

---

# 13. GeoJSON policy

Phase 1–2:

```text
specific day
    ↓
GeoJSON
```

GeoJSON phù hợp cho:

- daily route;
- stops;
- start/end;
- trips của một ngày;
- media markers.

Không dùng GeoJSON để trả toàn bộ lịch sử nhiều năm.

---

---

# 14. Vector tile migration path

Khi data lớn:

```text
month/year/all history
      ↓
PostGIS
      ↓
MVT / tile server
      ↓
MapLibre
```

Hướng dài hạn:

```text
PostgreSQL + PostGIS
        │
        ├── ST_AsMVT directly
        │
        └── Martin tile server
                │
                ▼
            MapLibre
```

Không cần triển khai vector tile trong Phase 1.

---

---

# 15. Basemap abstraction

Config:

```env
VITE_MAP_STYLE_URL=https://example/style.json
```

Code:

```ts
const map = new maplibregl.Map({
  container,
  style: import.meta.env.VITE_MAP_STYLE_URL,
})
```

Không import trực tiếp token/provider vào domain component.

Có thể thay:

```text
hosted provider
      ↓
self-hosted style + tiles
```

mà không thay cấu trúc feature map.

---

---

# 16. Timeline ↔ Map synchronization

Trạng thái trung tâm:

```text
selectedEventId
```

Không cho Timeline gọi method Map trực tiếp.

```text
Timeline click
     │
     ▼
selectedEventId
     │
     ├── Timeline highlight
     │
     └── Map selected layer + flyTo
```

Map click:

```text
Map feature click
     │
     ▼
selectedEventId
     │
     ├── marker highlight
     └── timeline scrollIntoView
```

Pinia có thể giữ:

```ts
interface SelectionState {
  selectedEventId: string | null
  selectedTripId: string | null
}
```

---

---

# 17. SSE architecture

Phase 1 có thể chưa bật SSE.

Khi cần web tự cập nhật sau device sync:

```text
Device upload
     ↓
Server commits batch
     ↓
Server emits event
     ↓
SSE
     ↓
Browser
     ↓
Vue Query invalidate
     ↓
HTTP refetch
```

SSE không vận chuyển toàn bộ GPS stream.

Nó chỉ là invalidation signal.

Ví dụ event:

```text
event: day.updated
data: {"device_id":"...","date":"2026-10-04"}
```

Web:

```ts
const source = new EventSource('/api/v1/events', {
  withCredentials: true,
})

source.addEventListener('day.updated', (event) => {
  const payload = JSON.parse(event.data)

  queryClient.invalidateQueries({
    queryKey: queryKeys.day(payload.device_id, payload.date),
  })
})
```

---

---

# 18. Date/time policy trên Web

Server và DB trả timestamp UTC ISO 8601.

Ví dụ:

```text
2026-10-04T08:31:42Z
```

Web convert sang timezone của user.

`date` của daily view phải có timezone context.

Ví dụ:

```text
2026-10-04
Asia/Ho_Chi_Minh
```

không được hiểu `date` bằng timezone của server machine.

Web nên dùng native `Intl.DateTimeFormat` trước.

Chỉ thêm date library nếu use case trở nên phức tạp.

---

---

# 19. Web dependency policy

## Required

```text
vue
vue-router
pinia
@tanstack/vue-query
maplibre-gl
openapi-fetch
```

## Dev required

```text
vite
@vitejs/plugin-vue
typescript
vue-tsc
openapi-typescript
vitest
@vue/test-utils
playwright
eslint
prettier
```

## Optional

```text
@vueuse/core
zod
msw
```

`zod` chỉ cần nếu có dữ liệu runtime không được bảo đảm bởi API contract hoặc cần validation form phức tạp.

---

---

# 20. Suggested package installation

Dùng `pnpm` làm package manager cho web.

```bash
pnpm add vue vue-router pinia @tanstack/vue-query maplibre-gl openapi-fetch

pnpm add -D \
  vite \
  @vitejs/plugin-vue \
  typescript \
  vue-tsc \
  openapi-typescript \
  vitest \
  @vue/test-utils \
  playwright \
  eslint \
  prettier
```

Repo phải commit:

```text
pnpm-lock.yaml
```

CI dùng frozen lockfile.

---

---

# 21. Vite config baseline

```ts
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
})
```

Development:

```text
Vite :5173
  ↓ proxy /api
Rust :8080
```

Production:

```text
Caddy/Nginx
├── /      → static Vue dist
└── /api   → Rust server
```

---

---

# 22. Web testing

## Unit

Vitest:

```text
formatters
query-key builders
selection logic
map data transforms
```

## Component

Vue Test Utils:

```text
Timeline item
Daily summary
Loading/error view
```

Không cần thực MapLibre canvas cho mọi component test.

Map boundary có thể mock.

## E2E

Playwright:

```text
login
select device
select day
route appears
click timeline → map selection
click map → timeline selection
logout
```

---

---

# 23. Web performance policy

- route-level code splitting;
- lazy load map page nếu cần;
- không clone GeoJSON lớn nhiều lần;
- không deep reactive hàng chục nghìn GPS points nếu không cần;
- giữ raw map source data ngoài Pinia nếu chỉ MapLibre cần;
- dùng server-simplified route cho daily overview;
- large history dùng vector tiles.

---

# PHẦN II — SERVER ARCHITECTURE

---

# 24. Map query strategy

## Daily

```text
GeoJSON route
```

## Raw debug points

```text
bounded time range + point limit
```

## Large history

```text
vector tiles
```

Không có endpoint:

```text
GET /all-gps-ever
```

---

---

# 25. Self-host map path

MVP:

```text
MapLibre
+
external hosted OSM-derived tiles
```

Long term:

```text
MapLibre
+
self-host vector style/tiles
```

Optional stack:

```text
Martin
PMTiles/MBTiles
PostGIS MVT
```

Không dùng public `tile.openstreetmap.org` như production bulk tile backend.

---

---

# 26. Security headers

Reverse proxy/server nên cấu hình phù hợp:

```text
Content-Security-Policy
X-Content-Type-Options
Referrer-Policy
Permissions-Policy
Strict-Transport-Security production
```

CSP phải cho phép map tile/style/font domains đã cấu hình.

Self-host tiles giúp CSP/privacy đơn giản hơn.

---

---

# 27. Privacy

LifeTrail chứa dữ liệu location cực kỳ nhạy cảm về mặt riêng tư.

Architecture nên áp dụng:

- private by default;
- không public map URL mặc định;
- không gửi coordinates sang analytics bên thứ ba;
- không log raw route nếu không cần;
- tile provider chỉ nhận basemap tile requests, không cần nhận LifeTrail event data;
- media URLs phải protected/signed nếu có auth;
- backup cần access control.

---

---

# 28. Analytics

Không thêm third-party session replay vào app mặc định.

Nếu có telemetry:

- opt-in;
- không gửi location/history;
- self-host metrics nếu có thể.

---

---

# 29. Error handling UX

Web phân biệt:

```text
network unavailable
401 unauthenticated
403 forbidden
404 no data
409 conflict
422 validation
500 server error
processing pending
```

Không hiển thị mọi lỗi thành:

```text
Something went wrong
```

Ví dụ daily view không có GPS:

```text
No GPS data for this day.
```

khác với server 500.

---

---

# 30. Offline web

Device là offline-first.

Web Phase 1 **không bắt buộc offline-first**.

Không thêm IndexedDB/service worker sync chỉ vì firmware offline-first.

Web có thể cache in-memory qua TanStack Query.

PWA/offline history là feature riêng trong tương lai.

---

---

# 31. Definition of Done — Web Phase 1

Web Phase 1 hoàn thành khi:

- Vue TypeScript build clean;
- API types generate từ OpenAPI;
- login/session nếu deployment yêu cầu auth;
- chọn device;
- chọn ngày;
- load daily read model;
- hiển thị route MapLibre;
- hiển thị start/end;
- hiển thị point count;
- hiển thị duration;
- hiển thị estimated distance;
- loading/error/empty states rõ ràng;
- page reload giữ đúng device/date qua URL;
- E2E smoke test pass.

---

---

# 32. Library decision matrix — Web

| Need | Chọn | Không ưu tiên |
|---|---|---|
| UI framework | Vue 3 | React/Angular trong cùng app |
| Build | Vite | custom webpack |
| Routing | Vue Router | custom router |
| Client state | Pinia | Vuex mới |
| Server state | TanStack Vue Query | nhét API cache vào Pinia |
| HTTP | openapi-fetch | Axios mặc định |
| API types | openapi-typescript | DTO viết tay lặp lại |
| Map | MapLibre GL JS | Google Maps hard dependency |
| Unit test | Vitest | Jest mới cho Vite app |
| Component test | Vue Test Utils | browser E2E cho mọi unit |
| E2E | Playwright | manual-only QA |

---

---

# 33. Dependencies không nên thêm sớm

Không thêm nếu chưa có requirement:

```text
Redis
Kafka
NATS
Elasticsearch
ClickHouse
Kubernetes
GraphQL
WebSocket
ORM nặng
PWA offline database
```

Mỗi dependency mới phải trả lời:

1. vấn đề cụ thể là gì;
2. PostgreSQL/Tokio/Vue hiện tại không giải quyết được vì sao;
3. failure mode mới là gì;
4. backup/upgrade/observability thêm bao nhiêu complexity.

---

---

# 34. CI — Web

CI tối thiểu cho `web/`:

```text
pnpm install --frozen-lockfile
pnpm typecheck
pnpm lint
pnpm test
pnpm build
```

Playwright smoke test chạy với test server environment. CI cũng phải kiểm tra generated OpenAPI types không drift so với contract đã commit.

---

# 35. Implementation roadmap — Web

## Phase W0 — Skeleton

```text
Vue 3
Vite
Vue Router
TanStack Vue Query
MapLibre empty map
OpenAPI generated client
```

## Phase W1 — Daily GPS view

```text
device selector
date selector
daily read model
summary
route
start/end
loading/error/empty states
```

## Phase W2 — Processing updates

```text
processing pending/ready state
SSE invalidation
query invalidation/refetch
```

## Phase W3 — Trips / stops / timeline

```text
trip layer
stop layer
TimelineEvent list
selectedEventId
Timeline ↔ Map focus synchronization
```

## Phase W4 — Media

```text
photo/audio event
media viewer
protected media URLs
media markers
```

## Phase W5 — Large history

```text
history search
aggregated spatial view
MVT/vector tiles
optional self-hosted map stack
```

---

# 36. Source/reference notes — Web

- Vue: https://vuejs.org/
- Vue Router: https://router.vuejs.org/
- Pinia: https://pinia.vuejs.org/
- Vite: https://vite.dev/
- TanStack Vue Query: https://tanstack.com/query/latest/docs/framework/vue
- MapLibre GL JS: https://maplibre.org/maplibre-gl-js/docs/
- openapi-typescript/openapi-fetch: https://openapi-ts.dev/
- Vitest: https://vitest.dev/
- Vue Test Utils: https://test-utils.vuejs.org/
- Playwright: https://playwright.dev/

Version policy:

- commit `pnpm-lock.yaml`;
- ưu tiên stable release;
- không tự động nhảy major version;
- regenerate API types khi OpenAPI thay đổi;
- dependency upgrade phải pass typecheck, unit test, build và E2E smoke test.

---

# 37. Final recommended stack

```text
WEB
──────────────────────────────────
Vue 3
TypeScript
Vite
Vue Router
Pinia
TanStack Vue Query
MapLibre GL JS
openapi-typescript
openapi-fetch
Vitest
Vue Test Utils
Playwright

PROTOCOL / DATA
──────────────────────────────────
HTTPS
REST JSON
OpenAPI-generated types
GeoJSON daily/detail views
SSE cache invalidation
MVT/vector tiles for large history later

MAP
──────────────────────────────────
MapLibre GL JS
Configurable basemap style URL
Hosted tiles initially
Self-host tiles later
```
