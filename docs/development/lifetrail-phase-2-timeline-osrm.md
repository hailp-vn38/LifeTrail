# LifeTrail Phase 2 — Timeline, Trip/Stop Processing & OSRM Map Matching

## Status

Proposed implementation baseline for Phase 2.

The domain and publication decisions below were agreed on 2026-10-06 during
rounds 1–6 of `grill-with-docs`. Q1–Q31 are closed; consolidated decisions are in §62;
suggested
tables and API field names are illustrative until those decisions are resolved.
See [ADR-0001](../adr/0001-continuous-activity-and-daily-publication.md) and
[ADR-0002](../adr/0002-matcher-evidence-and-processing-revisions.md),
[ADR-0003](../adr/0003-generation-verified-multiday-publication.md) and
[ADR-0004](../adr/0004-route-parts-and-historical-progress-anchors.md) and
[ADR-0005](../adr/0005-immutable-range-revisions-and-daily-snapshots.md) and
[ADR-0006](../adr/0006-timezone-reprojection-with-stale-activity.md).

This document assumes the current Web baseline from branch `feat/option2-web-ui`, including:

- Option 2 application shell and navigation;
- Daily Map layout;
- Timeline component framework;
- Pinia-based Map/Timeline selection state;
- route playback;
- Course-Up / Heading-Up camera;
- look-ahead follow camera;
- Nginx `web` service serving the Vue SPA;
- Rust/Axum `server` running API-only on the internal Docker network.

Phase 2 does **not** rebuild those features from scratch. It extends them with real server-side processed data.

---

# 1. Outcome

Phase 2 turns LifeTrail from a raw GPS history viewer into a processed daily movement timeline.

The target pipeline is:

```text
Raw GPS
   |
   v
GPS Quality Processing
   |
   v
Stop / Move / Gap Segmentation
   |
   v
Movement Classification
   |
   +-- WALK
   +-- BIKE
   +-- CAR
   +-- UNKNOWN (raw fallback)
   |
   v
OSRM Map Matching
   |
   v
Matched Movement Geometry
   |
   v
Trip / Stop / Gap Derivation
   |
   v
Daily Timeline
   |
   v
Daily View API
   |
   v
MapLibre + MapTiler
   |
   +-- Timeline
   +-- Matched route
   +-- Route playback
   +-- Heading-Up camera
   +-- Overview animation
```

At the end of Phase 2, a user must be able to open one Owner-local day and understand:

- where the Device moved;
- when movement started and stopped;
- how long each stop lasted;
- which movement segments were likely walking, cycling or driving;
- the road-matched path for each movement segment;
- the daily Trip/Stop/Gap timeline, including cross-day event projections;
- the full processed route;
- the raw route when needed for debugging;
- synchronized interaction between Timeline and Map;
- route playback using historical GPS time.

---

# 2. Core Architecture Rules

## 2.1 Raw GPS remains the source of truth

`gps_points` remains immutable.

Phase 2 processing must never overwrite, delete or mutate Raw GPS in order to create a cleaner route.

```text
gps_points
   |
   +-- authoritative source
        |
        v
derived processing tables
```

Derived outputs can be reconstructed from their processing inputs. Phase 2
rebuilding creates new versions and retains persisted history under §46; it does
not automatically delete outputs or break snapshot references.

This rule allows:

- historical reprocessing;
- algorithm improvements;
- OSRM dataset upgrades;
- debugging GPS hardware;
- comparison between raw and matched routes;
- recovery from processing bugs.

## 2.2 Device collects, server understands

Firmware responsibilities remain:

```text
Collect
  |
Validate acquisition
  |
Store durably
  |
Sync
```

Phase 2 analytics belong on the server:

```text
Raw GPS
  |
Quality classification
  |
Stop/Move segmentation
  |
Movement classification
  |
Map matching
  |
Trip/Stop timeline
```

No trip detection, stop detection, map matching or routing logic should be moved into ESP32 firmware.

## 2.3 OSRM is a derived-data processor

OSRM is not a source of truth and is not a rendering library.

Responsibilities:

```text
MapTiler
  +-- basemap / map style / terrain / buildings

OSRM
  +-- map matching / routing / nearest-road operations

MapLibre
  +-- render / animation / camera / interaction

LifeTrail Server
  +-- processing orchestration / persistence / API
```

OSRM must not be called directly from the browser.

OSRM must not be on the critical request path of historical Daily View reads.

## 2.4 Daily View must remain available when OSRM is unavailable

OSRM failure must degrade route quality, not availability.

```text
MOVE
 |
 v
OSRM Match
 |
 +-- success
 |      |
 |      v
 |   matched route
 |
 +-- unavailable / failed / low confidence
        |
        v
     raw-derived route
```

A processed day may therefore contain route sources such as:

```text
osrm_match
raw_fallback
```

The Web must still render the day when OSRM is unavailable.

---

# 3. Phase 2 Scope

Phase 2 includes:

- GPS quality classification;
- impossible jump detection;
- temporal gap detection and first-class Gap events;
- Stop/Move segmentation;
- movement classification;
- OSRM infrastructure;
- OSRM Match integration;
- chunked map matching;
- matched route persistence;
- confidence/fallback policy;
- continuous multimode Trip derivation and daily projection;
- stop derivation;
- processed Daily View;
- Timeline API/read model;
- Timeline <-> Map interaction with real Trip/Stop events;
- matched-route playback;
- route source debugging;
- realistic GPS simulator based on routed road geometry;
- end-to-end processing tests.

Phase 2 explicitly excludes:

- Web login/authentication;
- public Internet deployment;
- TLS/ACME;
- multi-user authorization;
- photo ingestion;
- audio ingestion;
- media timeline;
- BLE/captive-portal provisioning;
- live tracking;
- video export;
- AI place recognition;
- semantic search;
- geofence automation;
- automatic POI naming;
- replacing MapTiler with OSRM Tile Service.

---

# 4. Publication and Processing State Model

A processed day is a published projection/snapshot for one Device and one
Owner-local calendar day. It is not the ownership boundary of activity events.
Trip, Stop and Gap belong to the Device's continuous activity history; a Trip
contains 1..N ordered Movement Segments.

Publication and background work are separate concepts:

```text
publication / data_freshness: current | stale | unavailable
processing:                  idle | queued | running | failed
```

A successful published snapshot stays available while a replacement is queued,
running or failed. `unavailable` means there is no successful processed snapshot;
Raw Daily View may then be shown. `current` means the publication covers the
relevant committed input and processing target; `stale` means refresh is needed.
Neither freshness nor processing state describes OSRM route quality: a current
snapshot may legitimately contain raw fallback.

Conceptual persistence responsibilities:

```text
processed_days
  device_id, local_date, timezone
  published_revision (nullable)
  data_freshness
  processing_state
  refresh failure details

processing_revisions
  captured Device input_generation
  processing_version, configuration identity
  matcher evidence references
  processing start/completion/failure metadata
  immutable successful derived output

Device activity history
  Trips -> ordered Movement Segments
  Stops
  Gaps
```

Exact table names remain illustrative. Every Device has a monotonic
`input_generation`; candidate processing captures this generation. Publication
scope covers the activity changes and all daily projections affected by them.
An activity event must not
belong exclusively to one `processed_day_id`. Snapshot reads must remain
consistent with the activity revision from which they were projected.

`processing_version` identifies LifeTrail processing logic. Material algorithm
changes increment it; configuration and matcher evidence are also part of the
provenance, not implied by that number alone.

Evidence quality is separate from background processing and freshness:

```text
evidence_state: sufficient | partial | insufficient
```

One GPS Record, or a day with all records excluded, can produce a successful
published snapshot with `evidence_state=insufficient`, no route parts and no
activity events. Do not invent Trip/Stop/Gap to populate the Timeline. Successful
publication leaves background work `idle`; examples calling a result `ready`
refer to successful output, not a replacement for the two-axis state model.

Trips and Stops at the data edges can have open boundaries:

```text
started_at: confirmed actual start, or null
ended_at: confirmed actual end, or null
observed_from_at, observed_until_at
start_boundary_state: open | confirmed
end_boundary_state: open | confirmed
observed_duration_s
full_duration_s: known only when both boundaries confirmed, otherwise null
```

Do not overload started_at/ended_at with observation bounds. When a boundary is
open, its actual start/end is null. observed_from_at/observed_until_at bound the
available evidence for that activity. observed_duration_s is their difference;
full_duration_s is null if either boundary is open. Daily projection clips only
the observed interval; it never extrapolates to now() or the end of a day.

Storage is range-based and immutable:

```text
ActivityRevision
  id, device_id
  processing_target_id, input_generation
  range_start, range_end
  supersedes_from, supersedes_until (or equivalent range)
  events, ordered Movement Segments, Route Parts, matcher evidence

DailySnapshot
  published_revision / snapshot identity
  device_id, local_date, timezone / projection generation
  referenced immutable activity versions
  complete summary, route_parts, evidence_holes and timeline
```

ActivityRevision contains only the continuous processing/replacement range,
not a copy of the Device's full history. Daily publication composes unchanged
activity versions outside that range with the new versions inside it. Replacement
cuts must be at stable processing boundaries; do not truncate an event where its
semantics still depend on evidence across the cut. Observation/Evidence Hole
edges preserve their open-boundary semantics rather than inventing confirmation.

`activity_revision_id` identifies the processing that created an event/segment/
part. `published_revision` identifies the daily snapshot currently activated.
These are different identities: a daily snapshot may reuse activity versions
from several processing revisions.

ActivityRevision is immutable after successful creation. A published DailySnapshot
is immutable. To change either, create a replacement and activate pointers;
never update existing event contents in place. ActivityManifest determines authoritative range membership; DailySnapshot pins
that immutable manifest version. The manifest and retention rules are below.

The three immutable layers have distinct responsibilities:

```text
ActivityRevision = what processing produced
ActivityManifest = which revision ranges form authoritative Device history
DailySnapshot    = the published projection of that history
```

An ActivityManifest version contains ordered non-overlapping UTC ranges using
`[from, until)` semantics, each referencing an immutable revision slice. Adjacent
ranges may touch exactly. The referenced slice must provide precisely the entry's
coverage; it may be a safe subset of an older revision's original processing
range. This is required by reuse after splice, not an assertion that every entry
has the full original revision range. A partition covers its declared processed
range, including explicit Gap/Evidence Hole coverage; it does not invent coverage
outside known history.

Manifest replacement is an explicit splice, never implicit revision precedence:

```text
M10: [A,D) -> R12
replace [B,C) with R18
M11: [A,B) -> R12; [B,C) -> R18; [C,D) -> R12
```

Later overlapping replacement splices those active entries again to produce a
new canonical manifest. Readers use the snapshot-pinned manifest, never SELECT
latest revision per timestamp or highest revision ID wins. The active manifest
pointer, activity activation and all affected daily snapshot pointers change in
one fenced publication transaction. Historical manifests/snapshots retain their
meaning when that active pointer changes.

Replacement endpoints must belong to a machine-checked semantic-safe boundary
registry. Eligible endpoints are Trip/Stop semantic boundaries, verified Evidence
Hole boundaries and verified observation start/end edges. A processing-window
edge qualifies only if independently semantic-safe. If an endpoint is unsafe,
expand and reprocess to a semantic-safe boundary; do not truncate an activity.

Chunk seam safety != activity boundary safety. A validated chunk seam MUST NOT,
by itself, authorize an ActivityRevision boundary. Chunk seams are technical
RoutePart assembly boundaries. A seam may coincide with a replacement endpoint
only when that timestamp is independently semantic-safe; the semantic boundary,
not the seam, authorizes replacement.

For a continuous Trip spanning [A,B), a requested replacement [x,y) inside it must
expand to its semantic-safe boundaries, even if x or y is a validated chunk seam.
A seam does not justify creating separate ActivityRevisions on its two sides.

Snapshot provenance includes:

```text
manifest_version
source_manifest_version
source_raw_generation / processed_through_generation
local_date, timezone_id, timezone_generation
reducer_version, projection_schema_version
projection_created_at
activity/source freshness and projection freshness
```

Names can be consolidated in the physical schema, but these meanings must be
preserved. Reused source revisions also retain their own input/target provenance;
a new daily snapshot ID does not imply all underlying activity was recomputed.

---

# 5. Dirty-Day Processing and Atomic Publication

The ingestion path commits Raw GPS and schedules processing without running
Phase 2 analytics synchronously.

```text
Batch committed
  -> determine affected activity range and daily projections
  -> mark affected publications stale; queue refresh
  -> worker builds a replacement revision
  -> validate and atomically publish successful output
```

Late GPS may change events across midnight. Dirty propagation must cover every
affected daily projection, including neighboring days when activity changes
extend beyond the calendar day containing the new GPS Record. Start at the dirty data range and expand left/right to stable segmentation
boundaries: confirmed Stop/Gap transitions with enough context on both sides
that activity outside the range is unchanged by recomputation. A fixed ±hours
window is not a semantic rule. At data edges, preserve open activity boundaries.
New evidence can extend or close them and propagate dirtiness into further days.

```text
published R7 + new input -> serve R7, stale, queued/running
R8 succeeds             -> atomic publication R7 -> R8
R8 fails                -> serve R7, stale, failed
```

Daily View reads persisted publication data and never call OSRM. A reader must
not see a partially replaced timeline, route or summary. Concurrent ingestion
must not make an older captured input appear current; publication must verify the captured generation inside the activation
transaction. If it differs from the current generation, discard candidate
publication, keep prior snapshots, and mark dirty/queued again.

Build candidate rows before publication; never hold a DB transaction open while
calling OSRM. The short activation transaction must:

1. Verify current Device `input_generation` equals the captured generation.
2. Activate the candidate activity revision and every affected daily projection.
3. Commit all pointer swaps together, or none.

Two daily projections changed by the same crossing activity cannot be published
separately as new/old revisions. Unaffected days do not need replacement merely
to share a revision number. Timezone projection identity must also be checked so
an old-timezone candidate cannot activate as a new-timezone projection.
input_generation advances once within a successfully committed new valid Batch
transaction, including late data, overlapping timestamps and poor/unusable
quality. Replay, conflict, validation/authentication failure and rollback never
advance it. Configuration, dataset, timezone and reprocessing requests do not
change Raw GPS and therefore do not advance input_generation.

A candidate captures a separate ProcessingTarget alongside input_generation:
processing version, configuration version/hash, matcher engine/profile/dataset
identity, and timezone/projection generation for its projections. Publication
must verify the current target as well as current Raw input. A target change
without a new Batch must reject the stale candidate. Exact target/storage names
are implementation details; worker claiming and range revision storage are specified below.

PostgreSQL is the durable queue; Phase 2 needs no Redis/Kafka. One in-process
server worker is sufficient for acceptance, while persistence remains per Device
rather than imposing a one-Device schema constraint.

Suggested Device control row responsibilities:

```text
device_id
input_generation
processing_target_generation, timezone_generation
desired_generation, published_generation
next_fencing_token / active fencing authority
dirty_from, dirty_until
```

A new Batch transaction locks that row with FOR UPDATE, commits Batch/Raw GPS,
advances input_generation once, merges dirty bounds and enqueues/coalesces work.
Target updates and publication use the same durable control-row serialization.
Validation may occur before the transaction; failures never advance generation.

Suggested durable job fields:

```text
id, device_id, state
lease_until, fencing_token, attempt
requested_from, requested_until
```

Claim work in a short transaction with FOR UPDATE SKIP LOCKED, assign a fresh
fencing token/lease, and capture the work target. Read generation/target/Raw input
from a consistent database snapshot; end DB transactions before OSRM processing.
Nearby/overlapping dirty requests coalesce per Device before expansion to stable
boundaries. New dirty input while work runs must not be lost when that job ends.

Publication additionally verifies the candidate's fencing token equals current
active authority. Lease expiry alone does not make an old worker safe: after
reclaim, a worker with an older token must be rejected even if its input/target
still match. Lease-expired jobs can be reclaimed with a newer token. Publication
is idempotent: repeated candidate execution cannot create two active publications.
Temporary duplicate candidate rows are acceptable; duplicate activation is not.
No control-row lock, job-claim transaction or input-read transaction is held
across OSRM calls.

---

# 6. GPS Quality Classification

Phase 1 intentionally accepts valid Raw GPS even when quality is poor.

Phase 2 classifies whether a point should participate in derived movement geometry.

Possible point classifications:

```text
usable
low_quality
temporal_duplicate
impossible_jump
gap_boundary
excluded
```

Inputs may include:

- `fix_quality`;
- `satellites`;
- `hdop`;
- speed;
- course;
- distance from previous point;
- elapsed time;
- implied speed;
- neighboring trajectory shape.

No Phase 2 classification may delete the original row from `gps_points`.

---

# 7. Impossible Jump Detection

A point must be eligible for exclusion from derived geometry when it implies physically implausible movement.

Example:

```text
A -- B -- C -------------- X -- D

C -> X = 3000 m
dt     = 2 s
```

This should not produce a road-matched trip through `X`.

Detection should use configurable thresholds, not protocol invariants.

Suggested configuration surface:

```text
max_implied_speed_mps
max_jump_distance_m
max_gap_seconds
```

Thresholds must be tunable using realistic fixtures.

---

# 8. Gap Detection and Timeline Semantics

A GPS Gap is a first-class activity event representing an interval with missing
GPS evidence. It terminates a Trip. It is not evidence of a Stop or movement.
Examples include a powered-off Device, unavailable GNSS, paused recording or a
long tunnel outage. Upload delay alone is not a GPS Gap.

```text
08:00–08:10 TRIP
08:10–08:30 GAP · No GPS data
08:30–08:50 TRIP
```

Gap detection uses a configurable temporal threshold on the Raw GPS observation
sequence, before quality filtering can remove points. A Gap requires an actual
absence of Raw GPS Records between two known observed boundaries. No Gap is
created before the first Raw GPS Record or after the last.

No OSRM request may fill the missing interval, no straight connector is included
in distance, and no playback position is interpolated through it.

A Gap may cross midnight and is projected into Daily Views like other activity
events. Its duration is separate from Trip and Stop duration. Its interval is bounded
by the two observations enclosing the temporal absence;
periods outside available observation coverage are not Gaps.

The processed daily route contains disconnected LineStrings, represented as a
MultiLineString or an equivalent segment read model. It must never introduce a
connector across a Gap.

An Evidence Hole is different: Raw observations exist, but the processor cannot
derive reliable activity or geometry from them. Dropping low-quality records
must not create a GPS Gap. Phase 2 need not add an Evidence Hole Timeline kind;
its coverage is disclosed in the separate evidence_holes structure (§28). A matcher
split or failure also does not prove a GPS Gap.

An Evidence Hole may occur inside the Device's history. Activity on its left
has an open end, activity on its right an open start. Do not infer a Stop/Gap
inside the hole or assert that both sides are one continuous Trip. OPEN
boundaries are not restricted to the first/last records in the dataset.

---

# 9. Stop / Move Segmentation

Stop detection happens **before** OSRM map matching. First establish Stop/Gap
boundaries and continuous movement ranges; split those ranges into relatively
mode-homogeneous Movement Segments and group them into Trips.

Do not send one whole day directly to OSRM Match.

Recommended model:

```text
07:42 MOVE
08:01 STOP
08:34 MOVE
08:57 STOP
...
```

A stop should not be detected solely from `speed == 0`.

Inputs may include:

- dwell duration;
- spatial radius;
- median/centroid position;
- speed;
- HDOP/quality;
- movement before and after the cluster.

Initial tuning defaults may begin near:

```text
STOP_RADIUS_M = 30
STOP_MIN_DURATION_S = 180
```

These values are configuration defaults, not domain invariants.

---

# 10. Stop Data Model

A Stop is a spatial dwell that meets the configured detection criteria. It
terminates a Trip; a short pause below the minimum dwell duration does not
itself terminate a Trip.

Suggested activity-level fields:

```text
stops
  id
  device_id
  activity revision reference
  started_at, ended_at (nullable when boundary open)
  observed_from_at, observed_until_at
  start_boundary_state, end_boundary_state
  observed_duration_s, full_duration_s (nullable)
  center_geometry, radius_m
  source_point_count, quality_score
```

The server derives stop geometry. The Device does not supply authoritative stop
geometry. A Stop is not owned by a calendar day or `processed_day_id`.

A Stop from 23:50 to 00:20 is one 30-minute event. Daily projections show 10
minutes on the first day and 20 on the next, preserving the same event identity
within the corresponding published activity history. Event identity is stable across daily projections within a revision only;
reprocessing may change IDs without merge/split reconciliation (§27).

---

# 11. Movement Segments

A Movement Segment is a relatively homogeneous transport-mode portion of a Trip.
A Trip contains 1..N ordered Movement Segments. A transport-mode change creates
a segment boundary without ending the Trip. Stop and Gap boundaries end Trips.

Suggested fields:

```text
movement_segments
  id
  trip_id
  sequence
  observed_from_at, observed_until_at
  first_gps_point_id, last_gps_point_id
  raw_point_count, raw_distance_m, duration_s
  transport_mode, classification_confidence
```

Allowed initial modes: `unknown`, `walk`, `bike`, `car`. Each segment owns its
selected route geometry and matching provenance. Mode transitions use time windows, confidence persistence and hysteresis (§12).
Insufficient evidence does not require a fabricated segment or geometry (§4).

---

# 12. Movement Classification

Initial classification may use deterministic heuristics.

Inputs can include:

- average speed;
- median speed;
- maximum sustained speed;
- acceleration distribution;
- stop frequency;
- route length;
- duration.

Example directional heuristics:

```text
low sustained speed
   -> WALK candidate

moderate sustained speed
   -> BIKE candidate

higher sustained speed
   -> CAR candidate
```

Exact thresholds must be validated against real or realistic datasets.

Phase 2 does not require machine learning.

Every classification should preserve a confidence value.

Example:

```text
transport_mode = car
classification_confidence = 0.84
```

Low-confidence classification uses `unknown` for profile selection. A confident
`walk` selects foot, `bike` selects bike, and `car` selects car. The profile-selection
confidence threshold is configurable. `unknown`, including a below-threshold
candidate mode, uses `raw_fallback`; do not try all profiles and choose the most
plausible-looking route. Successful matching does not prove the transport mode.

Mode evidence is computed over time windows and fed to a hysteresis state
machine, not used to cut a segment at each GPS Record:

```text
candidate confidence >= enter threshold
AND candidate maintained >= minimum duration
  -> accept mode transition

exit threshold < enter threshold
  -> tolerate brief evidence fluctuations
```

Configuration includes:

```text
MODE_WINDOW_SECONDS
MODE_CHANGE_MIN_DURATION_SECONDS
MODE_ENTER_CONFIDENCE
MODE_EXIT_CONFIDENCE
MODE_UNKNOWN_GRACE_SECONDS
```

A short UNKNOWN interval may collapse into its surrounding mode only when both
sides supply compatible evidence. A sustained UNKNOWN interval remains its own
segment. Short pauses neither create a Stop below dwell criteria nor themselves
change transport mode. Exact transition timestamp attribution and threshold
values are tuning/algorithm decisions; pauses still belonging to a Trip count
in trip_duration_s (§28).

---

# 13. OSRM Responsibilities

Phase 2 uses OSRM for three internal use cases.

## 13.1 Match Service

Primary production use.

```text
GPS trace
   |
   v
OSRM Match
   |
   v
road-matched geometry
```

Used for real historical movement segments.

## 13.2 Route Service

Primary simulator use.

```text
Home
 |
Cafe
 |
Office
 |
OSRM Route
 |
road geometry
```

Used to generate realistic synthetic GPS traces.

It may also support future planned-route features, but that is not Phase 2 product scope.

## 13.3 Nearest Service

Internal utility only.

Possible uses:

- debugging;
- single-point road snapping;
- future processing diagnostics.

Do not expose it as a public Web feature in Phase 2.

---

# 14. OSRM Deployment Topology

Current runtime:

```text
web
server
postgres
```

Phase 2 target:

```text
web
server
postgres

osrm-car
osrm-bike
osrm-foot
```

High-level Docker topology:

```text
Browser
   |
   v
web : Nginx
   |
   +-- Vue SPA
   +-- /api/* proxy
          |
          v
       server : Axum
          |
          +-- postgres
          +-- osrm-car
          +-- osrm-bike
          +-- osrm-foot
```

Suggested environment variables:

```text
LT_OSRM_CAR_URL=http://osrm-car:5000
LT_OSRM_BIKE_URL=http://osrm-bike:5000
LT_OSRM_FOOT_URL=http://osrm-foot:5000
```

OSRM services should only be available on the internal Compose network.

---

# 15. OSRM Dataset Preparation

Use self-hosted OSM extracts.

The processing chain should use MLD:

```text
.osm.pbf
   |
   v
osrm-extract
   |
   v
osrm-partition
   |
   v
osrm-customize
   |
   v
osrm-routed --algorithm mld
```

Each profile requires its own processed dataset.

Example layout:

```text
data/osrm/
+-- car/
|   +-- test-region.osrm*
+-- bike/
|   +-- test-region.osrm*
+-- foot/
    +-- test-region.osrm*
```

Use a bounded test-region extract covering all acceptance waypoints from the
start. Vietnam-wide data is a later deployment/scaling option, not the Phase 2
acceptance baseline.

---

# 16. OSRM Dataset Versioning

Persist which routing dataset produced a matched route.

Example:

```text
engine = osrm
engine_version = 6.x
profile = car
dataset_version = vietnam-2026-10-01
processing_version = 1
```

After an OSM update:

```text
vietnam-2026-10-01
        |
        v
vietnam-2026-11-01
```

LifeTrail must be able to determine which historical rows were generated with which dataset.

Reprocessing old days is an explicit operation, not an accidental side effect.

---

# 17. Match Input Construction

A movement segment may contain far more raw GPS points than should be sent in one OSRM Match request.

Example:

```text
20 minutes x 1 sample/second
= 1200 points
```

Therefore the server must have a Match input builder.

Responsibilities:

- select usable points;
- remove invalid same-second timestamp collisions if required;
- reduce redundant points;
- preserve important turns;
- preserve temporal ordering;
- split into bounded chunks;
- create overlap between chunks.

---

# 18. Timestamp Conversion

LifeTrail Raw GPS uses:

```text
ts_ms
```

Matcher input may need second-resolution timestamps.

Conversion:

```text
timestamp_s = floor(ts_ms / 1000)
```

The matcher input builder must guarantee strictly increasing timestamps.

When multiple Raw GPS records map to the same second, the builder must deterministically select or reduce points before creating the Match request.

The Raw GPS rows remain unchanged.

Same-second selection has a deterministic total order:

1. Usable status, then explicitly defined quality rank
   (conceptually usable > low_quality > excluded).
2. Lower HDOP, with known values ahead of missing values.
3. More satellites, with known values ahead of missing values.
4. Earlier original ts_ms.
5. GPS Record ID as the final tie-break.

Define quality ranking explicitly; do not rely on nullable SQL comparison or
numeric fix-quality values accidentally expressing the intended order. Preserve
mapping in matcher evidence:

```text
matcher_point_index, source_gps_record_id, source_ts_ms
matcher_timestamp_s, selection_reason
```

Reasons may include same-second winner, turn-preservation, segment-boundary and
normal-reduction. The reducer selects/drops/retains mapping; it never adds seconds
or invents timestamps. Preserve turns using neighboring seconds when possible.
If second resolution cannot represent a detail, record the reduction loss.

---

# 19. Matching Radius

The current `gps/1` schema contains:

```text
fix_quality
satellites
hdop
```

It does not contain a direct horizontal `accuracy_m`.

Therefore Phase 2 must not pretend an exact receiver accuracy exists.

If HDOP is present, an estimated matching radius may be derived:

```text
estimated_match_radius_m
    = hdop x configurable_error_factor
```

This derived value must be treated as a processing estimate.

When quality metadata is insufficient, use a configurable fallback radius.

No derived radius is written back into the Raw GPS record.

---

# 20. Match Chunking

The Match client must support chunking.

Recommended initial defaults:

```text
MATCH_CHUNK_POINTS = 80
MATCH_OVERLAP_POINTS = 5
```

Concept:

```text
1 --------------------- 80
                  76 --------------------- 155
                                     151 ------------- ...
```

Overlap exists to help preserve continuity across Match requests.

Chunk size must remain configurable.

The implementation must not assume one movement segment equals one OSRM request.

Merge chunks using shared source observations, historical timestamps, matched
progress and tracepoint locations, not a fixed number of output vertices.
A safe seam requires a shared source observation, compatible temporal order,
geometric separation within configurable seam tolerance and compatible progress
direction. A bearing tolerance may also be used.

Select among valid seams with this deterministic total order: highest shared
observation quality, smallest geometry separation, smallest bearing discontinuity
when available, closest to the overlap midpoint, then source GPS Record ID.
Missing optional metrics rank after available values. Exact thresholds are
configuration/tuning choices.

A validated seam authorizes RoutePart assembly only. It does not authorize an
ActivityRevision replacement endpoint unless independently semantic-safe (§4).

Record chunk IDs, shared source IDs, selected seam source record, progress in both
chunks, geometry separation and decision, or fallback reason. Never connect
divergent matched roads to conceal an unsafe seam. If overlap temporal boundaries
are safe, a raw fallback overlap may join accepted matched parts as a published
part. Otherwise fall back the entire affected Movement Segment under §37.

---

# 21. Match Input Reduction

Raw 1 Hz GPS may contain many nearly redundant points.

Before chunking, the matcher input may be reduced while preserving:

- route shape;
- major heading changes;
- temporal ordering;
- quality changes;
- segment boundaries.

Do not apply geometry simplification that ignores timestamps.

Input reduction is only for matching efficiency.

It must never replace the Raw GPS source.

---

# 22. Matched Route Data Model

Suggested table:

```text
matched_routes

id
movement_segment_id

engine
engine_version
dataset_version
profile

geometry

distance_m
duration_s
confidence

input_point_count
matched_point_count
outlier_point_count

route_source
status

created_at
```

Recommended statuses:

```text
matched
partial
low_confidence
no_match
failed
```

Recommended `route_source`:

```text
osrm_match
raw_fallback
```

Persist ordered Route Parts separately from the logical Movement Segment:

```text
Movement Segment -> 0..N drawable Route Parts
Route Part -> one contiguous LineString + progress anchors
```

A segment can have several parts after partial/split matching. Never concatenate
those parts with a guessed connector. An insufficient-evidence snapshot can
have no drawable parts. The canonical read model is `route_parts`; a daily
MultiLineString is only a derived rendering convenience (§37).
Matcher attempt evidence is retained alongside route derivation (§45).

---

# 23. Confidence Policy

OSRM confidence must be treated as processing input.

Do not encode confidence thresholds as protocol invariants.

Possible initial tuning:

```text
HIGH_CONFIDENCE = 0.80
MEDIUM_CONFIDENCE = 0.50
```

Policy example:

```text
confidence >= HIGH
    -> matched

MEDIUM <= confidence < HIGH
    -> matched but uncertain

confidence < MEDIUM
    -> raw fallback
```

Thresholds must remain configurable and must be tuned against representative data.

---

# 24. Outliers and Partial Matches

A Match result may contain unmatched tracepoints.

The server must preserve metrics such as:

```text
input_point_count
matched_point_count
outlier_point_count
```

A movement segment with partial Match output must not automatically fail the entire processed day.

Possible behavior:

```text
good partial match
   -> persist as partial

poor partial match
   -> raw fallback
```

The policy must be deterministic.

---

# 25. Raw Fallback Route

Every Movement Segment must have a safe non-OSRM fallback policy. Where at
least two usable points form a valid line, it can produce raw-derived geometry.
Insufficient usable evidence may yield no drawable route parts (§4); never
invent a second coordinate merely to satisfy a LineString schema.

Fallback route:

```text
usable raw GPS points
      |
      v
server-derived LineString
```

This geometry may be less visually accurate but guarantees availability.

The API must expose the route source so the Web can distinguish:

```text
Road matched
Raw GPS
```

---

# 26. Trip Derivation

A Trip is a maximal continuous chain of movement, terminated by a valid Stop
or a GPS Gap. It is not one Movement Segment and is not the Raw GPS source.

```text
07:50 WALK to parking -> 07:55 CAR to office
no valid Stop or Gap between them

Trip #1
  Segment #1 WALK
  Segment #2 CAR
```

A short pause below `STOP_MIN_DURATION_S` does not itself split a Trip. Persist
`movement_segments.trip_id` with deterministic ordering, or an equivalent ordered
relationship; do not persist a single `trip.movement_segment_id` assumption.

Suggested Trip fields:

```text
id, device_id, activity revision reference
started_at, ended_at (nullable when corresponding boundary open)
observed_from_at, observed_until_at
start_boundary_state, end_boundary_state
observed_duration_s, full_duration_s (nullable)
ordered Movement Segments
distance_m
```

Trip distance is the sum of its published Route Part lengths, including safe
matched/raw hybrid parts. Rejected geometry and invented connectors contribute
no distance. Modes, route source and confidence are segment-level facts;
a Trip can contain several modes and both matched and fallback routes.

Trips exist across midnight. Daily View clips their visible interval and daily
metrics without splitting the authoritative event into day-owned Trips. Time-based distance clipping uses route-part progress anchors (§37).

---

# 27. Timeline Model and Daily Projection

The Device's continuous activity history contains Trip, Stop and Gap events.
Daily View projects that history onto the Owner-local calendar interval
`[local midnight, next local midnight)`, using the Owner's IANA timezone.

```text
start
trip
stop
trip
gap
trip
...
end
```

Start/end are presentation markers, not substitutes for activity events.
Future photo/audio event kinds remain outside Phase 2.

Every projected activity event exposes both its full interval and visible
intersection with the selected day, with semantics equivalent to:

```text
started_at, ended_at (actual boundaries, nullable if open)
observed_from_at, observed_until_at
start_boundary_state, end_boundary_state
observed_duration_s, full_duration_s (nullable)
visible_started_at, visible_ended_at (daily clipped observed coverage)
continues_before, continues_after
```

Underlying timestamps stay UTC. The names above may change in OpenAPI, but the
actual/observed/visible distinctions are required. continues_before means the
observed interval starts before this day; continues_after means it extends past
the day's end. Neither flag infers unobserved actual continuation: boundary states
carry that uncertainty. Daily summary durations count only
the intersection with the day; a crossing event keeps one identity rather than
becoming two domain events. Timeline order is chronological and deterministic.

Processing must read context on both sides of day boundaries or process a
continuous range before projecting days. A fixed context window is not assumed
sufficient for arbitrarily long activities. Range closure follows stable segmentation boundaries (§5). The current Owner
IANA timezone defines the projection; UTC activity has no calendar-day ownership.

Projection identity must include timezone, using a key equivalent to
`(device_id, local_date, timezone, manifest_version, projection_schema_version)`
with timezone generation guarding activation. The manifest supplies all referenced
activity versions; one activity revision alone need not cover the whole day.
Owner timezone changes rebuild only projection, summary, clipping, visible
intervals and route parts. They do not rerun GPS quality, segmentation,
classification or OSRM.

Never relabel an old-timezone snapshot under the new timezone. If no projection
exists for the current timezone, serve Raw Daily View with projection
queued/running status. Lazy rebuilding requested days may be used rather than
reprojecting the entire history immediately; no OSRM work is put on read paths.

IDs are stable only within an activity revision, including across its daily
projections. Phase 2 does not reconcile identities after reprocessing merges or
splits events. IDs may include a revision namespace, e.g. trip:R7:T1. Activity revision identity and daily publication identity are distinct (§4).

On published revision change, Web clears selectedEventId, pauses playback,
resets playback position and replaces route/timeline/summary atomically. Do not
retain references to events from the previous publication.

Timezone-only reprojection may use the currently published activity even when
new Raw GPS awaits processing. For example, published activity based on G142
can be projected under the new timezone generation T6 while current Raw input
is G143. The snapshot is a valid new-timezone projection with stale source; it
must not claim activity processed through G143.

Distinguish activity freshness from projection freshness. A projection may be
current for its timezone and source manifest while that source is stale relative
to Raw input/processing target. Existing current/stale/unavailable API freshness
summarizes these axes; source metadata retains the reason (e.g. STALE_SOURCE).
If no usable current-timezone projection exists yet, Raw Daily View remains the
fallback described above.

Projection-only activation verifies:

```text
active_manifest.version == expected_manifest_version
AND owner.timezone_generation == expected_timezone_generation
AND current projection-job fencing authority remains valid
```

It must not modify activity publication state. It does not require the older
published source to pretend it covers the latest Raw generation. If activity
publishes a new manifest or timezone changes during reprojection, reject the
candidate projection and queue the current manifest/timezone combination.
New activity publication later replaces affected projections atomically under
the activity generation/target/fencing protocol.

---

# 28. Daily View API

The canonical endpoint remains:

```http
GET /api/v1/devices/:deviceId/days/:date
```

Evolve its OpenAPI response and generate Web types. The following is a field
sketch, not a complete response or a valid nonempty GeoJSON example:

```text
device_id, date, timezone
processing:
  state: idle | queued | running | failed
  data_freshness: current | stale | unavailable
  published_revision: revision identifier or null
  refresh failure details when applicable
published processing provenance
summary:
  point_count, usable_point_count, excluded_point_count
  distance_m
  trip_duration_s, stop_duration_s, gap_duration_s
  trip_count, stop_count, gap_count
  first_fix_at, last_fix_at
evidence_state: sufficient | partial | insufficient
route_parts:
  ordered contiguous LineStrings, vertex_distance_m and historical progress anchors
evidence_holes: projected unresolved Raw evidence intervals and reasons
optional overview route:
  derived MultiLineString assembled from route_parts
timeline:
  projected Trip / Stop / Gap events
```

A published revision remains the source of route, timeline and summary while
refresh runs or fails. Freshness describes whether that revision covers current
input, not whether its routes were successfully matched. A first-time day with
no published processed snapshot may expose Raw Daily View and processing status.

Daily metrics use only overlap with the selected calendar day and never include
Gap connectors. Insufficient evidence can yield empty route_parts/timeline without failure.
Progress anchors and route-part geometry are specified in §37. Summary and
open-boundary semantics below are settled; evidence_holes encodes unresolved coverage.

A lightweight status endpoint is part of the canonical OpenAPI contract:

```http
GET /api/v1/devices/:deviceId/days/:date/status
```

It returns background state, freshness, `published_revision`, current
`input_generation` and current timezone/projection identity, without route
geometry. Status reflects the requested projection under the current Owner
timezone. The full Daily View includes that identity and a complete published
snapshot so Web can replace all displayed data together.

Daily summary semantics:

- point_count counts all Raw GPS Records in the Owner-local day, not only
  usable/fix-quality-filtered records. first_fix_at/last_fix_at follow the first
  and last Raw observation timestamps in that day, or null when none; these
  fields do not assert Phase 2 usable quality.
- usable_point_count counts records usable for activity/geometry derivation.
  excluded_point_count may be exposed; usable + excluded <= point_count because
  intermediate classifications need not exhaust the set.
- trip_duration_s includes short pauses belonging to Trips and does not claim
  physical movement every second. stop_duration_s and gap_duration_s are separate.
- Durations sum only each event's observed intersection with the day. Event
  counts include events with strictly positive observed overlap; crossing events
  count once in each daily projection, not as global distinct counts.
- Distance sums published Route Parts clipped to that day using progress anchors.
  It excludes rejected geometry and connectors without evidence.

Evidence state reflects activity evidence coverage, independently of matcher
success or route source:

```text
sufficient: all considered observed coverage has reliable activity assignment,
            apart from explicitly modeled GPS Gaps
partial:    at least one reliable activity, with unresolved evidence coverage
insufficient: no reliable activity can be derived
```

A day with Trip/Stop/Gap/Trip may be sufficient: the Gap explicitly states the
known absence. An Evidence Hole can make a day partial. A successful raw fallback
does not by itself make activity evidence partial. Coverage metadata explains unresolved intervals without inventing a Timeline
event; see evidence_holes below.

Evidence Holes are first-class read-model metadata, not Timeline events:

```text
evidence_holes[]:
  from, until (daily projected interval)
  reason
  raw_point_count
```

Initial reasons include insufficient_quality, ambiguous_activity,
insufficient_geometry and unsupported_classification. Do not use matcher_failed
as the default reason if Raw GPS supports valid fallback activity/geometry.
The Web must distinguish “No GPS data” for Gap from “Insufficient data to determine
activity” for Evidence Hole. Playback does not infer movement across either.
Their intervals must be represented consistently with the observed coverage
and activity assignments; neither expands to unobserved day edges.

---

# 29. Trip Timeline Event

A projected Trip includes:

```text
id, type=trip
started_at, ended_at                 # confirmed boundaries or null
observed_from_at, observed_until_at
start_boundary_state, end_boundary_state
observed_duration_s, full_duration_s # full duration nullable
visible_started_at, visible_ended_at # observed intersection with the day
continues_before, continues_after
full event metrics and daily visible metrics (distinct)
ordered segments:
  id, sequence
  transport_mode, classification_confidence
  segment interval and daily intersection
  route source, geometry, match confidence, provenance
```

Do not imply a single transport mode, source or confidence for a multimode Trip.
Each visible segment contributes its own disconnected geometry as needed.
Selection highlights the Trip's visible geometry in that Daily View. Field names and metric encoding must be finalized in OpenAPI; route parts and
playback anchors follow §37.

---

# 30. Stop and Gap Timeline Events

A projected Stop includes:

```text
id, type=stop
started_at, ended_at                 # confirmed boundaries or null
observed_from_at, observed_until_at
start_boundary_state, end_boundary_state
observed_duration_s, full_duration_s # full duration nullable
visible_started_at, visible_ended_at # daily observed intersection
continues_before, continues_after
visible_duration_s
server-derived center Point and radius_m
```

A projected Gap includes the same full/visible interval distinction and
`type=gap`. It means missing GPS evidence, not stationary time or movement.
Its visible duration contributes to `gap_duration_s` only. It has no inferred
route through the missing interval. Last/next observed positions may support UI
and playback, but must not become an invented connecting geometry.

---

# 31. API Geometry Strategy

Avoid returning unnecessary duplicate geometry when response size becomes large.

Initial Phase 2 may keep geometry in Daily View for simplicity.

If payload size becomes a problem, a later compatible optimization may split geometry retrieval from event metadata.

Do not prematurely introduce this split before measurements show it is needed.

---

# 32. Option 2 Web Baseline

The current Web already provides:

- `AppLayout`;
- sidebar;
- topbar;
- Daily Map page;
- Timeline panel;
- Timeline item model;
- Map selection store;
- playback store;
- route playback;
- Course-Up camera;
- heading puck;
- final route overview.

Phase 2 frontend work must extend these components rather than replace them.

---

# 33. Timeline <-> Map Interaction

Current shared state pattern remains valid:

```text
Timeline item
     |
     v
selectedEventId
     |
     v
Map
```

and:

```text
Map feature
     |
     v
selectedEventId
     |
     v
Timeline
```

Phase 2 extends supported event IDs from:

```text
start
end
```

to:

```text
start
trip:<id>
stop:<id>
gap:<id>
end
```

---

# 34. Trip Selection Behavior

Selecting a Trip should:

```text
select timeline item
      |
      v
highlight Trip route
      |
      v
fitBounds(trip geometry)
```

The user must be able to distinguish the selected Trip from the complete-day route.

---

# 35. Stop Selection Behavior

Selecting a Stop should:

```text
select Stop
    |
    v
highlight stop marker/radius
    |
    v
flyTo(stop center)
```

The Timeline should display at minimum:

- confirmed arrival/departure time, or an explicit unknown boundary when OPEN;
- observed time bounds and duration;
- full duration only when both actual boundaries are confirmed.

Automatic POI naming is not required in Phase 2.

---

# 36. Playback Geometry vs Playback Time

This is a required architectural distinction.

Playback geometry:

```text
selected geometry per Movement Segment:
  accepted OSRM match or raw fallback
  disconnected at Gaps
```

Playback clock:

```text
historical GPS timestamps
```

Do not use OSRM estimated route duration as the replay clock.

LifeTrail is replaying history, not simulating navigation.

---

# 37. Matched Route Time Mapping

Because matched geometry contains vertices that do not necessarily correspond one-to-one with Raw GPS points, Phase 2 must define a deterministic progress mapping.

Progress mapping:

```text
Raw GPS timestamp
      |
      v
matched tracepoint / segment progress
      |
      v
interpolated position on matched geometry
```

Requirements:

- monotonic progress;
- no backwards playback;
- stable seek;
- historical timing preserved;
- segment boundary behavior deterministic.

The canonical playback contract is ordered `route_parts`. Each Movement Segment
can have multiple contiguous drawable parts; one part per segment is not required.
Each part exposes:

```text
id, movement_segment_id, source
observed_from_at, observed_until_at
geometry: contiguous GeoJSON LineString
vertex_distance_m: server-computed cumulative progress per coordinate
distance_m: final cumulative distance
anchors:
  recorded_at
  distance_m along this part
  optional source_point_id for debugging
```

Anchor timestamps are strictly increasing; distances are non-decreasing and
within `[0, part.length]`. Include start/end anchors. Playback interpolates
distance between historical-time anchors, then evaluates that distance along
the part geometry. There is no timestamp-per-OSRM-vertex requirement.

Daily clipping uses the same anchors. A part crossing midnight is clipped at the
corresponding anchored distance, with a synthetic anchor at the exact day
boundary. Distances in a clipped part are relative to that clipped geometry's
start. Synthetic boundary anchors are projection data, not new GPS Records.
Daily MultiLineString is a derived overview convenience, never the playback
contract. No position interpolation connects disconnected parts, including
those split by matcher output. A matched part is publishable only with valid geometry, known historical
coverage boundaries and valid monotonic anchors. Reject a visually plausible
matched geometry if it cannot meet these requirements.

Hybrid matched/raw fallback parts are allowed within one Movement Segment when
the processor can assign safe temporal boundaries and each part has valid
coverage/anchors. If that mapping cannot be trusted, use raw fallback for the
entire Movement Segment rather than guessing hybrid joins. Matcher failure does
not create a GPS Gap. Distances sum published part lengths only; a connector must
itself be an evidence-supported Route Part to contribute distance.

At a Gap, hold the last observed position without animating across the missing
interval. Historical-time mode may let the clock pass through the Gap, then
jump to the next observed position at its timestamp. Seeking into a Gap must
retain the missing-data indication. Smart playback may compress a Gap but must
still disclose it; Smart mode is optional.

Server owns the distance/progress metric:

```text
len(geometry.coordinates) == len(vertex_distance_m)
vertex_distance_m[0] == 0
vertex_distance_m is non-decreasing
part.distance_m == vertex_distance_m.last (within numerical tolerance)
```

Web uses these distances and anchors for playback, drawing progress, seek and
tooltips; it does not independently compute Haversine lengths to redefine the
metric. Locating/interpolating a coordinate at supplied progress is presentation
math under the server-owned measure.

For clipping, derive Dstart/Dend from the same temporal anchors and compute daily
distance as Dend - Dstart. Synthetic midnight anchors and clipped geometry use
that original progress measure, rebased to zero for the visible part. Adjacent
daily projections must conserve the original part distance within numerical
tolerance; no additional straight connector or duplicated boundary contributes.
Evidence Hole playback explains uncertain activity, separately from GPS Gap.

---

# 38. Playback Behavior

Existing controls remain:

```text
Play
Pause
Restart
Seek

1x
2x
5x
10x
```

Phase 2 may add:

```text
Auto
30 sec
60 sec
90 sec
```

Playback speed may be computed as:

```text
playback_rate =
    historical_duration / target_playback_duration
```

Example:

```text
2 hours / 60 seconds = 120x
```

This is presentation logic.

It must not modify persisted timeline timestamps.

---

# 39. Smart Stop Compression

Optional Phase 2 enhancement:

```text
Moving
   -> normal compressed playback

Long Stop
   -> fast-forward
```

A user-visible mode may later expose:

```text
Real time
Compressed
Smart
```

The minimum Phase 2 requirement is matched-route playback with historical time.

---

# 40. Course-Up Camera

Existing behavior remains the preferred navigation-style playback.

Current baseline:

```text
zoom ~= 16.5
pitch ~= 45 deg
look-ahead ~= 30 m
bearing = route heading
```

The moving position stays below viewport center so more road ahead is visible.

```text
          route ahead
              ^
              |
        camera target

              |
              *
         current position
```

At playback completion:

```text
bearing -> 0
pitch   -> 0
fit full route
```

---

# 41. Raw vs Matched Debug Display

Phase 2 should support a developer/debug route display mode.

Example:

```text
Route display

(*) Road matched
( ) Raw GPS
```

This is valuable for:

- GPS hardware debugging;
- processing validation;
- map-matching tuning;
- confidence threshold tuning.

This control does not need to be prominent in the normal user UI.

---

# 42. Simulator Architecture

The Phase 2 simulator should no longer require manually inventing hundreds of road coordinates.

Scenario:

```text
Home
  |
Coffee
  |
Office
  |
Restaurant
  |
Office
  |
Supermarket
  |
Home
```

Generator flow:

```text
scenario waypoints
      |
      v
OSRM Route
      |
      v
road geometry
      |
      v
sample route
      |
      v
assign timestamps
      |
      v
inject GPS behavior
      |
      v
gps/1 Batch fixtures
```

---

# 43. Simulator Behaviors

Synthetic data should be able to inject:

- realistic speed variation;
- acceleration/deceleration;
- GPS jitter;
- temporary GNSS loss;
- poor HDOP intervals;
- isolated impossible jump;
- repeated road sections;
- U-turn;
- same location visited multiple times;
- long stops;
- short stops that should not classify as stops;
- route returns;
- walk/bike/car movement modes.

Synthetic data must remain valid for the Phase 1 ingestion protocol.

---

# 44. Simulator Time Model

A scenario must define logical real-world time.

Example:

```text
06:40 Home

06:40 -> 07:00
CAR

07:00 -> 07:25
STOP

07:25 -> 08:00
CAR

08:00 -> 12:00
STOP

12:00 -> 12:15
WALK

12:15 -> 13:05
STOP
```

Route sampling then derives GPS timestamps along the routed path.

This makes Phase 2 tests reproducible.

---

# 45. Processing Determinism and Matcher Evidence

The determinism contract is:

```text
Raw GPS input
+ processing version
+ configuration
+ matcher input
+ fixed matcher result/evidence
= deterministic derived result
```

An online OSRM run and an outage run have different dependency evidence and are
not required to produce the same routes. Tests freeze matcher evidence and
compare normalized derived outputs.

Each Match operation preserves sufficient provenance:

```text
matcher_input, input_hash, request parameters
engine, engine_version, dataset_version, profile
match status, confidence
normalized result OR normalized failure evidence
attempt count, started_at, finished_at
```

Outage/failure evidence must also explain why fallback was selected. Retain enough input/result or failure evidence to audit and replay; a hash alone
is insufficient. Phase 2 does not automatically garbage-collect evidence.
JSON/JSONB/compressed blob are implementation choices. A snapshot published with
raw fallback is a valid revision. OSRM recovery must not silently change it.

---

# 46. Reprocessing

Reprocessing builds a new derived revision from immutable Raw GPS and recorded
processing inputs. It does not delete the active publication while rebuilding.

```text
published R7 remains readable
  -> construct and validate R8
  -> atomic publish R8
```

Upgrading `raw_fallback` to `osrm_match` after OSRM recovery requires explicit
reprocessing and a new published revision. Dependency retries may occur before
publication under a bounded policy; a published snapshot does not mutate later.
CLI naming remains an implementation choice. Reprocessing range closure follows
§5; IDs need not remain stable across revisions (§27).

Retry is limited to transient failures such as refused connections, timeouts,
temporary network errors and selected 5xx responses. Configure finite limits:

```text
MATCH_RETRY_MAX_ATTEMPTS
MATCH_RETRY_TOTAL_BUDGET_MS
MATCH_RETRY_BASE_DELAY_MS
```

Do not apply dependency retry to NoMatch, valid low-confidence responses or
invalid matcher input: apply the explicit processing policy instead. Exhausted
transient retry budgets lead to raw fallback where safe. A future explicit
scheduled upgrade policy is outside Phase 2.

Phase 2 MUST NOT automatically delete ActivityRevision, ActivityManifest versions,
DailySnapshot versions, MatchCandidate, MatcherEvidence or rejected/superseded
candidates. Not active does not mean safe to delete. Normal processing, refresh
and publication preserve historical referenced data and its readability.

A future retention/GC feature must use reference-aware reachability across
manifests/snapshots/revisions/evidence, not simple age-based deletion. Phase 2
observes storage only; it does not implement that GC.

---

# 47. Observability

Processing logs should include:

```text
device_id
local_date
processing_version

raw_point_count
usable_point_count
excluded_point_count

stop_count
movement_count

osrm_request_count
osrm_match_success_count
osrm_match_fallback_count

processing_duration_ms
```

OSRM failures must be observable without turning the whole Daily View into a server error.

Track storage growth without deleting persisted processing artifacts:

```text
activity_revision_bytes
activity_manifest_count
daily_snapshot_bytes
matcher_candidate_bytes
matcher_evidence_bytes
```

Keep per-attempt evidence and source manifest/generation identifiers in diagnostic
logs where useful for publication/reprojection race investigation.

---

# 48. Security and Network Boundary

OSRM is internal infrastructure.

Do not publish OSRM ports to the LAN by default.

```text
LAN
 |
 v
web:8080
 |
 v
server
 |
 +-- postgres
 +-- osrm-*
```

Only `web` should own the normal public LAN application port in the default Compose topology.

---

# 49. Repository Boundaries

Existing boundaries remain:

```text
firmware/
server/
web/
protocol/
docs/
tools/
```

Suggested Phase 2 placement:

```text
server/src/processing/
+-- quality/
+-- segmentation/
+-- movement/
+-- matching/
+-- timeline/
+-- worker/

server/src/osrm/
+-- client.rs
+-- match.rs
+-- route.rs
+-- types.rs

tools/simulator/
```

Exact module names may vary, but responsibilities must remain separated.

Do not create one large Phase 2 processor file.

---

# 50. OpenAPI Ownership

Daily View and Timeline response changes belong in the canonical OpenAPI contract.

Flow:

```text
protocol/openapi
      |
      v
server implementation
      |
      v
generated Web types
```

The Web must not hand-maintain a second incompatible version of the server schema.

---

# 51. Phase 2 Ticket Plan

## P2-01 — Processing Domain & Persistence

Implement:

- activity-level Trip/Stop/Gap ownership and ordered Trip segments;
- published daily projections and processing revisions;
- separate freshness and background-work states;
- monotonic Device input_generation and captured input/configuration provenance;
- dirty-range propagation across affected days;
- range-owned immutable ActivityRevision and DailySnapshot storage;
- generation/target/fencing-verified atomic multi-day pointer swaps;
- reprocessing without removing the active snapshot.

Acceptance:

- Raw GPS remains immutable.
- Refresh can transition `queued -> running -> idle` after successful publication.
- Failed refresh is persisted and retryable while the last good snapshot remains readable.
- Cross-day events are not exclusively owned by one processed day.
- Concurrent input cannot make an outdated snapshot appear current.

## P2-02 — GPS Quality, Gap & Outlier Processing

Implement:

- usable-point classification;
- temporal gap boundaries;
- impossible-jump detection;
- deterministic exclusion reasons;
- processing metrics.

Acceptance:

- impossible jumps do not enter derived route;
- original GPS rows remain intact;
- gap boundaries are stable.

## P2-03 — Stop / Move Segmentation

Implement:

- spatial dwell clustering;
- minimum stop duration;
- stop center/radius;
- movement boundaries;
- stable-boundary range expansion and open edge activities;
- configuration surface.

Acceptance:

- realistic long stops are detected;
- short traffic pauses are not automatically stops;
- repeated visits to one location remain distinct events when separated by movement.

## P2-04 — Movement Classification

Implement:

- deterministic WALK/BIKE/CAR/UNKNOWN classifier;
- classification confidence;
- confidence-gated profile selection and within-Trip mode transitions.

Acceptance:

- representative synthetic movements classify predictably;
- low-confidence segments use UNKNOWN/raw fallback without guessed profiles;
- a mode change can split Movement Segments without splitting the Trip;
- short pauses contribute Trip duration, without being labeled physical movement.

## P2-05 — OSRM Infrastructure & Client

Implement:

- Docker services for car/bike/foot;
- MLD datasets;
- internal environment configuration;
- Rust OSRM client;
- health/error handling;
- dataset version metadata.

Acceptance:

- server can call each internal profile;
- OSRM is not exposed publicly;
- OSRM outage is handled as a processing dependency failure/fallback condition.

## P2-06 — OSRM Match Pipeline

Implement:

- Match input reduction;
- timestamp conversion;
- estimated radius generation;
- chunking;
- overlap;
- result merge;
- partial-match handling;
- confidence policy;
- raw fallback;
- Route Part/anchor/vertex-distance persistence and full source/seam evidence;
- finite retry budget.

Acceptance:

- movement segments longer than one Match chunk work;
- unmatched points do not crash processing;
- matched parts without valid temporal coverage/anchors are rejected;
- safe matched/raw hybrid parts are supported, otherwise the full segment falls back;
- matcher failures do not create GPS Gaps;
- low-confidence routes fallback deterministically;
- matched geometry persists with engine/profile/dataset metadata.

## P2-07 — Continuous Activity and Daily Publication

Implement:

- maximal continuous multimode Trip derivation;
- first-class Gap events;
- cross-midnight activity and daily projection;
- disconnected processed-day route;
- daily summary;
- trip and stop statistics;
- atomic revision publication and freshness updates.

Acceptance:

- one realistic day produces the expected ordered Trip/Stop sequence;
- daily distance and durations count only the visible intersection with the day;
- Gap time is separate and contributes no connector distance;
- crossing events preserve one identity across daily projections;
- a Trip can contain several ordered transport modes.

## P2-08 — Timeline API & Option 2 Integration

Implement:

- OpenAPI Timeline schema;
- Daily View processed response;
- generated Web types;
- real Trip/Stop/Gap Timeline items with full and visible intervals;
- Map <-> Timeline selection;
- Trip highlighting;
- Stop focus behavior.

Acceptance:

- clicking a Trip focuses/highlights its route;
- clicking a Stop focuses its location;
- clicking Map features selects the corresponding Timeline event;
- stale published snapshots remain usable during refresh and failure;
- lightweight status polling triggers one complete refetch per new publication;
- timezone changes rebuild projections without rerunning activity processing;
- a new-timezone projection may use stale published activity with truthful source freshness;
- manifest/timezone changes during reprojection reject candidate activation.

## P2-09 — Matched Route Playback

Implement:

- matched geometry playback;
- GPS-time progress mapping;
- deterministic seek;
- Trip playback support;
- optional automatic duration compression;
- raw/matched debug switch.

Acceptance:

- marker follows each selected segment geometry (matched or fallback);
- Gap intervals disclose missing GPS without animated bridging;
- replay clock follows historical timestamps;
- seek does not replay from the beginning;
- playback end returns to full-route overview.

## P2-10 — OSRM-Based Simulator & E2E Acceptance

Implement:

- scenario waypoints;
- OSRM Route-based geometry generation;
- GPS sampling;
- noise injection;
- stop injection;
- multi-mode movement;
- Batch fixture generation;
- deterministic seed/config;
- E2E acceptance scenario.

Acceptance:

```text
fixture
  |
  v
Phase 1 ingestion
  |
  v
Phase 2 processing
  |
  v
Trips / Stops / Gaps
  |
  v
OSRM Match
  |
  v
Timeline API
  |
  v
Option 2 Daily Map
  |
  v
Playback
```

must complete successfully.

---

# 52. Ticket Dependencies

```text
P2-01
  |
  v
P2-02
  |
  v
P2-03
  |
  +---------------+
  v               |
P2-04             |
  |               |
  v               |
P2-05             |
  |               |
  v               |
P2-06 <-----------+
  |
  v
P2-07
  |
  v
P2-08
  |
  v
P2-09
  |
  v
P2-10
```

P2-05 may begin in parallel with P2-02/P2-03 as infrastructure work, but P2-06 requires both segmentation semantics and the OSRM client.

---

# 53. Phase 2 End-to-End Acceptance Scenario

Use a deterministic synthetic day such as:

```text
06:40 Home

06:40 -> 07:00
CAR
Home -> Coffee

07:00 -> 07:25
STOP

07:25 -> 08:00
CAR
Coffee -> Office

08:00 -> 12:00
STOP

12:00 -> 12:15
WALK
Office -> Restaurant

12:15 -> 13:05
STOP

13:05 -> 13:20
WALK
Restaurant -> Office

13:20 -> 17:30
STOP

17:30 -> 18:10
CAR
Office -> Supermarket

18:10 -> 18:35
STOP

18:35 -> 19:00
CAR
Supermarket -> Home
```

Inject:

- GPS jitter;
- one impossible jump;
- one temporary GNSS gap;
- variable HDOP;
- repeated road section;
- same final location as the initial location.

Expected result:

- correct stop count;
- correct movement count;
- correct chronological order;
- matched routes for eligible confident-mode segments;
- UNKNOWN segments use raw fallback;
- Gap injection creates explicit missing-data events and no route connectors;
- fallback for intentionally degraded segment if configured;
- no impossible jump in processed route;
- full Raw GPS still present.

Add focused deterministic acceptance fixtures for the newly agreed semantics:

- WALK followed by CAR without a qualifying Stop or Gap: one Trip, two ordered
  Movement Segments; a short pause does not split the Trip.
- A Trip and a Stop crossing midnight: one activity event each, full/visible
  intervals in both Daily Views, and daily metrics restricted to each overlap.
- An UNKNOWN movement: raw fallback without trying an arbitrary OSRM profile.
- Late Batch affecting a crossing event: all affected projections become stale,
  the last good snapshot remains readable during refresh, and publication is
  replaced only after success; a failed refresh preserves the prior snapshot.
- Published outage fallback followed by OSRM recovery: no silent upgrade; explicit
  reprocessing creates a new revision.

Finalize expected boundaries and metrics from the settled semantics and pinned
fixture configuration. Storage activation and provenance must preserve these activity and distance
expectations.

Add lifecycle/concurrency acceptance cases:

- M10 references R10; M11 supersedes it with R11 while S5 still pins M10/R10:
  normal processing/refresh leaves R10, M10 and S5 readable.
- Projection starts on M17/T6; activity publishes M18 before projection activation:
  the M17 candidate cannot activate; create a projection for M18/T6.
- Projection starts on M18/T6; timezone changes to T7 before activation:
  the T6 candidate cannot activate.
- G143 Raw input is pending while published activity covers G142: a T6 projection
  of that publication may activate with STALE_SOURCE and truthful generation
  metadata, without advancing activity publication state.
- Overlapping revision replacements produce ordered non-overlapping manifest
  slices with no latest-revision inference; each published snapshot pins the
  manifest version from which it was built.
- A reclaimed worker's stale fencing token cannot activate a candidate, even if
  its Raw generation and processing target still happen to match.
- Concurrent new Batch commit and publication serialize through the control row;
  replay does not advance generation, and changed generation rejects a candidate.
- One continuous Trip spans [A,D), with a validated chunk seam at C and
  A < B < C < D. A replacement candidate [B,C) cannot terminate at C solely
  because it is a seam; expand/reprocess to semantic-safe activity boundaries.
- Two independently established Trip boundaries meet at C, which is also a
  validated seam: C may authorize replacement because it is a semantic Trip
  boundary. The seam adds no independent activity-boundary authority.

---

# 54. Required Acceptance Checks

Phase 2 is not complete unless all of the following are verified:

```text
[ ] Raw GPS is unchanged by processing

[ ] Reprocessing is possible

[ ] Fixed Raw GPS, version, configuration and matcher evidence produce deterministic derived outputs

[ ] Impossible jumps are excluded from processed geometry

[ ] Large gaps become explicit Gap events and terminate Trips without inferred connectors

[ ] Long stops are detected

[ ] Short pauses do not automatically become stops

[ ] MOVE segments are classified

[ ] OSRM car/bike/foot profiles are reachable internally

[ ] OSRM is not directly exposed to the browser

[ ] Long movement traces are chunked

[ ] Chunk overlap does not produce obvious duplicated geometry

[ ] Unmatched OSRM tracepoints are handled

[ ] Low-confidence matching has deterministic fallback

[ ] OSRM outage does not make historical Daily View unavailable

[ ] Matched routes persist engine/profile/dataset version

[ ] Trips contain 1..N ordered mode-homogeneous Movement Segments

[ ] Trips use selected segment geometry and may mix route sources

[ ] Cross-midnight activity events are projected without day-owned duplicates

[ ] Daily metrics include only the overlap with the selected day

[ ] Refresh keeps the last good publication and replaces it atomically

[ ] Published fallback upgrades only through explicit reprocessing

[ ] Complete matcher input/result/failure evidence is retained without automatic GC

[ ] OSRM transient retries have finite attempt and time budgets

[ ] Open activities do not treat the last observation as a confirmed end

[ ] Generation mismatch rejects candidate publication and requeues processing

[ ] Publication atomically activates every affected daily projection

[ ] Timezone change only rebuilds projections and never relabels old snapshots

[ ] Insufficient evidence may produce a successful empty processed read model

[ ] Route Parts allow several parts per segment and have valid progress anchors

[ ] Web polls lightweight status rather than full geometry

[ ] Quality-filtered observations create no artificial GPS Gap

[ ] Open activities have separate observed bounds and null actual open boundaries

[ ] Full duration is null when either boundary is open; projection never extrapolates

[ ] Reprocessing may change IDs; publication change clears Web selection/playback

[ ] Batch replay/failure never advances input_generation

[ ] Publication validates processing and projection targets as well as Raw generation

[ ] Summary counts all Raw records and positive observed event overlaps

[ ] Short pauses count as Trip time, not asserted physical movement

[ ] Activity evidence state is independent of matching/fallback quality

[ ] Distance includes only published parts, without invented connectors

[ ] Range replacement reuses immutable activity versions outside stable cuts

[ ] Activity revision ID is distinct from daily published revision

[ ] Lease-expired workers cannot publish after fencing authority changes

[ ] Crash recovery and repeated candidate execution cause no duplicate activation

[ ] Ingestion, target updates and publication serialize via the Device control row

[ ] Dirty work coalesces per Device without losing new input during processing

[ ] Evidence Holes are separate API metadata and may create interior open boundaries

[ ] Vertex distances are server-owned and match geometry coordinate count

[ ] Adjacent day clipping conserves distance within tolerance

[ ] Same-second reduction preserves source mapping and never fabricates timestamps

[ ] Chunk seams use shared source/time/progress evidence, not output vertex counts

[ ] Active manifest ranges are ordered, half-open and non-overlapping

[ ] Overlapping replacements create explicit canonical manifest splices

[ ] Snapshots pin immutable manifest/source versions rather than mutable active state

[ ] Replacement endpoints pass machine-checkable semantic boundary validation

[ ] Validated chunk seams alone never authorize ActivityRevision replacement cuts

[ ] Replacement inside a continuous activity expands past technical chunk seams

[ ] Timezone-only reprojection may publish stale source with correct provenance

[ ] Projection-only jobs never modify activity publication state

[ ] Manifest or timezone changes reject a running projection candidate

[ ] Normal processing/publication never auto-deletes historical or rejected artifacts

[ ] Historical snapshot/manifest/revision references remain readable

[ ] Processing storage growth metrics are recorded

[ ] Benchmark measurements are recorded

[ ] Stops preserve real historical duration

[ ] Timeline is chronological

[ ] Timeline selection controls Map focus/highlight

[ ] Map selection updates Timeline selection

[ ] Playback uses each segment's selected geometry: accepted match or raw fallback

[ ] Playback time follows historical GPS timestamps

[ ] Seek is deterministic

[ ] Playback completion returns to overview

[ ] Raw-vs-matched route debugging is possible

[ ] Simulator uses road-network routing rather than hand-authored dense coordinates

[ ] Simulator can generate realistic stop/move behavior

[ ] E2E fixture passes Phase 1 ingestion and Phase 2 processing
```

---

# 55. Acceptance Scale and Performance Measurements

Phase 2 acceptance is bounded to:

```text
1 Owner
1 Device
<= 30,000 GPS Records per Owner-local day
bounded test-region OSM extract covering the acceptance scenario
car / bike / foot profiles
1 processing job at a time
```

A Vietnam-wide dataset is separate deployment/scaling work and is not required
for Phase 2 completion. No processing-time SLA is set before benchmarking on the
target deployment machine.

Record at least:

- Raw GPS Record count and usable point count;
- OSRM request count;
- processing wall time;
- peak/representative memory;
- Daily View payload size;
- Daily View DB read latency.

Use the first benchmark to agree subsequent time/resource budgets.

Architectural requirements remain: Daily View reads do not call OSRM; processing
runs outside requests; Match requests are bounded; selected geometry persists;
Web renders event-level features without one DOM marker per GPS Record. Any
presentation simplification retains the authoritative processed geometry.

---

# 56. Failure Semantics

Background processing failure is explicit and distinct from publication:

```text
processing.state = failed
failure_code / failure_message recorded
published R7 remains available
publication freshness = stale
```

OSRM failure alone should usually yield a successful new revision using safe
raw fallback, rather than failing the entire processing job. A publication with
fallback can be current. Failure means a valid replacement could not be built.
When no successful snapshot exists, publication is unavailable and Raw Daily View
may be served. Insufficient evidence is a successful read model with an explicit
evidence_state, and may have empty route_parts/timeline (§4).

---

# 57. Configuration

Phase 2 processing thresholds must be configurable.

Candidate configuration:

```text
LT_PROCESSING_VERSION

LT_STOP_RADIUS_M
LT_STOP_MIN_DURATION_S

LT_MODE_WINDOW_SECONDS
LT_MODE_CHANGE_MIN_DURATION_SECONDS
LT_MODE_ENTER_CONFIDENCE
LT_MODE_EXIT_CONFIDENCE
LT_MODE_UNKNOWN_GRACE_SECONDS

LT_MAX_IMPLIED_SPEED_MPS
LT_MAX_GAP_SECONDS

LT_MATCH_CHUNK_POINTS
LT_MATCH_OVERLAP_POINTS
LT_MATCH_SEAM_TOLERANCE_M

LT_MATCH_CONFIDENCE_HIGH
LT_MATCH_CONFIDENCE_MEDIUM

LT_MATCH_DEFAULT_RADIUS_M
LT_MATCH_HDOP_ERROR_FACTOR

LT_MATCH_RETRY_MAX_ATTEMPTS
LT_MATCH_RETRY_TOTAL_BUDGET_MS
LT_MATCH_RETRY_BASE_DELAY_MS

LT_OSRM_CAR_URL
LT_OSRM_BIKE_URL
LT_OSRM_FOOT_URL
```

Exact environment-variable names may change during implementation, but configuration must remain centralized and documented.

---

# 58. Migration Compatibility

Phase 2 must preserve Phase 1 ingestion compatibility.

Existing firmware using `gps/1` continues uploading unchanged.

Phase 2 must not require an immediate firmware upgrade merely to enable Timeline processing.

New optional protocol fields, if ever introduced, require explicit protocol versioning.

---

# 59. Web Compatibility During Processing

The Web handles processing and publication independently:

```text
published snapshot + idle/current
  -> processed Daily View
published snapshot + queued/running/stale
  -> keep processed Daily View; indicate refresh
published snapshot + failed/stale
  -> keep processed Daily View; indicate refresh failure
no published snapshot + unavailable
  -> Raw Daily View or empty/status presentation as appropriate
```

Do not revert a successfully processed day to Raw Daily View merely because a
refresh starts or fails. Snapshot route, timeline and summary must be consistent.
Web polls the lightweight status endpoint every 5 seconds while processing is
queued/running. It stops when idle, failed or `document.hidden`. On visibility
or focus restoration, immediately refetch status.

When `published_revision` changes, fetch the full Daily View once and replace
route, timeline and summary from that complete response. Do not merge R7 fields
with R8 fields or poll full geometry every 5 seconds. If revision is unchanged,
keep displayed snapshot data. When it changes, clear selection, pause and reset
playback before atomically replacing the displayed snapshot. Projection/timezone identity also keys the query:
an old-timezone response cannot replace a current-timezone view. SSE/WebSocket
are not required in Phase 2.

---

# 60. Definition of Done

Phase 2 is complete when a newly ingested realistic day can move through the entire system without manual DB editing:

```text
Device/simulator
      |
      v
Phase 1 Batch ingestion
      |
      v
Raw GPS persisted
      |
      v
Phase 2 processing worker
      |
      +-- quality
      +-- gaps
      +-- stops
      +-- moves
      +-- classification
      +-- OSRM Match
      +-- timeline
      |
      v
Processed Daily View
      |
      v
Option 2 Web
      |
      +-- Summary
      +-- Timeline
      +-- Matched Map
      +-- Trip/Stop interaction
      +-- Historical playback
```

The implementation must satisfy these architectural guarantees:

1. Raw GPS is never replaced by matched data.
2. OSRM is internal infrastructure.
3. OSRM is not called synchronously by Daily View reads.
4. Every Movement Segment has a safe non-OSRM fallback policy without invented evidence.
5. Trip/Stop/Gap are server-derived continuous activity; Daily View is a published projection.
6. The Web remains read-oriented.
7. Playback uses selected segment geometry and historical GPS time, without interpolation across Gaps.
8. Processing is versioned and re-runnable; publication is atomic and preserves the last good snapshot.
9. Matcher evidence and OSRM dataset/profile/version are persisted with matched output.
10. The current Option 2 Web architecture is extended, not replaced.

---

# 61. Future After Phase 2

Phase 3 may add Image Timeline Events:

```text
PHOTO
```

Phase 4 may add Audio Timeline Events:

```text
AUDIO
```

The Phase 2 Timeline model should therefore avoid hard-coding the UI around only Trip/Stop.

Long-term:

```text
GPS
Trip
Stop
Photo
Audio
Custom Event
      |
      v
Unified LifeTrail Timeline
```

Phase 2 establishes the processing and interaction model that those later media phases will reuse.


---

# 62. Consolidated Decisions — Design Tree Closed

Rounds 1–6 on 2026-10-06 closed Q1–Q31. The shared domain, publication, geometry,
storage and concurrency decisions are agreed; no design-interview frontier remains.
Earlier proposed Trip/Stop schema sketches and SSE expectations in
docs/server/architecture.md are superseded for Phase 2 by this specification and
the accepted ADRs; they are not migration/API contracts.

The agreed design separates immutable Raw observations, continuous activity,
range-owned immutable revisions, canonical versioned manifests and timezone-aware
published daily projections. Lease/fencing, generation and target checks guard
short atomic activation; OSRM processing stays outside transactions. Progress
anchors and server-owned vertex distances govern playback and clipping. Gap and
Evidence Hole remain distinct. No automatic GC or fallback upgrade occurs.

Q31 closes the boundary distinction: chunk seams are technical RoutePart
boundaries, not ActivityRevision boundaries. A seam may coincide with a valid
replacement endpoint only when that timestamp is independently semantic-safe.
Otherwise expand the replacement range rather than split continuous activity.

Threshold values, retry/lease durations, physical evidence storage formats and
SQL naming remain implementation choices verified with fixtures and race tests.
Implementation has not started during this design interview. The agreed baseline
is ready for detailed migration and worker-protocol design.
