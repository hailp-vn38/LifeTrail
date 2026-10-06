# 11: Publish safe road matches for short Movement Segments

Status: resolved
Type: task
Labels: ready-for-agent
Blocked by: 02, 10

**What to build:** The Owner sees a short confidently classified segment follow its matching road profile, while outage or unsuitable matching still produces a usable fallback view. Every published geometry retains reliable historical-time mapping and explainable provenance.

**Blocked by:** 02 — Generate routed Batch fixtures through internal OSRM; 10 — Show multimode Trips with persistent mode evidence.

## Acceptance criteria

- [x] The server calls internal OSRM Match on eligible short segments using the selected foot/bike/car profile; UNKNOWN never calls a guessed profile and the browser never calls OSRM.
- [x] Construct bounded usable Match input with original-source mapping and strictly increasing second timestamps. Same-second selection uses the agreed explicit quality/nullable ranking and never invents adjusted timestamps.
- [x] Estimate radius from HDOP/error factor or configurable fallback; preserve quality/confidence/input/matched/outlier metrics without claiming exact receiver accuracy.
- [x] Accept matched parts only with valid geometry, known temporal coverage and strictly increasing historical anchors with non-decreasing bounded progress and valid endpoint anchors.
- [x] Publish server-owned vertex distances and Route Part lengths through the existing generated API/Map contract; no timestamp-per-OSRM-vertex assumption or OSRM-duration clock is introduced.
- [x] NoMatch, low confidence, invalid temporal mapping or exhausted dependency errors apply explicit policy and safe raw fallback; no matcher failure creates a GPS Gap.
- [x] Transient retry is bounded by configured attempts, delay and total budget. Do not retry NoMatch/valid low confidence/invalid input as transient dependency failure.
- [x] Retain actual matcher input/parameters, selection mapping, hash, engine/profile/dataset/config identities, normalized result/failure, confidence/status and per-attempt timing.
- [x] Historical reads use persisted geometry and succeed with OSRM offline; published fallback does not silently upgrade when OSRM recovers. Explicit reprocessing creates a new retained revision.
- [x] Use frozen fake-OSRM HTTP evidence for deterministic processing/outage tests and a live bounded Compose example for each eligible profile, visible on the processed Daily Map.

## Answer

Implemented the server-owned short-segment OSRM Match adapter. Confident WALK/BIKE/CAR segments select only foot/bike/car respectively; UNKNOWN and low-confidence segments retain raw fallback without an OSRM request. Matcher inputs retain original record IDs and timestamps, deterministically select one source record per epoch-second, derive bounded HDOP-based radiuses, and retain request hash, engine/dataset/config identity, normalized decision and attempt timings with the immutable Route Part evidence.

Successful matches publish server-owned vertex distances and strictly increasing historical progress anchors; invalid geometry, non-monotonic anchors, NoMatch, low confidence, invalid input, and bounded dependency failures preserve raw Route Parts rather than creating GPS Gaps. The Activity Revision now captures matcher policy and provenance, so historical Daily View reads use persisted geometry and never invoke OSRM.
