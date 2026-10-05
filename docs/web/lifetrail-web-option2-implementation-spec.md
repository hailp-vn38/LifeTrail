# LifeTrail Web — Option 2 UI & Dedicated Web Service Implementation Spec

> **Status:** Approved implementation baseline
> **Project:** LifeTrail
> **Repository:** `hailp-vn38/LifeTrail`
> **Scope:** Web architecture refresh + Docker web service separation
> **Frontend:** Vue 3 + TypeScript + Vite
> **Map:** MapLibre GL JS + MapTiler configurable basemap
> **UI baseline:** Option 2 — Light, Clean, 2D Map, Right Timeline
> **Target:** Phase 1 compatible, Phase 2 ready

---

# 1. Objective

Refactor the current LifeTrail web application into a complete application shell with a modern light UI based on **Option 2**, while keeping current Phase 1 behavior working and preparing the codebase for Phase 2 timeline playback.

The web application must also become a **dedicated Docker service** instead of being built into and served by the Rust server image.

The final architecture must support:

- persistent left sidebar;
- top application header;
- Daily Map as the primary product screen;
- MapLibre + MapTiler basemap;
- right-side Timeline panel;
- route playback preparation;
- camera follow support in Phase 2;
- responsive desktop/tablet/mobile layout;
- web service served by Nginx;
- `/api/*` reverse proxy from web container to Rust server;
- same-origin browser API access;
- no breaking change to Phase 1 device ingest API.

---

# 2. Existing baseline

The repository currently uses:

```text
Vue 3
TypeScript
Vite
Vue Router
TanStack Vue Query
MapLibre GL JS
openapi-fetch
MapTiler configurable basemap
```

Current deployment behavior:

```text
Vue build
   ↓
embedded into Rust Docker image
   ↓
Rust server serves both API and static web
```

Current script behavior also treats `web` as an alias of `server`.

This must change.

---

# 3. Target deployment topology

Use three runtime services:

```text
Browser / ESP32
       │
       │ http://<LAN-IP>:8080
       ▼
┌───────────────────────────────┐
│ web                           │
│ Nginx                         │
│                               │
│ /           → Vue SPA         │
│ /assets/*   → static assets   │
│ /api/*      → proxy server    │
└──────────────┬────────────────┘
               │ Docker network
               ▼
┌───────────────────────────────┐
│ server                        │
│ Rust + Axum                   │
│ :8080 internal               │
└──────────────┬────────────────┘
               │
               ▼
┌───────────────────────────────┐
│ postgres                      │
│ PostgreSQL + PostGIS          │
└───────────────────────────────┘
```

## 3.1 Required public port behavior

The public LAN entrypoint remains:

```text
http://<LAN-IP>:8080
```

Browser:

```text
GET /
GET /assets/*
GET /api/v1/*
```

ESP32:

```text
POST http://<LAN-IP>:8080/api/v1/device/batches
```

The device upload URL does not change.

## 3.2 Server exposure

The Rust server should only be reachable on the Docker network in the default acceptance topology.

The `web` service owns host port `8080`.

---

# 4. Docker Compose target

Update `deploy/docker-compose.yml` to use real services:

```yaml
services:
  postgres:
    image: postgis/postgis:17-3.5

  server:
    build:
      context: ..
      dockerfile: deploy/server.Dockerfile

    environment:
      LT_DATABASE_URL: postgres://lifetrail:lifetrail_dev_only@postgres:5432/lifetrail
      LT_BIND_ADDR: 0.0.0.0:8080

    depends_on:
      postgres:
        condition: service_healthy

    expose:
      - "8080"

  web:
    build:
      context: ..
      dockerfile: deploy/web.Dockerfile
      args:
        VITE_MAP_STYLE_URL: ${VITE_MAP_STYLE_URL:-}
        VITE_MAPTILER_KEY: ${VITE_MAPTILER_KEY:-}

    depends_on:
      - server

    ports:
      - "8080:80"
```

Preserve existing test services and PostgreSQL volumes unless a test-specific change is required.

---

# 5. Docker file split

Replace the current combined `deploy/Dockerfile` architecture with:

```text
deploy/
├── docker-compose.yml
├── server.Dockerfile
├── web.Dockerfile
└── nginx.conf
```

## 5.1 `server.Dockerfile`

The Rust server image must:

- build only the Rust server;
- no longer build Vue;
- no longer copy `web/dist`;
- no longer require `LT_STATIC_DIR` for normal runtime;
- expose internal port 8080;
- run `lifetrail-server serve`.

## 5.2 `web.Dockerfile`

Use a multi-stage build.

Reference baseline:

```dockerfile
FROM node:22-bookworm-slim AS builder

WORKDIR /app

COPY web/package.json web/package-lock.json ./
RUN npm ci

COPY web/ ./

ARG VITE_MAP_STYLE_URL
ARG VITE_MAPTILER_KEY

ENV VITE_MAP_STYLE_URL=$VITE_MAP_STYLE_URL
ENV VITE_MAPTILER_KEY=$VITE_MAPTILER_KEY

RUN npm run build

FROM nginx:alpine

COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist /usr/share/nginx/html

EXPOSE 80
```

Production web runtime must not require Node.js.

---

# 6. Nginx reverse proxy

Create `deploy/nginx.conf`.

Required behavior:

```nginx
server {
    listen 80;
    server_name _;

    root /usr/share/nginx/html;
    index index.html;

    location /api/ {
        proxy_pass http://server:8080;
        proxy_http_version 1.1;

        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

If SSE is introduced later, extend the `/api/` proxy settings to disable buffering for the SSE endpoint.

---

# 7. Local development topology

Keep the existing Vite development loop:

```text
Vue/Vite localhost:5173
        │
        │ proxy /api
        ▼
Rust localhost:8080
        │
        ▼
PostgreSQL
```

Current Vite proxy behavior is still valid:

```ts
server: {
  proxy: {
    "/api": "http://127.0.0.1:8080",
  },
}
```

Do not force developers to run Nginx during normal frontend development.

---

# 8. Application navigation structure

The UI must follow **Option 2**.

Desktop baseline:

```text
┌────────────────────────────────────────────────────────────────────────────┐
│                            APPLICATION                                    │
├──────────────┬─────────────────────────────────────────────────────────────┤
│              │ Topbar                                                      │
│ LifeTrail    │ Daily Map                    [date]     status / actions     │
│              ├───────────────────────────────────────────┬─────────────────┤
│ Overview     │                                           │                 │
│              │                                           │ Timeline        │
│ Daily Map ●  │                                           │                 │
│              │               MAP                         │ Start           │
│ Timeline     │                                           │ Checkpoints     │
│              │               MapTiler                    │ Events          │
│ Devices      │                                           │ End             │
│              │                                           │                 │
│ Reports      │                                           │                 │
│              ├───────────────────────────────────────────┼─────────────────┤
│ Settings     │ Playback / status                         │ Statistics      │
└──────────────┴───────────────────────────────────────────┴─────────────────┘
```

---

# 9. Sidebar specification

Desktop width baseline:

```text
224px
```

Navigation:

```text
LifeTrail

Overview
Daily Map
Timeline
Devices
Reports
Settings

System status
Version
```

Required routes:

| Menu | Route |
|---|---|
| Overview | `/` |
| Daily Map | `/devices/:deviceId/day/:date` |
| Timeline | `/timeline` |
| Devices | `/devices` |
| Reports | `/reports` |
| Settings | `/settings` |

Rules:

- highlight active item;
- selected item uses soft blue background;
- sidebar persists across page navigation;
- do not mount/unmount sidebar per route;
- system status stays at bottom;
- Phase 1 may use placeholder content for Timeline, Reports and Settings;
- do not hide the navigation entries simply because features are not finished.

---

# 10. Application shell

The current `App.vue` is too minimal.

Target hierarchy:

```text
App.vue
   │
   └── RouterView
          │
          ▼
     AppLayout.vue
          │
          ├── AppSidebar
          │
          └── AppMain
                │
                ├── AppTopbar
                │
                └── RouterView
```

Preferred routing style:

```text
root route
└── component: AppLayout
    ├── overview
    ├── devices
    ├── daily map
    ├── timeline
    ├── reports
    └── settings
```

The layout is part of routing, not duplicated inside every page.

---

# 11. Target source structure

Refactor toward:

```text
web/
├── package.json
├── package-lock.json
├── index.html
├── tsconfig.json
├── vite.config.ts
├── vitest.config.ts
│
├── public/
│   ├── favicon.svg
│   └── logo.svg
│
└── src/
    ├── main.ts
    ├── App.vue
    │
    ├── app/
    │   ├── bootstrap.ts
    │   ├── router.ts
    │   ├── routes.ts
    │   └── query-client.ts
    │
    ├── api/
    │   ├── client.ts
    │   ├── query-keys.ts
    │   │
    │   ├── generated/
    │   │   └── lifetrail-v1.d.ts
    │   │
    │   ├── queries/
    │   │   ├── devices.query.ts
    │   │   ├── daily-view.query.ts
    │   │   └── health.query.ts
    │   │
    │   └── errors/
    │       └── api-error.ts
    │
    ├── layouts/
    │   └── AppLayout.vue
    │
    ├── components/
    │   ├── ui/
    │   │   ├── AppButton.vue
    │   │   ├── AppIconButton.vue
    │   │   ├── AppCard.vue
    │   │   ├── AppBadge.vue
    │   │   ├── AppSkeleton.vue
    │   │   ├── AppEmptyState.vue
    │   │   ├── AppErrorState.vue
    │   │   ├── AppSpinner.vue
    │   │   └── AppTooltip.vue
    │   │
    │   └── layout/
    │       ├── AppSidebar.vue
    │       ├── SidebarLogo.vue
    │       ├── SidebarNav.vue
    │       ├── SidebarNavItem.vue
    │       ├── SidebarFooter.vue
    │       ├── AppTopbar.vue
    │       └── PageHeader.vue
    │
    ├── features/
    │   ├── overview/
    │   │   ├── pages/
    │   │   │   └── OverviewPage.vue
    │   │   └── components/
    │   │
    │   ├── devices/
    │   │   ├── pages/
    │   │   │   ├── DeviceListPage.vue
    │   │   │   └── DeviceDetailPage.vue
    │   │   ├── components/
    │   │   │   ├── DeviceCard.vue
    │   │   │   └── DeviceStatus.vue
    │   │   └── composables/
    │   │
    │   ├── daily-map/
    │   │   ├── pages/
    │   │   │   └── DailyMapPage.vue
    │   │   ├── components/
    │   │   │   ├── DailyMapHeader.vue
    │   │   │   ├── DailyMapCanvas.vue
    │   │   │   ├── DailyMapToolbar.vue
    │   │   │   ├── DailyMapStats.vue
    │   │   │   ├── DailyMapEmpty.vue
    │   │   │   └── DailyMapLoading.vue
    │   │   └── composables/
    │   │       └── useDailyMap.ts
    │   │
    │   ├── map/
    │   │   ├── components/
    │   │   │   ├── LifeTrailMap.vue
    │   │   │   ├── MapControls.vue
    │   │   │   └── MapAttribution.vue
    │   │   ├── core/
    │   │   │   ├── create-map.ts
    │   │   │   ├── map-config.ts
    │   │   │   ├── sources.ts
    │   │   │   ├── layers.ts
    │   │   │   ├── camera.ts
    │   │   │   ├── interactions.ts
    │   │   │   └── bounds.ts
    │   │   └── types.ts
    │   │
    │   ├── timeline/
    │   │   ├── components/
    │   │   │   ├── TimelinePanel.vue
    │   │   │   ├── TimelineHeader.vue
    │   │   │   ├── TimelineList.vue
    │   │   │   ├── TimelineItem.vue
    │   │   │   └── TimelineStats.vue
    │   │   └── types.ts
    │   │
    │   ├── playback/
    │   │   ├── components/
    │   │   │   ├── PlaybackBar.vue
    │   │   │   ├── PlaybackButton.vue
    │   │   │   ├── PlaybackSlider.vue
    │   │   │   └── PlaybackSpeed.vue
    │   │   ├── composables/
    │   │   │   ├── usePlayback.ts
    │   │   │   └── useCameraFollow.ts
    │   │   └── playback.types.ts
    │   │
    │   ├── reports/
    │   │   └── pages/
    │   │       └── ReportsPage.vue
    │   │
    │   └── settings/
    │       └── pages/
    │           └── SettingsPage.vue
    │
    ├── stores/
    │   ├── ui.store.ts
    │   ├── map.store.ts
    │   └── playback.store.ts
    │
    ├── composables/
    │   ├── useMediaQuery.ts
    │   └── useTimezone.ts
    │
    ├── lib/
    │   ├── date.ts
    │   ├── distance.ts
    │   ├── geo.ts
    │   ├── format.ts
    │   └── assert.ts
    │
    ├── styles/
    │   ├── reset.css
    │   ├── tokens.css
    │   ├── typography.css
    │   ├── layout.css
    │   ├── map.css
    │   └── main.css
    │
    └── types/
        ├── map.ts
        └── timeline.ts
```

Do not move every existing file immediately if it creates unnecessary churn. Migrate toward this structure while preserving working behavior.

---

# 12. Package changes

Keep existing packages.

Add:

```text
pinia
lucide-vue-next
```

Do not introduce a heavy component framework such as:

```text
Vuetify
Element Plus
Quasar
Ant Design Vue
```

unless explicitly approved later.

The Option 2 appearance must be implemented using:

```text
Vue components
CSS variables
CSS Grid/Flexbox
Lucide icons
```

---

# 13. State ownership rules

Use:

```text
Remote/server state  → TanStack Vue Query
URL state            → Vue Router
Shared UI state      → Pinia
Local component UI   → component refs/reactive
Map renderer state   → MapLibre instance/core modules
```

Do not turn Pinia into an API response cache.

Examples of valid Pinia state:

```text
sidebarCollapsed
selectedEventId
selectedTripId
cameraFollow
playback status
playback speed
map layer visibility
```

---

# 14. Daily Map page

The canonical Daily Map route remains:

```text
/devices/:deviceId/day/:date
```

Target component structure:

```text
DailyMapPage
│
├── DailyMapHeader
│   ├── page title
│   ├── device selector
│   ├── date picker
│   └── actions
│
├── DailyMapWorkspace
│   ├── MapArea
│   │   ├── LifeTrailMap
│   │   ├── MapControls
│   │   └── PlaybackBar
│   │
│   └── TimelinePanel
│       ├── TimelineHeader
│       ├── TimelineList
│       └── TimelineStats
│
└── responsive states
```

Desktop layout:

```text
                Daily Map       [date]
┌───────────────────────────────────────────────┐
│                              │ Timeline       │
│                              │                │
│                              │ Start          │
│                              │ Events         │
│        MAPTILER MAP          │ Selected       │
│                              │ End            │
│                              │                │
├──────────────────────────────┼────────────────┤
│ Playback / current time      │ Daily stats    │
└──────────────────────────────┴────────────────┘
```

---

# 15. Phase 1 behavior inside the new UI

The new layout must work with the current Phase 1 Daily View response.

Phase 1 rendering:

```text
Map
├── route
├── start
└── end

Timeline
├── Start
└── End

Playback
└── hidden or disabled until time-indexed route data is available
```

Current empty/loading/error behavior must be preserved and redesigned to match Option 2.

## 15.1 Empty state

When `summary.point_count === 0`:

- map area may show the configured basemap or a clean empty placeholder;
- timeline shows an empty state;
- stats show zero values;
- no runtime error;
- Daily Map page remains usable.

## 15.2 Device not found

Show a page-level error card inside AppLayout.

Do not remove sidebar/topbar.

---

# 16. Phase 2-ready behavior

The page structure must already support:

```text
Map
├── completed route
├── remaining route
├── current position
├── start
├── end
├── stops
└── events

Timeline
├── start
├── trip
├── stop
├── photo/audio
└── end

Playback
├── play
├── pause
├── seek
├── speed
└── camera follow
```

Do not implement fake business logic for trips/stops in Phase 1.

Only prepare component and state boundaries.

---

# 17. Map architecture

MapTiler is the basemap provider.

MapLibre is the map renderer.

Architecture:

```text
MapTiler
    │
    ▼
style.json / tiles
    │
    ▼
MapLibre GL JS
    │
    ├── daily-route
    ├── playback-route
    ├── start/end
    ├── current-position
    ├── stops
    └── events
```

Continue using configurable environment variables:

```env
VITE_MAP_STYLE_URL=
VITE_MAPTILER_KEY=
```

Do not hard-code a provider URL into feature components.

---

# 18. Map source/layer model

Prepare a stable layer naming scheme.

Recommended structure:

```text
source: daily-route
  layer: route-background

source: playback-route
  layer: route-completed

source: route-points
  layer: start-point
  layer: end-point

source: current-position
  layer: current-halo
  layer: current-dot

source: timeline-events
  layer: event-circle
  layer: event-selected
```

Do not render a DOM marker for every GPS point.

Use GeoJSON sources and MapLibre layers.

---

# 19. Initial map fit behavior

After a Daily View route loads:

1. compute bounds from the full route;
2. call `fitBounds()`;
3. respect UI padding so route is not hidden behind timeline/sidebar;
4. use a sensible max zoom for short routes;
5. if the route contains only one usable point, center with a default zoom;
6. do not repeatedly auto-fit after the user starts interacting with the map.

Expected padding for desktop should account for:

```text
left sidebar
right timeline
bottom playback bar
```

---

# 20. Timeline ↔ Map synchronization

Do not make Timeline directly call methods on the map component.

Use shared selection state:

```text
selectedEventId
```

Flow:

```text
Timeline click
     │
     ▼
selectedEventId
     │
     ├── Timeline highlight
     └── Map selection + flyTo/easeTo
```

Map click:

```text
Map feature click
     │
     ▼
selectedEventId
     │
     ├── Map highlight
     └── Timeline scrollIntoView
```

This boundary is mandatory.

---

# 21. Playback model for Phase 2

Prepare this state model even if the initial Phase 1 UI does not enable playback:

```ts
interface PlaybackState {
  status: "idle" | "playing" | "paused" | "finished";

  currentTimeMs: number;
  startTimeMs: number;
  endTimeMs: number;

  speed: 1 | 2 | 4;

  cameraFollow: boolean;

  selectedEventId: string | null;
}
```

Use a Pinia store only for state that must be shared between playback, timeline and map.

Animation frame rendering should remain in playback/map composables, not in the Pinia store itself.

---

# 22. Playback flow

Phase 2 target:

```text
PlaybackBar
       │
       ▼
 playback state
       │
 ┌─────┴────────┐
 ▼              ▼
Map           Timeline
```

Timeline event selection:

```text
Timeline item
     ↓
selectedEvent
     ↓
playback currentTime
     ↓
map current position
```

Seek behavior must be deterministic.

---

# 23. Camera follow specification

Phase 2 target behavior:

```text
PLAY
 │
 ▼
current route position changes
 │
 ▼
camera follow
 │
 └── easeTo(current position)
```

Camera rules:

- route/current-position stays near the visual center of the usable map area;
- compensate for right Timeline panel;
- do not continuously zoom in/out during normal playback;
- update bearing only if a future UX decision explicitly enables heading-follow;
- smooth updates to avoid camera jitter.

## 23.1 Manual interaction

If the user manually:

```text
drag
zoom
rotate
```

set:

```text
cameraFollow = false
```

Expose a control:

```text
Re-center / Follow
```

Clicking it re-enables camera follow.

## 23.2 Playback completed

When playback status becomes `finished`:

```text
cameraFollow = false
       ↓
fitBounds(full route)
```

The user should see the complete day route after playback ends.

---

# 24. UI design tokens

Create `src/styles/tokens.css`.

Baseline:

```css
:root {
  --color-bg: #f5f7fb;
  --color-surface: #ffffff;

  --color-border: #e7ebf2;

  --color-text: #172033;
  --color-text-muted: #718096;

  --color-primary: #1677ff;
  --color-primary-soft: #eaf3ff;

  --color-success: #16c784;
  --color-danger: #ff4d4f;

  --sidebar-width: 224px;
  --timeline-width: 320px;
  --topbar-height: 64px;

  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;

  --shadow-card: 0 4px 18px rgba(23, 32, 51, 0.06);
}
```

These are baseline values, not protocol invariants. Small visual adjustments are allowed if they preserve Option 2 appearance.

---

# 25. Typography and visual language

Use a clean modern light UI.

Requirements:

```text
light neutral background
white surfaces
blue primary accent
soft borders
small shadows
clear typography hierarchy
compact but not dense
minimal decoration
```

Avoid:

```text
heavy gradients
large glassmorphism effects
oversized shadows
neon color palette
dark-only UI
```

Default font stack can remain:

```css
font-family: Inter, system-ui, sans-serif;
```

Do not load an external font unless needed.

---

# 26. Sidebar visual behavior

Active state:

```text
soft blue background
primary blue icon/text
rounded item
```

Inactive state:

```text
muted text
transparent background
```

Hover:

```text
subtle neutral or blue-tinted background
```

Sidebar footer:

```text
system online indicator
build/version
```

Do not put business data in the sidebar.

---

# 27. Topbar specification

The AppTopbar should support:

```text
page title / breadcrumb
optional contextual actions
system/user area
```

For Daily Map, contextual controls may include:

```text
device selector
date selector
refresh
```

Avoid duplicating the same control in both topbar and page body.

---

# 28. Responsive requirements

## 28.1 Desktop >= 1280px

```text
Sidebar | Map | Timeline
```

Sidebar expanded.
Timeline target width around `320px`.

## 28.2 Tablet 768–1279px

Preferred:

```text
collapsed sidebar | map | smaller timeline
```

Allow timeline width around 280px.

## 28.3 Mobile < 768px

Use:

```text
Topbar
──────────────
Map
──────────────
Playback
──────────────
Timeline bottom sheet / stacked section
──────────────
Bottom navigation or drawer
```

Do not attempt to keep a permanent 224px sidebar on mobile.

Mobile support must be functional even if desktop receives the highest visual polish.

---

# 29. Overview page

Phase 1 Overview should be useful but simple.

Suggested sections:

```text
System status
Device count
Today's GPS summary
Recent device activity
Shortcut to Daily Map
```

Do not invent server data that does not exist.

If a metric is unavailable, omit it or show an explicit placeholder.

---

# 30. Devices page

Preserve current device listing behavior.

Convert list presentation to Option 2 cards/table styling.

Each device should support navigation to Daily Map.

The device list must remain reloadable directly through URL routing.

---

# 31. Placeholder pages

Create route-safe placeholder pages for:

```text
Timeline
Reports
Settings
```

Requirements:

- render inside AppLayout;
- no broken links;
- visually match Option 2;
- state that the feature is planned or not yet available;
- do not fake server-backed data.

---

# 32. API integration rules

Keep the current generated OpenAPI pipeline.

Expected architecture:

```text
Rust API contract
      ↓
OpenAPI
      ↓
openapi-typescript
      ↓
openapi-fetch
      ↓
TanStack Vue Query
```

Do not duplicate API DTOs manually without need.

Browser client must continue using same-origin relative URLs.

Preferred:

```ts
baseUrl: "/api/v1"
```

Do not use Docker hostname `server` in browser JavaScript.

`server` is only used inside Nginx configuration.

---

# 33. Error handling

Create consistent reusable states:

```text
AppSkeleton
AppEmptyState
AppErrorState
```

Daily Map must have explicit states:

```text
loading
loaded with route
loaded with zero points
404 device
network/server error
```

No state should leave a blank page.

---

# 34. Accessibility baseline

Required:

- semantic buttons for actions;
- `aria-label` for icon-only buttons;
- keyboard focus styles;
- sufficient text contrast;
- timeline items selectable by keyboard where interactive;
- map controls remain accessible;
- no meaning conveyed by color alone.

---

# 35. Performance rules

Mandatory:

- do not create DOM marker per raw GPS point;
- do not re-create the MapLibre map instance on every reactive update;
- use GeoJSON source updates;
- keep map instance lifecycle isolated;
- avoid pushing full route arrays through large chains of reactive components every animation frame;
- timeline long lists should be designed so virtualization can be added later;
- do not recompute full route bounds on each playback frame.

---

# 36. CSS organization

Replace the current single large stylesheet pattern with:

```text
styles/
├── reset.css
├── tokens.css
├── typography.css
├── layout.css
├── map.css
└── main.css
```

Component-specific styles may remain scoped inside Vue components.

Global CSS should contain only true global rules.

---

# 37. CLI script changes

Update `scripts/lifetrail`.

Current invalid assumption:

```text
web == server
```

Remove that alias.

Required service commands:

```bash
scripts/lifetrail start all
scripts/lifetrail start web
scripts/lifetrail start server
scripts/lifetrail start postgres

scripts/lifetrail stop all
scripts/lifetrail stop web
scripts/lifetrail stop server
scripts/lifetrail stop postgres

scripts/lifetrail restart web
scripts/lifetrail restart server

scripts/lifetrail logs web
scripts/lifetrail logs server
scripts/lifetrail logs postgres
```

Recommended build syntax:

```bash
scripts/lifetrail build
scripts/lifetrail build web
scripts/lifetrail build server
scripts/lifetrail build all
```

If compatibility is needed, bare `build` may build both web and server.

---

# 38. Health/test command behavior

Update the test command so it validates the public Web entrypoint and the API path through Nginx.

At minimum verify:

```text
GET http://localhost:8080/
```

Also verify an API endpoint if a stable health endpoint exists.

If no dedicated API health endpoint exists, do not invent one as part of this UI task unless needed and explicitly documented.

---

# 39. README updates

Update root README to remove statements that:

```text
server builds and serves web
web is alias of server
```

Document:

```text
web       → Nginx + built Vue application
server    → Rust/Axum API
postgres  → PostgreSQL/PostGIS
```

Update example commands.

---

# 40. Architecture docs updates

Update at minimum:

```text
docs/project/codebase.md
docs/web/architecture.md
README.md
```

Ensure there is one consistent deployment model everywhere.

Do not leave documentation claiming the Rust server owns static web assets.

---

# 41. Migration strategy

Do not rewrite the entire frontend in one destructive step.

Recommended order:

```text
Current
│
├── App.vue
├── views/DailyView.vue
├── components/RouteMap.vue
├── queries/
└── styles.css
        │
        ▼
Step 1 — App shell
        │
        ▼
Step 2 — Sidebar + Topbar
        │
        ▼
Step 3 — DailyMapPage Option 2
        │
        ▼
Step 4 — Map feature decomposition
        │
        ▼
Step 5 — Timeline shell
        │
        ▼
Step 6 — Docker web service
        │
        ▼
Step 7 — Documentation/tests
        │
        ▼
Phase 2 — Playback + animated route + camera follow
```

Preserve working API integration throughout.

---

# 42. Implementation steps

## Step 1 — Dependencies

- add `pinia`;
- add `lucide-vue-next`;
- keep existing Vue Query/MapLibre/OpenAPI packages;
- ensure typecheck and test scripts still pass.

## Step 2 — Application shell

Create:

```text
AppLayout.vue
AppSidebar.vue
AppTopbar.vue
SidebarNav.vue
SidebarNavItem.vue
```

Move route pages below AppLayout.

## Step 3 — Design system

Create token/global style files.

Replace old page-level styles with Option 2 tokens.

## Step 4 — Routes

Create routes for:

```text
/
/devices
/devices/:deviceId
/devices/:deviceId/day/:date
/timeline
/reports
/settings
```

Do not break canonical Daily Map URL.

## Step 5 — Daily Map

Refactor current Daily View into DailyMapPage.

Preserve current query/API behavior.

Build:

```text
header
map workspace
timeline shell
stats
empty/loading/error states
```

## Step 6 — Map core

Move imperative MapLibre code into map/core modules.

Ensure map instance remains stable during updates.

## Step 7 — Timeline shell

Render Start/End for Phase 1.

Prepare event types for Phase 2.

## Step 8 — Docker split

Create:

```text
deploy/server.Dockerfile
deploy/web.Dockerfile
deploy/nginx.conf
```

Update compose.

## Step 9 — CLI

Update `scripts/lifetrail` so web is a real service.

## Step 10 — Docs/tests

Update architecture docs and acceptance tests.

---

# 43. Do not implement in this task

Unless explicitly required by existing code or tests, do **not** implement:

```text
trip detection
stop detection
route smoothing
map matching
media processing
audio/photo timeline ingestion
full Phase 2 playback backend
vector tiles
authentication redesign
public TLS termination
```

The frontend may contain clean extension points for these features.

---

# 44. Acceptance criteria — UI shell

The task is accepted when:

1. All main pages render inside a shared AppLayout.
2. Desktop sidebar visually follows Option 2.
3. Sidebar remains mounted across page navigation.
4. Active route is highlighted.
5. Daily Map is the primary polished page.
6. Topbar matches light clean UI style.
7. Existing device listing remains accessible.
8. Placeholder Timeline/Reports/Settings routes do not 404.
9. UI has loading/error/empty states.
10. No heavy UI framework is introduced.

---

# 45. Acceptance criteria — Daily Map

1. `/devices/:deviceId/day/:date` loads current Phase 1 Daily View data.
2. Route renders through MapLibre.
3. Start and End render correctly.
4. MapTiler configuration remains environment-driven.
5. Initial route fits inside the usable visible map area.
6. Timeline panel appears on the right on desktop.
7. Phase 1 timeline displays Start/End.
8. Daily stats display available values.
9. Zero-point days render a clean empty state.
10. Device 404 and server error states render inside AppLayout.
11. No DOM marker is created per raw GPS point.

---

# 46. Acceptance criteria — Docker

1. `docker compose up -d` starts `postgres`, `server`, and `web`.
2. `web` owns host port `8080`.
3. `server` is reachable from `web` as `server:8080`.
4. Browser can load `http://localhost:8080/`.
5. Vue Router deep links work through Nginx fallback.
6. `/api/v1/*` requests are proxied through Nginx to Rust server.
7. ESP32 ingest URL can still use `http://<LAN-IP>:8080/api/v1/device/batches`.
8. Rust server image no longer builds or contains web assets.
9. Web image runtime contains Nginx/static assets only.
10. PostgreSQL volume behavior is unchanged.

---

# 47. Acceptance criteria — scripts/docs

1. `scripts/lifetrail start web` starts the real web service.
2. `scripts/lifetrail logs web` shows Nginx/web logs.
3. `web` is no longer an alias of `server`.
4. README describes the new three-service architecture.
5. Web architecture docs match compose behavior.
6. No active document claims Rust serves the production SPA.

---

# 48. Phase 2 readiness criteria

The UI implementation is Phase 2-ready if:

1. Timeline and Map communicate through shared selection state, not direct component calls.
2. Playback state has a clear store/composable boundary.
3. Map layers reserve ownership for route progress/current position/events.
4. Camera logic is isolated in `map/core/camera.ts` or equivalent.
5. Timeline item type can be extended beyond Start/End.
6. Daily Map layout does not require redesign to add playback.
7. Right panel and bottom playback area are already represented structurally.

---

# 49. Testing expectations

At minimum include or preserve tests for:

```text
router navigation
AppLayout rendering
active sidebar item
Daily Map loading state
Daily Map empty state
Daily Map API error state
Daily Map loaded state
Docker/static route smoke test if available
```

Map rendering tests may mock MapLibre where needed.

Do not make browser tests depend on live MapTiler network access.

---

# 50. Quality rules

Agent must:

- keep TypeScript strict enough to avoid widespread `any`;
- avoid duplicating generated OpenAPI types;
- avoid mixing API fetch code directly inside presentation components;
- avoid global mutable MapLibre instances;
- avoid unnecessary reactivity around large GeoJSON arrays;
- keep reusable UI components small and generic;
- keep domain feature components inside feature folders;
- preserve current Phase 1 API contract;
- keep changes incremental and reviewable.

---

# 51. Final architecture baseline

```text
                          LifeTrail Web
┌──────────────────────────────────────────────────────────────┐
│ AppLayout                                                    │
│                                                              │
│ ┌──────────────┐ ┌──────────────────────────────────────────┐ │
│ │ Sidebar      │ │ Topbar                                   │ │
│ │              │ ├──────────────────────────────────────────┤ │
│ │ Overview     │ │                                          │ │
│ │ Daily Map    │ │ Router View                              │ │
│ │ Timeline     │ │                                          │ │
│ │ Devices      │ │ ┌───────────────────┬──────────────────┐ │ │
│ │ Reports      │ │ │ Map               │ Timeline         │ │ │
│ │ Settings     │ │ │                   │                  │ │ │
│ │              │ │ │ MapLibre          │ Events           │ │ │
│ │ System       │ │ │ + MapTiler        │                  │ │ │
│ └──────────────┘ │ ├───────────────────┼──────────────────┤ │ │
│                  │ │ Playback          │ Stats            │ │ │
│                  │ └───────────────────┴──────────────────┘ │ │
│                  └──────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘

                               │
                         TanStack Query
                               │
                               ▼
                         /api/v1/...
```

Deployment:

```text
                 LAN
                  │
                  ▼
          ┌──────────────┐
          │ web / nginx  │
          │    :8080     │
          └──────┬───────┘
                 │
          /api   │
                 ▼
          ┌──────────────┐
          │ Rust server  │
          │    :8080     │
          │   internal   │
          └──────┬───────┘
                 │
                 ▼
          ┌──────────────┐
          │ PostgreSQL   │
          │ + PostGIS    │
          └──────────────┘
```

---

# 52. Locked decisions

The following decisions are considered approved for this implementation unless a repository constraint makes one impossible:

1. Option 2 is the official Daily Map design baseline.
2. Persistent left sidebar is the primary desktop navigation.
3. Daily Map is the main product surface.
4. Vue 3 + TypeScript remains the frontend stack.
5. MapLibre GL JS remains the renderer.
6. MapTiler remains the default configurable basemap provider.
7. TanStack Vue Query manages server state.
8. Pinia is introduced for shared client/UI state only.
9. Lucide Vue is used for the icon system.
10. No heavy component framework is added.
11. Web becomes a separate Docker service.
12. Nginx serves the SPA and proxies `/api`.
13. Rust server no longer serves frontend static assets.
14. Public/LAN port remains `8080`.
15. Phase 1 API/device upload contract remains unchanged.
16. UI structure is Phase 2-ready for Timeline, playback, route animation and camera follow.

---

# 53. Agent completion report

When implementation is complete, the agent must report:

```text
1. Files added
2. Files changed
3. Architecture changes made
4. Docker topology changes
5. UI routes implemented
6. Tests added/updated
7. Commands used to verify
8. Any deviations from this specification
9. Remaining Phase 2 work
```

Do not silently deviate from the locked decisions.
