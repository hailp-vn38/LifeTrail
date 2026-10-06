# 14: Generate reproducible realistic days and GPS failures

Status: resolved
Type: task
Labels: ready-for-agent
Blocked by: 02

**What to build:** An operator can generate a complete plausible road-network day with expected activity, mode transfers and deliberately degraded GPS, then upload it as unchanged gps/1 Batches to exercise the processed experience reproducibly.

**Blocked by:** 02 — Generate routed Batch fixtures through internal OSRM.

## Acceptance criteria

- [x] Extend timed waypoint/profile scenarios with Home/Coffee/Office/Restaurant/Supermarket/return visits, walking/cycling/driving, revisits, repeated roads and U-turns.
- [x] Sample logical historical time with speed variation and acceleration/deceleration rather than substituting OSRM estimated duration as the recorded clock.
- [x] Inject seeded jitter, poor HDOP/quality, isolated impossible jump, temporary GNSS observation loss, long Stops and short pauses while retaining valid protocol framing/values.
- [x] Emit scenario/ground-truth metadata, seed/configuration, routing versions and valid Batch/manifests that can be inspected, committed and replayed using normal tooling.
- [x] Include focused variants for a mode transfer without Stop/Gap, cross-midnight Trip/Stop, interior Evidence Hole, late upload and intentionally degraded route processing.
- [x] Expected scenarios distinguish GPS absence from poor-quality observations and physical movement from Trip time; do not bake guessed processing outcomes into raw protocol data.
- [x] Tool tests freeze Route dependency evidence for byte/scenario reproducibility, and an operator command uploads the generated fixture for existing Raw Daily Map inspection.
- [x] This ticket does not require classification, matching publication or playback to run; full processed acceptance of these fixtures is integrated in the final ticket.

## Answer

Implemented a deterministic multi-mode OSRM-day generator with named Home, Coffee,
Office, Restaurant and Supermarket visits, repeat/reverse roads, long Stops and a
short pause. Logical timestamps use seeded variable-speed sampling rather than Route
duration. The generator emits protocol-valid immutable Batches/manifests, factual
scenario/ground-truth sidecars and per-route frozen OSRM evidence/version provenance.
It supports seeded poor-quality observations, an impossible jump and GNSS observation
loss as separate Raw facts. Focused variant guidance points to the committed
multimode, cross-midnight, Evidence Hole, late-upload and matcher-failure fixtures.

Operator usage (including upload/replay and Raw Daily Map inspection) is documented
in `tools/README.md`. Tool tests verify byte-stable regeneration, normal Batch
framing, historical logical time, quality-versus-absence behavior and routing
evidence. Verified with `PYTHONPATH=tools python3 -m unittest discover -s tools/tests -v`.
