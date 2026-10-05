# LifeTrail — Server Architecture Specification

> **Status:** Proposed implementation baseline  
> **Project:** LifeTrail  
> **Scope:** `server/`  
> **Language:** Rust  
> **Framework:** Axum + Tokio  
> **Database:** PostgreSQL + PostGIS  
> **Last reviewed:** 2026-10-04

---

# 1. Mục tiêu và nguyên tắc Server

Tài liệu này là specification độc lập cho `server/` của LifeTrail. Server là **authoritative data plane** giữa LifeTrail Device và Web.

Trách nhiệm chính:

- authenticate device và user;
- nhận batch upload idempotent;
- validate và commit raw GPS atomically;
- giữ raw data immutable;
- chạy GPS processing/trip/stop/timeline ở background;
- cung cấp read model tối ưu cho Web;
- thực hiện spatial query bằng PostgreSQL + PostGIS;
- phát SSE invalidation khi derived data thay đổi;
- sinh OpenAPI làm contract cho Vue client;
- giữ khả năng self-host, backup và restore độc lập.

Baseline là **modular monolith**. Không tách microservice/Redis/Kafka nếu chưa có bottleneck hoặc requirement đã đo được.

---

# 2. System architecture — Server view

```text
LifeTrail Device
       │
 HTTPS batch upload
       │
       ▼
┌────────────────────────────┐
│ Rust Server                │
│ Axum + Tokio               │
│ Tower middleware           │
│ SQLx                       │
│ background processing      │
└─────────────┬──────────────┘
              │ transaction
              ▼
┌────────────────────────────┐
│ PostgreSQL + PostGIS       │
│ raw GPS                    │
│ ingest batches             │
│ processing jobs            │
│ trips / stops / timeline   │
│ media metadata             │
└─────────────┬──────────────┘
              │
       REST / GeoJSON / SSE
              │
              ▼
          Vue Web App
```

Durability boundary:

```text
Device POST
    ↓
auth + validate
    ↓
DB transaction
    ↓
raw GPS + ingest batch COMMIT
    ↓
ACK device
    ↓
background processing
```

ACK chỉ có nghĩa raw batch đã được commit bền vững; không chờ trip/stop/timeline processing hoàn tất.

---

# 3. Vai trò của Rust server

Server chịu trách nhiệm:

```text
device authentication
batch ingestion
idempotency
raw GPS persistence
processing queue
GPS filtering
route reconstruction
trip detection
stop detection
daily summary
timeline aggregation
media metadata
web authentication
REST API
SSE
```

Server không được biến device upload request thành một request xử lý toàn bộ pipeline nặng.

---

---

# 4. Vì sao chọn Axum

Axum phù hợp LifeTrail vì:

- tích hợp trực tiếp Tokio;
- dùng Tower middleware ecosystem;
- typed extractors;
- `State` cho shared app state;
- built-in SSE support;
- ít framework magic;
- dễ chia module;
- phù hợp REST service.

Server baseline:

```text
Axum
  ↓
Tower
  ↓
Tokio
```

---

---

# 5. Vì sao chọn SQLx thay ORM

LifeTrail sẽ có SQL spatial đặc thù:

```text
ST_DWithin
ST_AsGeoJSON
ST_MakePoint
ST_SetSRID
ST_MakeLine
ST_Simplify
ST_Envelope
ST_AsMVT
```

ORM abstraction không mang lại nhiều lợi ích ở các query này.

SQLx cho phép:

- query SQL trực tiếp;
- async PostgreSQL;
- pool;
- transaction;
- migration;
- compile-time checked query macros;
- UUID/time/JSON support.

Do đó chọn:

```text
SQLx
```

Không chọn ORM làm default.

---

---

# 6. Rust dependency stack

## Core runtime

```text
tokio
axum
```

## HTTP middleware

```text
tower
tower-http
```

## Database

```text
sqlx
```

## Serialization

```text
serde
serde_json
```

## Error

```text
thiserror
anyhow
```

## Observability

```text
tracing
tracing-subscriber
```

## API schema

```text
utoipa
```

## IDs/time

```text
uuid
time
```

## Spatial algorithms

```text
geo
```

## Security/auth

```text
argon2
secrecy
tower-sessions
subtle            # device token comparison if needed
```

---

---

# 7. Cargo.toml baseline

Ví dụ baseline:

```toml
[package]
name = "lifetrail-server"
version = "0.1.0"
edition = "2024"

[dependencies]
axum = { version = "0.8", features = ["macros"] }
tokio = { version = "1", features = ["full"] }

serde = { version = "1", features = ["derive"] }
serde_json = "1"

sqlx = {
  version = "0.9",
  default-features = false,
  features = [
    "runtime-tokio",
    "tls-rustls-ring-native-roots",
    "postgres",
    "macros",
    "migrate",
    "uuid",
    "time",
    "json",
  ]
}

uuid = { version = "1", features = ["v7", "serde"] }
time = { version = "0.3", features = ["serde", "formatting", "parsing", "macros"] }

thiserror = "2"
anyhow = "1"

tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

tower = "0.5"
tower-http = { version = "0.7", features = [
  "trace",
  "cors",
  "compression-gzip",
  "request-id",
  "timeout",
] }

utoipa = { version = "6", features = ["axum_extras", "uuid", "time"] }

geo = "0.33"

argon2 = "0.6"
secrecy = "0.10"
tower-sessions = "0.15"
subtle = "2.6"
```

Không copy version cụ thể mù quáng về sau.

`Cargo.lock` là canonical dependency resolution cho deployment.

---

---

# 8. Rust toolchain policy

Tạo:

```text
server/rust-toolchain.toml
```

Ví dụ:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

Production build phải dùng cùng stable toolchain đã qua CI.

Không dùng nightly nếu không có ADR giải thích.

---

---

# 9. Server directory structure

Giai đoạn đầu nên dùng một crate modular monolith:

```text
server/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── .env.example
├── migrations/
│
├── src/
│   ├── main.rs
│   ├── lib.rs
│   │
│   ├── app.rs
│   ├── config.rs
│   ├── error.rs
│   │
│   ├── http/
│   │   ├── mod.rs
│   │   ├── router.rs
│   │   ├── middleware.rs
│   │   ├── response.rs
│   │   └── sse.rs
│   │
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── device.rs
│   │   └── password.rs
│   │
│   ├── devices/
│   │   ├── mod.rs
│   │   ├── model.rs
│   │   ├── repo.rs
│   │   ├── service.rs
│   │   └── routes.rs
│   │
│   ├── ingest/
│   │   ├── mod.rs
│   │   ├── dto.rs
│   │   ├── validate.rs
│   │   ├── service.rs
│   │   └── routes.rs
│   │
│   ├── gps/
│   │   ├── mod.rs
│   │   ├── model.rs
│   │   ├── repo.rs
│   │   ├── filter.rs
│   │   └── route.rs
│   │
│   ├── processing/
│   │   ├── mod.rs
│   │   ├── worker.rs
│   │   ├── jobs.rs
│   │   ├── daily.rs
│   │   ├── trips.rs
│   │   └── stops.rs
│   │
│   ├── timeline/
│   │   ├── mod.rs
│   │   ├── model.rs
│   │   ├── repo.rs
│   │   ├── service.rs
│   │   └── routes.rs
│   │
│   ├── map/
│   │   ├── mod.rs
│   │   ├── geojson.rs
│   │   └── routes.rs
│   │
│   ├── media/
│   │   ├── mod.rs
│   │   ├── storage.rs
│   │   ├── model.rs
│   │   └── routes.rs
│   │
│   ├── db/
│   │   ├── mod.rs
│   │   └── pool.rs
│   │
│   └── openapi.rs
│
└── tests/
    ├── ingestion.rs
    ├── idempotency.rs
    ├── daily_view.rs
    └── auth.rs
```

---

---

# 10. Layering convention

Trong mỗi feature:

```text
route
  ↓
service
  ↓
repository
  ↓
PostgreSQL
```

Không cho handler chứa business logic lớn.

Ví dụ:

```rust
async fn upload_batch(
    State(state): State<AppState>,
    device: AuthenticatedDevice,
    Json(request): Json<UploadBatchRequest>,
) -> Result<Json<UploadBatchResponse>, ApiError> {
    let response = state
        .ingest_service
        .upload_batch(device, request)
        .await?;

    Ok(Json(response))
}
```

Validation/business/transaction logic nằm trong service.

---

---

# 11. AppState

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Arc<AppConfig>,
    pub events: EventBus,
}
```

Không nhét mutable global domain state vào `Arc<Mutex<_>>` nếu database đã là source of truth.

---

---

# 12. Configuration

Config qua environment variables.

Ví dụ:

```env
LT_BIND_ADDR=0.0.0.0:8080
LT_DATABASE_URL=postgres://...
LT_PUBLIC_BASE_URL=https://lifetrail.example.com
LT_COOKIE_SECURE=true
LT_SESSION_SECRET=...
LT_MAP_DEFAULT_TIMEZONE=Asia/Ho_Chi_Minh
LT_PROCESSING_WORKERS=2
```

Không commit secret.

`.env` chỉ dùng development.

Production dùng environment/secrets manager.

---

---

# 13. Database baseline

Docker/deployment nên pin:

```text
PostgreSQL 18.x
PostGIS 3.6.x stable
```

Không dùng PostGIS prerelease 3.7 RC cho production baseline.

---

---

# 14. Database extensions

Migration đầu tiên:

```sql
CREATE EXTENSION IF NOT EXISTS postgis;
```

Optional tương lai:

```sql
CREATE EXTENSION IF NOT EXISTS pg_trgm;
```

cho search text/place name nếu cần.

---

---

# 15. IDs

Server-generated entities dùng UUID v7.

Ví dụ:

```text
device.id
user.id
trip.id
stop.id
timeline_event.id
media.id
```

Lợi ích:

- globally unique;
- sortable theo time tốt hơn UUID v4;
- phù hợp DB key.

`device_id` protocol có thể là UUID string.

Raw `gps_points.id` không cần globally unique ID. Vì đây là bảng high-volume, dùng `bigint GENERATED ALWAYS AS IDENTITY` để giảm kích thước row/index và tăng locality.

---

## ID policy theo data volume

Không dùng cùng một loại primary key cho mọi bảng chỉ để đồng nhất hình thức.

```text
users/devices/trips/stops/events/media
    → UUID v7

gps_points
    → bigint identity
```

Lý do raw GPS là high-volume append-only table. `bigint` nhỏ hơn UUID, index gọn hơn và không cần ID mang ý nghĩa cross-system. Idempotency của ingest nằm ở `(device_id, batch_id)`, không phụ thuộc GPS point UUID.

---

---

# 16. Core database schema

## users

```sql
CREATE TABLE users (
    id uuid PRIMARY KEY,
    email text NOT NULL UNIQUE,
    password_hash text NOT NULL,
    timezone text NOT NULL DEFAULT 'UTC',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
```

---

## devices

```sql
CREATE TABLE devices (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users(id),
    name text NOT NULL,
    token_hash bytea NOT NULL,
    enabled boolean NOT NULL DEFAULT true,
    last_seen_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
```

Không lưu device token plaintext.

---

## ingest_batches

```sql
CREATE TABLE ingest_batches (
    id uuid PRIMARY KEY,
    device_id uuid NOT NULL REFERENCES devices(id),
    batch_id text NOT NULL,
    schema_version text NOT NULL,
    first_record_at timestamptz,
    last_record_at timestamptz,
    record_count integer NOT NULL,
    received_at timestamptz NOT NULL DEFAULT now(),
    payload_hash bytea,
    UNIQUE (device_id, batch_id)
);
```

`UNIQUE(device_id, batch_id)` là idempotency constraint cuối cùng ở database.

---

---

# 17. Raw GPS schema

```sql
CREATE TABLE gps_points (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    device_id uuid NOT NULL REFERENCES devices(id),
    source_batch_id uuid NOT NULL REFERENCES ingest_batches(id),

    recorded_at timestamptz NOT NULL,

    latitude double precision NOT NULL,
    longitude double precision NOT NULL,
    altitude_m real,
    speed_mps real,
    course_deg real,
    fix_quality smallint NOT NULL,
    satellites smallint,
    hdop real,

    geom geometry(Point, 4326) NOT NULL,

    quality_state text NOT NULL DEFAULT 'raw',
    created_at timestamptz NOT NULL DEFAULT now()
);
```

Raw fields không overwrite sau khi ingest.

Nếu processing phát hiện outlier:

```text
quality_state = suspected_outlier
```

không delete raw point.

---

---

# 18. GPS indexes

```sql
CREATE INDEX gps_points_device_time_idx
ON gps_points (device_id, recorded_at);

CREATE INDEX gps_points_geom_gist_idx
ON gps_points
USING GIST (geom);

CREATE INDEX gps_points_batch_idx
ON gps_points (source_batch_id);
```

Index `(device_id, recorded_at)` quan trọng cho daily queries.

GiST dùng cho spatial queries.

---

---

# 19. Insert geometry

Không trust geometry từ device.

Server tạo geometry từ validated lon/lat:

```sql
ST_SetSRID(ST_MakePoint($longitude, $latitude), 4326)
```

Device gửi:

```text
lat
lon
```

Server chịu trách nhiệm spatial representation.

---

---

# 20. Trip schema

```sql
CREATE TABLE trips (
    id uuid PRIMARY KEY,
    device_id uuid NOT NULL REFERENCES devices(id),
    start_at timestamptz NOT NULL,
    end_at timestamptz NOT NULL,
    distance_m double precision NOT NULL,
    duration_s bigint NOT NULL,
    geometry geometry(LineString, 4326) NOT NULL,
    algorithm_version text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
```

Trip là derived data.

Có thể regenerate.

---

---

# 21. Stop schema

```sql
CREATE TABLE stops (
    id uuid PRIMARY KEY,
    device_id uuid NOT NULL REFERENCES devices(id),
    start_at timestamptz NOT NULL,
    end_at timestamptz NOT NULL,
    duration_s bigint NOT NULL,
    center geometry(Point, 4326) NOT NULL,
    radius_m real,
    algorithm_version text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
```

---

---

# 22. Timeline event schema

```sql
CREATE TABLE timeline_events (
    id uuid PRIMARY KEY,
    device_id uuid NOT NULL REFERENCES devices(id),
    event_type text NOT NULL,
    occurred_at timestamptz NOT NULL,
    location geometry(Point, 4326),
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    source_type text NOT NULL,
    source_id uuid,
    algorithm_version text,
    created_at timestamptz NOT NULL DEFAULT now()
);
```

Event type:

```text
GPS
TRIP_START
TRIP_END
STOP
PHOTO
AUDIO
CUSTOM
```

Không nhất thiết materialize từng raw GPS point thành Timeline Event cho UI.

`GPS` event abstraction có thể tồn tại trong protocol/domain, nhưng UI timeline không cần 86k events/day.

---

---

# 23. Processing job table

Phase 1 không cần Redis.

Có thể dùng PostgreSQL làm durable queue nhỏ:

```sql
CREATE TABLE processing_jobs (
    id uuid PRIMARY KEY,
    job_type text NOT NULL,
    device_id uuid NOT NULL,
    date date,
    payload jsonb NOT NULL DEFAULT '{}'::jsonb,
    state text NOT NULL DEFAULT 'pending',
    attempts integer NOT NULL DEFAULT 0,
    available_at timestamptz NOT NULL DEFAULT now(),
    locked_at timestamptz,
    locked_by text,
    last_error text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
```

Worker claim bằng transaction + `FOR UPDATE SKIP LOCKED`.

Không thêm Redis chỉ để chạy một worker.

---

---

# 24. Device upload transaction

Flow:

```text
POST batch
   │
   ▼
authenticate device
   │
   ▼
validate schema/batch
   │
   ▼
begin transaction
   │
   ├── insert ingest_batches
   │
   ├── duplicate?
   │      └── return previous ACK
   │
   ├── bulk insert raw GPS
   │
   ├── enqueue daily processing job
   │
   └── commit
          │
          ▼
      ACK committed
```

ACK không chờ trip/stop calculation.

---

---

# 25. Batch upload endpoint

```text
POST /api/v1/device/batches
```

Headers:

```text
Authorization: Bearer <device-token>
Content-Type: application/x-ndjson
X-LifeTrail-Batch-Id: gps_000123
X-LifeTrail-Schema: gps.v1
```

Hoặc metadata envelope tùy protocol đã khóa.

Nếu firmware đang dùng NDJSON file, server nên hỗ trợ streaming body thay vì buffer toàn bộ request vào RAM.

---

---

# 26. Streaming ingestion

Không làm mặc định:

```text
request body 10 MB
     ↓
Vec<u8> 10 MB
     ↓
parse
```

Ưu tiên:

```text
HTTP body stream
     ↓
line decoder
     ↓
validation
     ↓
batched DB insert
```

Tuy nhiên transaction semantics phải rõ:

- không ACK nếu parse lỗi giữa batch;
- không partial-commit nếu contract batch là atomic;
- request size limit bắt buộc.

---

---

# 27. Bulk insert

Không chạy 80k `INSERT` round-trip riêng.

Lựa chọn theo thứ tự:

1. SQLx `COPY FROM STDIN` cho batch lớn;
2. chunked multi-row INSERT;
3. individual insert chỉ cho prototype rất nhỏ.

SQLx PostgreSQL có COPY support.

Phase 1 có thể bắt đầu multi-row insert rồi benchmark.

---

---

# 28. Validation pipeline

Mỗi GPS record:

```text
parse
  ↓
schema validation
  ↓
range validation
  ↓
quality metadata validation
  ↓
raw insert
```

Range:

```text
latitude  [-90, 90]
longitude [-180, 180]
fix_quality >= 0
satellites >= 0
hdop >= 0 if present
speed_mps >= 0 if present
course_deg [0, 360) if present
```

Không loại point chỉ vì thuật toán nghĩ nó xấu.

Schema-invalid và physically-impossible là hai khái niệm khác nhau.

---

---

# 29. Processing architecture

```text
Raw GPS committed
       │
       ▼
processing_jobs
       │
       ▼
worker
       │
       ├── quality classification
       ├── filter trajectory
       ├── route reconstruction
       ├── distance
       ├── stop detection
       ├── trip detection
       ├── daily summary
       └── timeline materialization
```

Mọi derived entity lưu:

```text
algorithm_version
```

để hỗ trợ reprocess.

---

---

# 30. Worker strategy

Phase 1 worker có thể nằm cùng binary nhưng task riêng:

```text
lifetrail-server
├── HTTP tasks
└── processing worker tasks
```

Hoặc CLI mode:

```text
lifetrail-server api
lifetrail-server worker
```

Code processing phải không phụ thuộc Axum handler.

Khi scale có thể chạy:

```text
1 API instance
N worker instances
```

trên cùng database.

---

---

# 31. Rust geo library

Dùng:

```text
geo
```

cho algorithm chạy trong Rust:

- Haversine/Geodesic distance;
- geometry transforms;
- clustering utilities nếu phù hợp;
- simplification thử nghiệm.

Không dùng API Haversine deprecated.

Pattern mới:

```rust
use geo::{Distance, Haversine, Point};

let a = Point::new(lon_a, lat_a);
let b = Point::new(lon_b, lat_b);

let distance_m = Haversine.distance(a, b);
```

PostGIS vẫn phụ trách spatial query trên database.

---

---

# 32. Rust vs PostGIS responsibility

## Rust

Tốt cho:

```text
sequential GPS processing
stateful trip detection
stateful stop detection
filter pipelines
algorithm versioning
```

## PostGIS

Tốt cho:

```text
bounding-box queries
within-radius queries
spatial index
GeoJSON conversion
spatial aggregation
vector tiles
```

Không cố ép toàn bộ algorithm sang SQL.

Không kéo toàn bộ history ra Rust chỉ để query spatial đơn giản.

---

---

# 33. Stop detection

Prototype:

```text
radius 30–50 m
minimum duration 3–5 min
```

Nhưng đây phải là config/algorithm parameter.

Derived record lưu version:

```text
stop-v1
```

Khi có `stop-v2`:

```text
raw GPS
  ↓
reprocess
  ↓
replace derived stops
```

Raw data không đổi.

---

---

# 34. Route generation

Daily route không nên được tạo ở browser từ mọi raw point mỗi lần.

Processing tạo route representation hoặc API query tạo từ filtered points.

```text
raw points
    ↓
quality filter
    ↓
filtered points
    ↓
LineString
    ↓
optional simplification
    ↓
GeoJSON
```

Server trả route phù hợp zoom/use case.

---

---

# 35. GeoJSON từ PostGIS

PostGIS có thể trả GeoJSON trực tiếp bằng:

```sql
ST_AsGeoJSON(...)
```

API không cần hand-build mọi coordinate array nếu query đã có geometry.

Ví dụ:

```sql
SELECT ST_AsGeoJSON(geometry)::jsonb AS geometry
FROM trips
WHERE id = $1;
```

---

---

# 36. Spatial radius query

Dùng `ST_DWithin` thay vì tự load toàn bộ points rồi tính Rust.

Ví dụ:

```sql
SELECT id, recorded_at
FROM gps_points
WHERE device_id = $1
  AND ST_DWithin(
        geom::geography,
        ST_SetSRID(ST_MakePoint($2, $3), 4326)::geography,
        $4
      );
```

Với geography, distance tính bằng meters.

---

---

# 37. Server API groups

```text
/api/v1/auth/*
/api/v1/devices/*
/api/v1/device/*          # firmware-facing
/api/v1/timeline/*
/api/v1/media/*
/api/v1/events            # SSE
```

Phân biệt rõ:

```text
/device/*
```

cho hardware authentication và:

```text
/devices/*
```

cho authenticated web user.

---

---

# 38. Device endpoints

```text
POST /api/v1/device/batches
POST /api/v1/device/media/init
PUT  /api/v1/device/media/:uploadId
POST /api/v1/device/media/:uploadId/complete
GET  /api/v1/device/config
```

Media endpoints là future phase.

---

---

# 39. Web endpoints Phase 1

```text
GET /api/v1/me

GET /api/v1/devices
GET /api/v1/devices/:deviceId

GET /api/v1/devices/:deviceId/days/:date

GET /api/v1/devices/:deviceId/gps
    ?from=<UTC>
    &to=<UTC>
    &quality=all|valid
```

Raw GPS endpoint là debug/inspection endpoint.

Daily web view không nên phụ thuộc raw GPS endpoint.

---

---

# 40. Web endpoints Phase 2

```text
GET /api/v1/devices/:deviceId/timeline?date=...
GET /api/v1/devices/:deviceId/trips?date=...
GET /api/v1/devices/:deviceId/stops?date=...
GET /api/v1/trips/:tripId
GET /api/v1/stops/:stopId
```

---

---

# 41. API error format

Mọi API error dùng một envelope ổn định:

```json
{
  "error": {
    "code": "invalid_batch",
    "message": "Batch contains invalid GPS records",
    "request_id": "...",
    "details": {
      "line": 42
    }
  }
}
```

Web logic dựa vào `code`, không parse `message`.

---

---

# 42. ApiError

Rust:

```rust
pub enum ApiError {
    Unauthorized,
    Forbidden,
    NotFound,
    Validation(ValidationError),
    Conflict(ConflictError),
    Database(sqlx::Error),
    Internal(anyhow::Error),
}
```

Implement `IntoResponse` ở một chỗ.

Không tạo JSON error thủ công ở từng handler.

---

---

# 43. OpenAPI

Dùng:

```text
utoipa
```

Server generate:

```text
protocol/openapi/lifetrail-v1.json
```

CI kiểm tra schema generated có thay đổi hay không.

Nếu API change mà OpenAPI chưa update, CI fail.

---

---

# 44. OpenAPI ownership

Recommended flow:

```text
Rust DTO + endpoint annotations
        ↓
utoipa
        ↓
OpenAPI JSON
        ↓
committed artifact / CI artifact
        ↓
openapi-typescript
        ↓
Vue client types
```

Không định nghĩa schema JSON/YAML bằng tay song song với Rust DTO trừ khi team chủ động chọn schema-first sau này.

---

---

# 45. Web authentication

LifeTrail web là browser app cùng origin.

Khuyến nghị:

```text
server-side session
+
Secure HttpOnly SameSite cookie
```

thay vì lưu JWT trong `localStorage`.

Stack:

```text
argon2
+
tower-sessions
+
custom Axum auth middleware
```

`axum-login` có thể được đánh giá lại sau nếu release tương thích trực tiếp với nhánh `tower-sessions` đang pin. Không đưa nó vào dependency baseline hiện tại để tránh hai version session types cùng tồn tại.

---

---

# 46. Password policy

Password hash:

```text
Argon2id
```

Không lưu:

```text
SHA256(password)
MD5
plaintext
```

Hash string theo PHC format.

---

---

# 47. Session policy

Cookie:

```text
HttpOnly
Secure              production
SameSite=Lax        default
Path=/
```

Session data server-side.

Sau login:

```text
cycle session id
```

Sau logout:

```text
delete/flush session
```

---

---

# 48. CSRF

Nếu dùng same-origin cookie session:

- state-changing endpoints phải kiểm tra Origin/CSRF policy;
- không bật permissive CORS;
- SameSite cookie là một lớp bảo vệ, không phải toàn bộ CSRF strategy.

Phase 1 có thể dùng custom CSRF token cho POST/PUT/DELETE web endpoints nếu app có nhiều mutations.

Device API không dùng browser cookie.

---

---

# 49. Device authentication

Device token khác hoàn toàn user session.

Provision:

```text
device_id
+
random device token
```

Device lưu token trong NVS.

Server chỉ lưu hash.

Request:

```text
Authorization: Bearer <token>
```

Lookup theo device ID hoặc token identifier + constant-time verification.

Device token phải revoke/rotate được.

---

---

# 50. TLS

Production:

```text
HTTPS bắt buộc
```

Recommended deployment:

```text
Internet / LAN
     ↓
Caddy
     ↓
Rust HTTP
```

Caddy terminate TLS.

Rust server có thể bind private interface/container network.

---

---

# 51. CORS

Production same-origin:

```text
CORS không cần hoặc cực kỳ restricted
```

Development:

Vite proxy `/api` để tránh mở CORS permissive.

Không dùng:

```text
Access-Control-Allow-Origin: *
```

với credentialed auth.

---

---

# 52. Request limits

Bắt buộc giới hạn:

```text
JSON body size
batch upload size
media upload size
request timeout
concurrent uploads/device
```

Không để device gửi unlimited body.

Giới hạn batch được protocol định nghĩa.

---

---

# 53. Middleware stack

Ví dụ:

```text
request id
  ↓
tracing
  ↓
timeout
  ↓
compression             web API responses
  ↓
security headers
  ↓
auth
  ↓
router
```

`tower-http` dùng cho:

- TraceLayer;
- CompressionLayer;
- request IDs;
- timeout;
- CORS nếu thật sự cần.

Không gzip request upload GPS trừ khi protocol thống nhất và benchmark chứng minh lợi ích.

---

---

# 54. Observability

Dùng:

```text
tracing
tracing-subscriber
```

Log production nên structured JSON.

Ví dụ field:

```text
request_id
device_id
batch_id
user_id
route
status
latency_ms
record_count
```

Không log:

```text
password
device token
session cookie
raw Authorization header
```

---

---

# 55. Request ID

Mọi request tạo/nhận request ID.

Response:

```text
X-Request-Id: ...
```

API error trả cùng request ID.

Điều này rất quan trọng khi debug firmware retry.

---

---

# 56. Metrics

Phase 1 tối thiểu có counters/log-derived metrics:

```text
http_requests_total
ingest_batches_total
ingest_records_total
ingest_duplicates_total
ingest_failures_total
processing_jobs_pending
processing_jobs_failed
processing_duration_ms
sse_connections
```

Có thể thêm Prometheus exporter sau.

Không biến metrics backend thành dependency bắt buộc để server chạy.

---

---

# 57. SSE server

Axum có SSE response support.

Concept:

```text
App event bus
     │
     ├── day.updated
     ├── device.updated
     └── processing.completed
             │
             ▼
       connected browsers
```

SSE endpoint:

```text
GET /api/v1/events
```

Event không chứa sensitive payload lớn.

---

---

# 58. Event bus Phase 1

Một process server có thể dùng Tokio broadcast channel:

```text
tokio::sync::broadcast
```

Nhược điểm:

- event mất khi process restart;
- không cross-instance.

Điều này chấp nhận được vì SSE chỉ để invalidate cache.

Source of truth vẫn DB.

Nếu event mất:

```text
web refresh/refetch
```

vẫn đúng.

---

---

# 59. Multi-instance SSE future

Khi nhiều API instances:

Có thể dùng:

```text
PostgreSQL LISTEN/NOTIFY
```

SQLx hỗ trợ PostgreSQL listener.

Hoặc Redis pub/sub khi scale lớn.

Không thêm Redis từ đầu.

---

---

# 60. Daily processing state

Web cần biết derived data đã sẵn sàng hay chưa.

Ví dụ:

```text
pending
processing
ready
failed
```

Daily API:

```json
{
  "processing_state": "processing",
  "raw_data_complete": true
}
```

Web có thể hiện:

```text
Data synchronized. Route analysis is being updated.
```

Không block device ACK vì UI processing.

---

---

# 61. Processing invalidation

Khi batch mới ảnh hưởng ngày `2026-10-04`:

```text
insert raw
    ↓
mark daily derived stale
    ↓
enqueue job
    ↓
worker reprocess
    ↓
write derived
    ↓
emit day.updated
```

Nếu batch cũ upload muộn sau vài ngày vẫn reprocess đúng ngày lịch sử.

---

---

# 62. Timezone model

## Stored timestamps

```text
UTC timestamptz
```

## User preference

```text
users.timezone = IANA timezone
```

Ví dụ:

```text
Asia/Ho_Chi_Minh
```

## Daily query

Request:

```text
GET /devices/:id/days/2026-10-04
```

Server resolve local-day boundaries theo user timezone:

```text
local 2026-10-04 00:00
      ↓
UTC from

local 2026-10-05 00:00
      ↓
UTC to
```

Không dùng timezone OS của server.

---

---

# 63. Media architecture future

Photo/audio không lưu trực tiếp trong PostgreSQL `bytea` nếu file lớn.

Database chỉ giữ metadata.

```text
media
├── id
├── device_id
├── event timestamp
├── location
├── storage_key
├── content_type
├── size
├── checksum
└── metadata
```

Blob:

```text
Storage abstraction
├── local filesystem
└── S3-compatible object storage
```

Self-host production có thể dùng MinIO/S3-compatible storage.

---

---

# 64. Media storage interface

Rust domain interface:

```rust
#[async_trait]
pub trait MediaStore {
    async fn put(...);
    async fn get(...);
    async fn delete(...);
}
```

Implementation:

```text
LocalMediaStore
S3MediaStore
```

Không để API handler biết path vật lý.

---

---

# 65. Database migration

Dùng SQLx migrations:

```text
server/migrations/
```

Naming:

```text
0001_init.sql
0002_postgis.sql
0003_ingest_batches.sql
0004_gps_points.sql
0005_processing_jobs.sql
```

Migration chạy trong CI test database.

Production migration phải backup và có deployment procedure.

---

---

# 66. SQLx query policy

Ưu tiên:

```rust
sqlx::query!
sqlx::query_as!
```

cho static query quan trọng.

Dynamic filter phức tạp có thể dùng QueryBuilder.

Không nối SQL bằng string từ user input.

---

---

# 67. Transaction boundaries

Transaction dùng cho invariants thực sự.

Device batch:

```text
batch metadata
+
raw GPS rows
+
processing job
```

nên atomic.

Không giữ DB transaction mở trong lúc:

- gọi external service;
- upload object storage;
- chạy GPS processing dài;
- chờ SSE.

---

---

# 68. Idempotency

Database constraint là lớp cuối:

```sql
UNIQUE(device_id, batch_id)
```

Application response khi duplicate phải deterministic.

Ví dụ:

```json
{
  "batch_id": "gps_000123",
  "status": "committed",
  "duplicate": true
}
```

Device có thể coi đây là ACK hợp lệ.

---

---

# 69. Payload integrity

Recommended batch metadata:

```text
batch_id
schema_version
record_count
first_timestamp
last_timestamp
payload_sha256 optional
```

Server có thể verify:

```text
record_count
hash
```

để phát hiện corruption hoặc retry với payload khác cùng batch ID.

Nếu same `(device_id,batch_id)` nhưng hash khác:

```text
409 Conflict
```

và log severity cao.

---

---

# 70. API pagination

List endpoints phải pagination.

Ví dụ:

```text
GET /devices/:id/trips?cursor=...&limit=50
```

Ưu tiên cursor pagination cho timeline lớn.

Không trả toàn bộ history trong một request.

---

---

# 71. Map query strategy

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

# 72. PostGIS distance rules

Không dùng EPSG:4326 geometry degree như meter.

Cho radius query đơn giản:

```text
geom::geography
+
ST_DWithin(..., meters)
```

hoặc transform phù hợp nếu cần performance/precision đặc biệt.

---

---

# 73. Data retention

Raw GPS mặc định giữ lại.

Derived data có thể regenerate.

Retention nên phân biệt:

```text
raw GPS              keep
processed routes     regenerable
trips/stops           regenerable
API caches            discardable
media                 keep per user policy
logs                  rotate
```

---

---

# 74. Backup

Backup tối thiểu:

```text
PostgreSQL
media storage
server config/secrets separately
```

Không cần backup derived caches nếu có thể rebuild, nhưng vẫn nằm chung DB nên backup database toàn bộ đơn giản hơn.

Test restore quan trọng hơn chỉ có backup job.

---

---

# 75. Development Docker Compose

Ví dụ services:

```text
postgres
server
web-dev optional
```

Database image phải có PostGIS.

Production:

```text
reverse-proxy
web-static
server
postgres-postgis
media-store optional
```

---

---

# 76. Deployment topology — single host

Đây là topology khuyến nghị ban đầu:

```text
                 Internet / LAN
                       │
                       ▼
                 web (Nginx)
                  HTTP :8080
                 ┌─────┴─────┐
                 │           │
                 ▼           ▼
          Vue static       /api → proxy
          (+ SPA fallback)        │
                                  ▼
                             Rust server
                             (API only)
                                  │
                                  ▼
                          PostgreSQL/PostGIS
                                  │
                                  ▼
                         media volume/MinIO
```

The `web` service owns the public port and reverse-proxies `/api/*` to the
Rust server on the Compose network. The server image contains no web assets;
`LT_STATIC_DIR` only re-enables in-server static hosting for special setups.

Ưu điểm:

- dễ self-host;
- cùng origin;
- ít service;
- backup đơn giản;
- không cần Kubernetes.

---

---

# 77. Không dùng Kubernetes ở Phase 1

LifeTrail chưa cần:

```text
Kubernetes
service mesh
Kafka
Redis cluster
microservices
```

Các hệ này chỉ thêm operational complexity.

Scale bottleneck thực tế phải được đo trước.

---

---

# 78. CI — Rust

Pipeline:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
SQLx migration test
integration tests with PostGIS
cargo build --release
```

Có thể thêm:

```text
cargo audit
cargo deny
```

cho dependency/security policy.

---

---

# 79. Server testing

## Unit

```text
GPS validation
batch validation
stop detector
trip detector
distance calculations
error mapping
```

## Integration

Test với PostgreSQL + PostGIS thật.

```text
batch ingestion
idempotency
transaction rollback
GeoJSON endpoint
ST_DWithin queries
daily view
processing job claims
```

Không mock SQL cho các query PostGIS quan trọng.

---

---

# 80. Device simulator

Tạo tool:

```text
tools/device-simulator/
```

Có thể viết Rust hoặc Python host-side.

Simulator:

```text
read .ndjson
sign/auth request
upload batch
simulate timeout
retry same batch
corrupt ACK
late batch
```

Backend development không phụ thuộc hardware luôn cắm USB.

---

---

# 81. API compatibility

Route base:

```text
/api/v1
```

Breaking API change:

```text
/api/v2
```

Không đổi semantics âm thầm trong v1 nếu firmware cũ vẫn dùng.

Device protocol có version riêng:

```text
gps.v1
sync.v1
event.v1
```

---

---

# 82. Server startup sequence

```text
load config
   ↓
initialize tracing
   ↓
connect PostgreSQL
   ↓
verify required extension/version
   ↓
run/verify migrations
   ↓
create AppState
   ↓
start worker
   ↓
bind HTTP listener
```

Nếu database unavailable:

```text
fail fast
```

Không start HTTP server giả healthy.

---

---

# 83. Health endpoints

```text
GET /health/live
GET /health/ready
```

`live`:

```text
process alive
```

`ready`:

```text
DB reachable
required dependencies ready
```

Không expose sensitive config.

---

---

# 84. Read model vs raw model

Điểm quan trọng:

```text
raw model
    ≠
web read model
```

Raw GPS schema tối ưu durability/audit/reprocessing.

Daily API tối ưu UI.

Không bắt Vue biết database normalization.

---

---

# 85. Daily read model generation

Có hai chiến lược:

## Option A — Query-time

Daily endpoint join/query derived tables mỗi request.

Ưu điểm:

- đơn giản;
- ít duplicate data.

Chọn cho Phase 1.

## Option B — Materialized JSON/read table

Khi request rất nhiều:

```text
daily_views
```

có thể cache summary/read model.

Chỉ làm sau khi đo.

---

---

# 86. Map route simplification

Không làm destructive simplification trên raw data.

Có thể có nhiều resolution:

```text
route_full
route_medium
route_overview
```

hoặc simplify query-time.

Raw GPS vẫn giữ nguyên.

---

---

# 87. Self-host map path

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

# 88. Security headers

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

# 89. Privacy

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

# 90. Analytics

Không thêm third-party session replay vào app mặc định.

Nếu có telemetry:

- opt-in;
- không gửi location/history;
- self-host metrics nếu có thể.

---

---

# 91. Definition of Done — Server Phase 1

Server Phase 1 hoàn thành khi:

- Rust server start ổn định;
- PostgreSQL/PostGIS migration tự động test;
- device authentication hoạt động;
- batch ingestion atomic;
- `(device_id,batch_id)` idempotent;
- raw GPS immutable;
- invalid request không partial commit;
- query device/time hoạt động;
- daily route API trả GeoJSON;
- summary point count/duration/distance hoạt động;
- request tracing có request ID;
- integration tests chạy với PostGIS;
- backup/restore procedure tối thiểu được document.

---

---

# 92. Library decision matrix — Server

| Need | Chọn | Lý do |
|---|---|---|
| HTTP | Axum | Tower/Tokio native |
| Async | Tokio | ecosystem standard cho Axum |
| DB | SQLx | direct SQL + compile checks |
| DB engine | PostgreSQL | durability/query maturity |
| Spatial | PostGIS | spatial index + GeoJSON/MVT |
| Serialization | Serde | Rust standard ecosystem |
| Middleware | tower-http | tracing/compression/CORS/etc |
| Observability | tracing | structured async-aware spans |
| OpenAPI | utoipa | Rust code → OpenAPI |
| Geometry algorithm | geo | Haversine/geodesic/tools |
| Password | argon2 | Argon2id |
| Sessions | tower-sessions | server-side browser sessions |
| UUID | uuid v7 | sortable unique IDs |
| Time | time | UTC/time types + Serde/SQLx |

---

---

# 93. Dependencies không nên thêm sớm

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

# 94. Implementation roadmap — Server

## Phase S0 — Skeleton

```text
Axum
Tokio
SQLx
PostGIS
configuration
health endpoints
tracing
migrations
```

## Phase S1 — GPS ingestion

```text
device identity/auth
ingest_batches
gps_points
batch validation
atomic transaction
idempotency
ACK
```

## Phase S2 — Daily read model

```text
query by device/time
quality filtering
summary
LineString
GeoJSON daily endpoint
OpenAPI generation
```

## Phase S3 — Processing worker

```text
processing_jobs
algorithm_version
retry/failure state
distance calculation
SSE invalidation
```

## Phase S4 — Trips/stops/timeline

```text
trip detector
stop detector
timeline events
regeneration/reprocessing
```

## Phase S5 — Media

```text
media metadata
object storage abstraction
protected/signed delivery
photo/audio TimelineEvent
```

## Phase S6 — Large history

```text
spatial aggregation
MVT/vector tile endpoints
Martin/PostGIS tile path if needed
retention/partitioning based on measured scale
```

---

# 95. Source/reference notes — Server

- Axum: https://docs.rs/axum/
- Tokio: https://docs.rs/tokio/
- SQLx: https://docs.rs/sqlx/
- Serde: https://serde.rs/
- tower-http: https://docs.rs/tower-http/
- tracing: https://docs.rs/tracing/
- utoipa: https://docs.rs/utoipa/
- geo: https://docs.rs/geo/
- tower-sessions: https://docs.rs/tower-sessions/
- PostgreSQL: https://www.postgresql.org/
- PostGIS: https://postgis.net/

Version policy:

- commit `Cargo.lock` cho application;
- pin Rust toolchain qua `rust-toolchain.toml`;
- ưu tiên stable crates/toolchain;
- review changelog trước major upgrade;
- dependency upgrade phải pass integration test với PostGIS thật;
- API/protocol compatibility không được phụ thuộc ngầm vào crate version.

---

# 96. Final recommended stack

```text
SERVER
──────────────────────────────────
Rust
Axum
Tokio
SQLx
Serde
Tower / tower-http
tracing
utoipa
uuid v7
bigint identity for high-volume GPS rows
time
geo
argon2
tower-sessions
subtle

DATABASE
──────────────────────────────────
PostgreSQL 18
PostGIS 3.6 stable
GiST spatial indexes
SQL migrations
PostgreSQL-backed processing queue initially

PROTOCOL
──────────────────────────────────
HTTPS
REST JSON
NDJSON / streaming device ingestion
OpenAPI
GeoJSON
SSE invalidation
MVT/vector tiles later

ARCHITECTURE
──────────────────────────────────
Modular monolith
Direct SQL with SQLx
Background workers in the same deployable initially
No Redis/Kafka/Kubernetes until justified by measured requirements
```
