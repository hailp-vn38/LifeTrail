# 14: Generate reproducible realistic days and GPS failures

Status: ready-for-agent
Type: task
Labels: ready-for-agent
Blocked by: 02

**What to build:** An operator can generate a complete plausible road-network day with expected activity, mode transfers and deliberately degraded GPS, then upload it as unchanged gps/1 Batches to exercise the processed experience reproducibly.

**Blocked by:** 02 — Generate routed Batch fixtures through internal OSRM.

## Acceptance criteria

- [ ] Extend timed waypoint/profile scenarios with Home/Coffee/Office/Restaurant/Supermarket/return visits, walking/cycling/driving, revisits, repeated roads and U-turns.
- [ ] Sample logical historical time with speed variation and acceleration/deceleration rather than substituting OSRM estimated duration as the recorded clock.
- [ ] Inject seeded jitter, poor HDOP/quality, isolated impossible jump, temporary GNSS observation loss, long Stops and short pauses while retaining valid protocol framing/values.
- [ ] Emit scenario/ground-truth metadata, seed/configuration, routing versions and valid Batch/manifests that can be inspected, committed and replayed using normal tooling.
- [ ] Include focused variants for a mode transfer without Stop/Gap, cross-midnight Trip/Stop, interior Evidence Hole, late upload and intentionally degraded route processing.
- [ ] Expected scenarios distinguish GPS absence from poor-quality observations and physical movement from Trip time; do not bake guessed processing outcomes into raw protocol data.
- [ ] Tool tests freeze Route dependency evidence for byte/scenario reproducibility, and an operator demo uploads the generated fixture and views it through the existing Raw Daily Map.
- [ ] This ticket does not require classification, matching publication or playback to run; full processed acceptance of these fixtures is integrated in the final ticket.
