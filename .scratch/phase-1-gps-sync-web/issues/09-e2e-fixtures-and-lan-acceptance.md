# 09 — Prove offline-to-web LAN acceptance

Status: claimed
Type: task
Blocked by: 04, 06, 08

## Goal

Create reproducible fixtures, simulator support where useful, and the documented hardware/software acceptance run for Phase 1.

## Scope

- Add host fixtures/tools to inspect a Batch, verify manifest/hash/framing, and replay a committed body unchanged.
- Automate server/Web integration gates where hardware is not needed.
- Document and execute the physical Device acceptance sequence in the Phase 1 spec.
- Validate built Vue plus server on one LAN origin, not only Vite dev mode.

## Acceptance criteria

- One Device records outdoors without Wi-Fi for at least 10 minutes and produces at least two ready Batches.
- A power-cycle during recording preserves complete records, recovers open state, and resumes recording.
- LAN sync commits at least two Batches; replay returns `duplicate: true` without raising GPS row count.
- Canonical Daily View renders nonempty route/start/end/summary and an independently chosen empty day renders the zero/null state.
- Evidence distinguishes automated source/test results from real hardware/LAN execution.

## Blocked by

04, 06, 08.

## Comments

Implemented the repeatable host/Compose portion:

- `tools/batch_acceptance.py` inspects immutable ready/manifest pairs and can
  either commit-then-replay a new Batch or verify a previously committed Batch
  remains `duplicate: true` without reserializing its body.
- `tools/verify_single_origin.py`, a server regression test, and the Compose
  run verify the built SPA and canonical Daily View URL are served from the
  same origin; the server now returns `200` for that canonical URL.
- `docs/development/phase-1-lan-acceptance.md` records the exact physical
  Device procedure and evidence required for the remaining acceptance gates.

Automated source, firmware-host, Compose, PostGIS, and Web checks passed.
No physical ESP32, GNSS, microSD, outdoor recording, power-cycle, or second
LAN client was available in this run. The ticket remains `claimed` until that
evidence is captured; it is not resolved by simulator or source-level tests.
