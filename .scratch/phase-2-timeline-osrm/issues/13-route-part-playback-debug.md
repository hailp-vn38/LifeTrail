# 13: Replay published Route Parts and compare Raw GPS

Status: ready-for-agent
Type: task
Labels: ready-for-agent
Blocked by: 05

**What to build:** The Owner can play, seek and inspect any published Route Part using historical GPS time and the server distance metric, with clear missing/uncertain interval behavior and a developer Raw-versus-processed comparison.

**Blocked by:** 05 — Distinguish quality failures, GPS Gaps and Evidence Holes.

## Acceptance criteria

- [ ] Playback consumes canonical ordered Route Parts, observed coverage, historical progress anchors and server vertex distances; it supports raw_fallback and osrm_match using the same generated contract.
- [ ] Position comes from interpolated anchored progress, never OSRM estimated travel duration, vertex index timing or client-redefined Haversine length.
- [ ] Play/pause/restart/seek and existing speeds remain available; deterministic seek locates the position without replaying from the beginning, including repeated sections and turns.
- [ ] Do not interpolate between disconnected parts. At a GPS Gap hold the last observation, disclose missing GPS and jump at the next observation timestamp; historical clock may pass through the interval.
- [ ] Evidence Holes disclose insufficient activity evidence with a distinct message and no invented movement or Stop. Smart compression is not required.
- [ ] Preserve Course-Up/Heading-Up puck, look-ahead follow camera and final full-route overview for both daily and Trip playback.
- [ ] Expose Raw/processed debug display without confusing Raw observations with selected derived parts or changing persisted history.
- [ ] Publication replacement clears selection, pauses/resets playback and initializes the new complete snapshot; stale timezone/revision responses cannot reinitialize current playback.
- [ ] Use existing playback math/controller/store/map-lifecycle test seams, a contract-valid matched fixture without needing live matching, and component/browser acceptance for controls, holes and camera behavior.
