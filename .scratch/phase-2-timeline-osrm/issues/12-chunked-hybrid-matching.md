# 12: Publish long traces with safe chunk seams and hybrid fallback

Status: resolved
Type: task
Labels: ready-for-agent
Blocked by: 11

**What to build:** The Owner can inspect long journeys without request-size failures, duplicated geometry or invented joins. Partial matching preserves safe matched portions and uses raw fallback where historical coverage supports it.

**Blocked by:** 11 — Publish safe road matches for short Movement Segments.

## Acceptance criteria

- [x] Reduce redundant Match inputs without ignoring timestamps/turns/boundaries/quality; persist source-index/time/selection reasons and unavoidable temporal-resolution losses.
- [x] Chunk long inputs with configurable bounds/overlap; initial 80/5 defaults are tuning choices, not protocol invariants.
- [x] Seam candidates share source observations with compatible temporal/progress direction and geometric tolerance; optional bearing checks cannot substitute for temporal evidence.
- [x] Choose seams with a deterministic total ordering by shared quality, geometry separation, bearing discontinuity when available, overlap-midpoint proximity and source ID; missing optional metrics rank explicitly.
- [x] Do not drop a fixed number of output vertices or insert a straight connector to conceal differing roads. Persist chunk/shared-source IDs, selected progress in both chunks, separation and decision/fallback reason.
- [x] One segment can contain multiple matched or raw parts. Hybrid matched/raw/matched requires safe temporal boundaries and independently valid part geometry/coverage/anchors.
- [x] If safe partial or seam coverage cannot be established, reject the matched geometry and fall back the whole affected Movement Segment. Matching splits/failure do not become GPS Gaps.
- [x] Trip/day distance counts published parts only and remains consistent with existing clipping/vertex metric; no overlapping coverage duplicates distance.
- [x] Validated chunk seams authorize only Route Part assembly. Semantic replacement expansion from the publication contract is unchanged.
- [x] Tests exercise long traces, same-second ties, nullable metadata, turns, repeated roads, U-turns, unmatched points, divergent seams, invalid anchors, safe hybrid publication and whole-segment fallback visible through API/Map.

## Answer

Implemented bounded OSRM Match chunking with configurable 80-point/5-point-overlap defaults. Match evidence now retains each chunk's selected source IDs, selection ranking and seam decision (both chunk progress values, separation and explicit absent-bearing ranking). Chunks assemble only across a shared source observation with compatible geometry; pieces are clipped at the selected observed progress, never joined with a straight connector. An unsafe seam falls back to the original whole Movement Segment, and chunk parts update the owning Movement Segment's published part IDs and distance totals.
