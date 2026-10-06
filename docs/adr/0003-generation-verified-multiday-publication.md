# Generation-verified publication of affected daily projections

Accepted on 2026-10-06. Each Device has a monotonic input generation. A worker captures it and constructs candidate activity and projection rows outside the publication transaction. A short activation transaction verifies the generation and atomically swaps the activity and all affected daily projection pointers. If the generation changed, candidate publication is discarded, the last good snapshots remain available and processing is queued again.

Publishing each affected day independently was rejected because one cross-midnight activity could acquire inconsistent representations in adjacent Daily Views. Holding a transaction open throughout OSRM processing was also rejected; dependency latency belongs outside activation. This design requires staging storage and coordinated activation but preserves consistent publication without a long-running database transaction.

UTC activity is independent of Owner timezone. Projection identity includes timezone and activity revision, or an equivalent timezone generation. A timezone change rebuilds projections only and never relabels old snapshots or repeats GPS/OSRM processing. If the requested current-timezone projection is missing, Raw Daily View with projection status is the fallback.

Round 3 confirmed on 2026-10-06: input_generation advances exactly once for a newly committed valid Batch, including late, overlapping and poor-quality data. Replay, conflict, authentication/validation failure and rollback do not advance it. Processing version, configuration, matcher engine/dataset and timezone have separate target identities; changes to them do not mutate Raw input generation. Publication verifies the captured processing/projection targets as well as input_generation.

Round 4 confirmed on 2026-10-06: PostgreSQL stores coalesced per-Device jobs. Worker claims use a reclaimable lease and fencing token; stale workers cannot publish after authority changes. Ingestion, target updates and publication serialize via the same Device control row in short transactions, with a consistent input snapshot and no OSRM calls inside database transactions. Candidate publication also checks fencing authority and is idempotent after crash/retry. This durable protocol was chosen over process-local locking because a restarted or reclaimed worker must not overwrite newer results.

Phase 2 alignment: [ADR-0007](0007-phase2-processed-gps.md) defers routing and matcher-specific policy; the publication and revision decisions above remain in force.
