# 06: Project continuous activity across midnight

Status: ready-for-agent
Type: task
Labels: ready-for-agent
Blocked by: 05

**What to build:** The Owner can inspect neighboring local days without a crossing Trip or Stop becoming duplicate domain events, and daily durations/distances account only for each day's observed overlap.

**Blocked by:** 05 — Distinguish quality failures, GPS Gaps and Evidence Holes.

## Acceptance criteria

- [ ] Processing reads context beyond calendar boundaries and stores UTC activity independently of any one processed day; source event identity is shared across its daily projections.
- [ ] Project using the current Owner IANA timezone and half-open local-midnight-to-next-midnight bounds, including DST-aware intervals rather than an assumed 24-hour day.
- [ ] Expose actual and observed event intervals plus clipped visible intervals, open boundary states and continuation flags based only on observed coverage.
- [ ] A confirmed 23:50–00:20 Stop remains one source event and contributes 10/20 minutes with one daily Stop count in each positive-overlap day.
- [ ] Clip Route Parts with the same historical anchors, synthetic midnight anchors and server progress measure; rebase clipped progress to zero and never create new Raw GPS Records.
- [ ] Daily part distance is the difference in original progress at clipping endpoints; adjacent daily totals conserve original distance within a pinned numerical tolerance.
- [ ] Counts/durations use strictly positive observed event overlap. Open activities do not extrapolate to midnight, current time or the end of another day.
- [ ] Web shows continuation/open semantics and fits only visible Trip geometry; published snapshots retain correct manifest/timezone/source provenance.
- [ ] Integration/Web tests cover crossing Trip/Stop/Gap/Evidence Hole, Route clipping, exact day-boundary observations, DST, single-point cases and distance conservation.
