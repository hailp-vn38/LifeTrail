# 15: Verify Phase 2 end to end and measure a 30k-record day

Status: ready-for-agent
Type: task
Labels: ready-for-agent
Blocked by: 08, 09, 12, 13, 14

**What to build:** An operator can run a reproducible Phase 2 acceptance session from simulator Batch upload through durable processing, persisted API, Timeline/Map and historical playback, and inspect benchmark/storage measurements without manual DB edits.

**Blocked by:** 08 — Recover worker crashes without stale publication; 09 — Reproject published history after timezone changes; 12 — Publish long traces with safe chunk seams and hybrid fallback; 13 — Replay published Route Parts and compare Raw GPS; 14 — Generate reproducible realistic days and GPS failures.

## Acceptance criteria

- [ ] Exercise one Owner/Device, up to 30,000 Records per local day, bounded car/bike/foot extracts and one worker; pin machine, scenario, configuration and engine/dataset identities.
- [ ] Complete normal ingestion-to-processed-Timeline/Map/playback for the realistic day and focused multimode, midnight, repeated-route, sparse, jump, quality and GPS-gap variants.
- [ ] Demonstrate Trip/Stop selection in both directions, mode/confidence/source display, truthful open boundaries, Raw debug comparison, deterministic seek and final camera overview with actual rendered interactions.
- [ ] Run the application with OSRM offline and after recovery: historical persisted reads remain available, finite retry can publish fallback, and no silent upgrade occurs before explicit reprocessing.
- [ ] Integrate late cross-day replacement, concurrent generation/target publication checks, reclaimed-worker fencing, no lost coalesced work and no duplicate activation.
- [ ] Integrate timezone stale-source reprojection and both manifest/timezone activation races without relabeling old data or modifying activity publication.
- [ ] Prove fixed matcher-evidence replay yields normalized deterministic output and retained historical snapshots/manifests/revisions/evidence remain readable after supersession; no automatic GC occurs.
- [ ] Verify no LAN OSRM port exposure, no synchronous OSRM on reads, no per-GPS-Record DOM rendering and no firmware/protocol upgrade requirement.
- [ ] Record Raw/usable counts, OSRM requests, processing wall time, peak/representative memory, Daily View payload size and DB read latency plus revision/manifest/snapshot/candidate/evidence storage metrics.
- [ ] Do not invent a processing-time SLA; report measured results and conditions for a subsequent budget decision. Run required real-PostGIS/live-OSRM checks explicitly rather than treating skipped tests as passes.
- [ ] Document a repeatable operator acceptance procedure, expected scenarios and evidence/results using the existing testing seams; no private helper structure is asserted as product behavior.
