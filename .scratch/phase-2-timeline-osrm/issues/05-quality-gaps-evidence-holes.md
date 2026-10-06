# 05: Distinguish quality failures, GPS Gaps and Evidence Holes

Status: ready-for-agent
Type: task
Labels: ready-for-agent
Blocked by: 04

**What to build:** The Owner sees a truthful processed day when GPS contains jumps, outages or unusable observations: no impossible Route excursion, no invented connector, and separate explanations for missing GPS versus evidence that cannot establish activity.

**Blocked by:** 04 — Show UNKNOWN Trips with raw Route Parts.

## Acceptance criteria

- [ ] Classify usable/low-quality/excluded observations deterministically without mutating Raw GPS; configurable implied-speed/jump/quality policies reject impossible excursions from derived geometry.
- [ ] Detect temporal GPS Gaps from absence of Raw observations between known observations before quality filtering; do not create Gaps before the first or after the last Raw Record.
- [ ] A GPS Gap terminates supported Trip continuity, contributes only Gap duration and does not infer movement/Stop or any straight Route connector.
- [ ] Existing but unreliable observations produce Evidence Hole coverage metadata rather than a GPS Gap; activities on either side may have open actual boundaries.
- [ ] Expose evidence_holes with projected intervals, reasons and Raw counts separately from Timeline events. Do not assert one Trip or an inferred Stop across the hole.
- [ ] Evidence state reflects supported activity coverage: sufficient, partial or insufficient. Entirely unusable data can produce a successful empty view; usable/excluded counts never exceed total Raw count.
- [ ] Map and Timeline show disconnected geometry and distinct Gap/Evidence Hole explanations. No DOM marker is created per GPS Record.
- [ ] Use fixtures with a genuine timestamp absence, continuous low-quality observations and one impossible jump; test public read responses and rendered distinctions, not private classifier call order.
- [ ] Raw timestamps and observations remain unchanged; this slice must not introduce OSRM-based filling or automatic deletion.
