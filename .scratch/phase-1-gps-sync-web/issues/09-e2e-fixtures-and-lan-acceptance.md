# 09 — Prove offline-to-web LAN acceptance

Status: open
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
