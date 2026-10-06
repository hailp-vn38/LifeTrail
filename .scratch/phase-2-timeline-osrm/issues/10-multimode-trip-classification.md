# 10: Show multimode Trips with persistent mode evidence

Status: resolved
Type: task
Labels: ready-for-agent
Blocked by: 05

**What to build:** The Owner sees walking, cycling and driving portions inside one continuous Trip, with UNKNOWN where evidence is weak. Brief noise does not create mode oscillation, new Trips or false Stops.

**Blocked by:** 05 — Distinguish quality failures, GPS Gaps and Evidence Holes.

## Acceptance criteria

- [x] Compute mode evidence over configurable time windows and feed a persistence/hysteresis state machine rather than cutting a segment per GPS Record.
- [x] Accept transitions only after enter confidence and minimum duration; use lower exit threshold and configurable unknown grace to tolerate brief compatible interruptions.
- [x] Sustained unknown evidence remains an ordered UNKNOWN segment. A short pause neither creates a Stop below Stop criteria nor itself forces a mode transition.
- [x] WALK followed by CAR without Stop/Gap is one Trip with two ordered relatively homogeneous segments; Trip source/mode/confidence is not collapsed to one segment-level value.
- [x] Persist confidence, configuration identity and source boundaries in the immutable activity revision; expose segment mode/confidence through generated API and Timeline/map presentation.
- [x] Only sufficiently confident WALK/BIKE/CAR selects foot/bike/car. UNKNOWN and below-threshold candidates stay raw fallback; matching success never determines transport mode.
- [x] Keep published Route Part metric/anchors and Trip-duration semantics unchanged while splitting mode segments; total published distance does not acquire connectors.
- [x] Tests cover all modes, low confidence, rapid candidate oscillation, brief versus sustained UNKNOWN, traffic pauses and continuous mode transfers through upload-to-published-Timeline behavior.

## Answer

Implemented windowed deterministic WALK/BIKE/CAR evidence with persisted configuration, enter/exit hysteresis and UNKNOWN grace. A continuous movement run is now partitioned into ordered Movement Segments without creating a Trip boundary; each segment and its raw Route Part carries its own mode and classification confidence. The immutable revision retains the complete classifier configuration and algorithm identity. OpenAPI, generated Web types and Timeline labels expose the segment facts while preserving Route Part anchors and published distance accounting.
