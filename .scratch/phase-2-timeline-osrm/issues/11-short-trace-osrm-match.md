# 11: Publish safe road matches for short Movement Segments

Status: ready-for-agent
Type: task
Labels: ready-for-agent
Blocked by: 02, 10

**What to build:** The Owner sees a short confidently classified segment follow its matching road profile, while outage or unsuitable matching still produces a usable fallback view. Every published geometry retains reliable historical-time mapping and explainable provenance.

**Blocked by:** 02 — Generate routed Batch fixtures through internal OSRM; 10 — Show multimode Trips with persistent mode evidence.

## Acceptance criteria

- [ ] The server calls internal OSRM Match on eligible short segments using the selected foot/bike/car profile; UNKNOWN never calls a guessed profile and the browser never calls OSRM.
- [ ] Construct bounded usable Match input with original-source mapping and strictly increasing second timestamps. Same-second selection uses the agreed explicit quality/nullable ranking and never invents adjusted timestamps.
- [ ] Estimate radius from HDOP/error factor or configurable fallback; preserve quality/confidence/input/matched/outlier metrics without claiming exact receiver accuracy.
- [ ] Accept matched parts only with valid geometry, known temporal coverage and strictly increasing historical anchors with non-decreasing bounded progress and valid endpoint anchors.
- [ ] Publish server-owned vertex distances and Route Part lengths through the existing generated API/Map contract; no timestamp-per-OSRM-vertex assumption or OSRM-duration clock is introduced.
- [ ] NoMatch, low confidence, invalid temporal mapping or exhausted dependency errors apply explicit policy and safe raw fallback; no matcher failure creates a GPS Gap.
- [ ] Transient retry is bounded by configured attempts, delay and total budget. Do not retry NoMatch/valid low confidence/invalid input as transient dependency failure.
- [ ] Retain actual matcher input/parameters, selection mapping, hash, engine/profile/dataset/config identities, normalized result/failure, confidence/status and per-attempt timing.
- [ ] Historical reads use persisted geometry and succeed with OSRM offline; published fallback does not silently upgrade when OSRM recovers. Explicit reprocessing creates a new retained revision.
- [ ] Use frozen fake-OSRM HTTP evidence for deterministic processing/outage tests and a live bounded Compose example for each eligible profile, visible on the processed Daily Map.
