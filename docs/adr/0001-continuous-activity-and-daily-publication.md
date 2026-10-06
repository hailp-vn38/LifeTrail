# Continuous activity with published daily projections

Accepted on 2026-10-06. Trip, Stop and GPS Gap belong to a Device's continuous activity history rather than one calendar day. A Trip is a maximal continuous chain terminated by a qualifying Stop or GPS Gap and contains 1..N ordered Movement Segments; mode changes and short pauses do not themselves terminate it. GPS Gaps represent missing evidence and must not acquire inferred routes, movement or stationary time.

Daily View projects these events onto an Owner-local calendar day, preserving full event intervals and identities while exposing visible intervals and counting only daily overlap. Processed routes therefore support disconnected geometry. This replaces the proposed one-Trip-per-segment and day-owned activity schema, which cannot represent multimode Trips or cross-midnight Stops correctly.

Daily publication and background processing have separate states. Refresh retains the last successful snapshot and replaces it atomically only after success; failure leaves that publication available and stale. This costs revision storage and publication coordination but avoids partial reads and losing a useful processed view whenever late GPS arrives.

Round 2 confirmed on 2026-10-06: expand processing to stable segmentation boundaries, rather than a fixed context window, and preserve open Trip/Stop boundaries at observation edges. Candidate publication verifies the captured Device input generation and atomically activates all affected daily projections; details are recorded in ADR-0003.

Round 3 confirmed on 2026-10-06: actual activity boundaries are nullable when open, while observed_from_at/observed_until_at describe supported coverage. Full duration is unknown when either actual boundary is open; daily projections never extrapolate observations. GPS Gaps require absence of Raw observations between observed boundaries. Existing but unusable observations form Evidence Holes, which are disclosed separately. Revision-local event identity was chosen instead of merge/split reconciliation across revisions; Web clears selection and resets playback when publication changes.
