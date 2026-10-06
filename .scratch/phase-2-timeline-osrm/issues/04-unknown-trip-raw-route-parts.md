# 04: Show UNKNOWN Trips with raw Route Parts

Status: ready-for-agent
Type: task
Labels: ready-for-agent
Blocked by: 03

**What to build:** The Owner can see movement between Stops as a Trip with ordered UNKNOWN Movement Segments, inspect its raw-derived Route, and select the Trip from either Timeline or Map. Route Parts establish the shared geometry/progress contract before matching is introduced.

**Blocked by:** 03 — Show observed Stops in the Timeline.

## Acceptance criteria

- [ ] Derive continuous Trip movement bounded by supported Stop transitions or open observation edges; a short pause below Stop criteria does not itself end a Trip.
- [ ] A Trip contains ordered Movement Segments rather than one movement_segment_id. This slice uses UNKNOWN mode without guessing an OSRM profile.
- [ ] Each drawable raw Route Part identifies its segment/source/observed coverage and has a valid LineString, strictly increasing historical-time anchors and non-decreasing bounded progress, including endpoint anchors.
- [ ] Publish server-computed vertex_distance_m aligned with coordinates, zero origin and final part distance; never fabricate a second coordinate or timestamp.
- [ ] Daily distance uses published part lengths without invented connectors. Trip time includes short pauses and is labeled trip_duration_s rather than physical moving duration.
- [ ] Projected Trip/Stop items are chronological, preserve actual/observed/visible boundary distinctions, and have IDs from their creating activity revision.
- [ ] Trip selection highlights/fits visible geometry; selecting its map Route selects the Timeline item. The complete-day Route remains distinguishable.
- [ ] Expose generated API types and static processed rendering without assuming one timestamp per geometry vertex. Processed playback is delivered separately; do not pass parts into the legacy Raw-only clock contract.
- [ ] Tests cover clean movement, short pauses, repeated location visits and sparse geometry, asserting upload-to-Trip response/render behavior and server progress invariants.
