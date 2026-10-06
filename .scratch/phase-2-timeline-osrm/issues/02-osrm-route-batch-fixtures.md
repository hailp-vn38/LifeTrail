# 02: Generate routed Batch fixtures through internal OSRM

Status: resolved
Type: task
Labels: ready-for-agent
Blocked by: None (can start immediately)

**What to build:** An operator can prepare a bounded map extract, generate valid gps/1 Batches from timed waypoints using internal OSRM Route, upload them through existing ingestion, and inspect the resulting Raw Daily Map without manually authoring dense road coordinates.

**Blocked by:** None (can start immediately).

## Acceptance criteria

- [x] Car, bike and foot profiles run as separate internal Docker services using their own extract/partition/customize MLD datasets; pin and record engine/profile/dataset identities.
- [x] The test-region extract covers the demonstration waypoints; no Vietnam-wide dataset is needed and OSRM ports are not exposed to the LAN by default.
- [x] A documented operator command accepts scenario waypoints, logical historical timing, mode/profile and a reproducible seed, obtains Route geometry and samples valid GPS Records.
- [x] Generated Batches/manifests satisfy existing gps/1 framing, hashes, timestamp and commit/replay contracts without changing firmware or protocol.
- [x] Commit a generated fixture through normal HTTP ingestion and view its Raw Route using the existing Daily View/Web path; replay preserves idempotency.
- [x] Route generation is an operator/simulator capability, not planned navigation or a browser-facing OSRM API. Nearest remains an internal diagnostic capability, not a new product feature.
- [x] Use deterministic dependency responses for simulator tests and a bounded live-OSRM Compose run to prove all three profiles and the upload-to-Raw-Map demonstration.
- [x] Retain the basic fixture/scenario and recorded versions so later processing slices can reuse it; infrastructure does not depend on the processed publication work.

## Answer

Implemented and verified. See [operator commands and acceptance evidence](../../../docs/development/phase-2-sparse-and-osrm-acceptance.md).

## Comments

2026-10-06: Completed via TDD at the approved HTTP/worker/PostGIS, Web and simulator HTTP seams. Full validation and bounded live OSRM car/bike/foot + upload/replay + built-Web acceptance passed. Independent Standards and Spec reviews completed; duplicate Batch writing was shared and the operator enqueue/publication race was fixed with queued-work generation and a deterministic regression. Dense activity stays deferred in this sparse slice.
