# 03: Show observed Stops in the Timeline

Status: resolved
Type: task
Labels: ready-for-agent
Blocked by: 01

**What to build:** The Owner can open a recorded stationary period, see a server-derived Stop with observed duration, and select it to focus its location/radius on the map. Incomplete arrival/departure times are shown as unknown rather than fabricated.

**Blocked by:** 01 — Publish Daily Snapshots for sparse evidence.

## Acceptance criteria

- [x] Spatial dwell and configurable radius/minimum duration detect a qualifying Stop; speed equal to zero alone does not establish a Stop.
- [x] Create immutable UTC Stop activity with revision-local identity, server-derived center/radius, quality/source counts and explicit actual versus observed boundaries.
- [x] At an observation edge, actual start/end is null with an open boundary state. Observed bounds/duration remain known; full duration is null if either actual boundary is open.
- [x] Publish and expose a daily Stop projection through the existing worker/snapshot contract; activity is not owned exclusively by a calendar-day row.
- [x] The generated API/Web contract renders Stop Timeline items, observed time/duration, confirmed or unknown arrival/departure, and positive-observed-overlap daily Stop counts/duration.
- [x] Timeline selection highlights the Stop marker/radius and focuses its center; selecting its map feature selects the same Timeline item.
- [x] Fixtures distinguish qualifying dwell from a short pause and verify no now()/day-end extrapolation. Unresolved observations remain explicit rather than guessed activity.
- [x] Integration and component tests exercise stationary upload-to-published-Stop behavior with Raw immutability and the established empty/Raw views remaining usable.

## Answer

Implemented and verified. See [operator behavior and acceptance evidence](../../../docs/development/phase-2-stationary-stop-acceptance.md) and [independent Standards/Spec review](../review-03.md).

## Comments

2026-10-06: Delivered continuous UTC spatial dwell, immutable Stop revisions and daily projections, open/confirmed boundaries, positive-overlap totals, explicit unresolved evidence and synchronized Timeline/map selection. Raw GPS remains immutable and explicitly selectable after publication. TDD regressions exercised cross-midnight Stop publication and the Raw-view review fix. All 13 server tests passed with real PostGIS integration included; all 149 Web tests and eight simulator tests passed. Typecheck, formatting, strict Clippy, production build, OpenAPI response validation and built-Web Chromium acceptance passed. Independent reviews found one naming heuristic and one Raw-view regression; both were fixed and rechecked with zero remaining findings. Full-range capture is the initial Stop implementation; incremental semantic-range reuse remains ticket 07.
