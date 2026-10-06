# 03: Show observed Stops in the Timeline

Status: ready-for-agent
Type: task
Labels: ready-for-agent
Blocked by: 01

**What to build:** The Owner can open a recorded stationary period, see a server-derived Stop with observed duration, and select it to focus its location/radius on the map. Incomplete arrival/departure times are shown as unknown rather than fabricated.

**Blocked by:** 01 — Publish Daily Snapshots for sparse evidence.

## Acceptance criteria

- [ ] Spatial dwell and configurable radius/minimum duration detect a qualifying Stop; speed equal to zero alone does not establish a Stop.
- [ ] Create immutable UTC Stop activity with revision-local identity, server-derived center/radius, quality/source counts and explicit actual versus observed boundaries.
- [ ] At an observation edge, actual start/end is null with an open boundary state. Observed bounds/duration remain known; full duration is null if either actual boundary is open.
- [ ] Publish and expose a daily Stop projection through the existing worker/snapshot contract; activity is not owned exclusively by a calendar-day row.
- [ ] The generated API/Web contract renders Stop Timeline items, observed time/duration, confirmed or unknown arrival/departure, and positive-observed-overlap daily Stop counts/duration.
- [ ] Timeline selection highlights the Stop marker/radius and focuses its center; selecting its map feature selects the same Timeline item.
- [ ] Fixtures distinguish qualifying dwell from a short pause and verify no now()/day-end extrapolation. Unresolved observations remain explicit rather than guessed activity.
- [ ] Integration and component tests exercise stationary upload-to-published-Stop behavior with Raw immutability and the established empty/Raw views remaining usable.
