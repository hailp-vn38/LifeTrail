# Firmware GPS persistence is stateful and preserves Raw GPS provenance

Firmware continues to acquire Navigation Epochs at 1 Hz, but a dedicated persistence policy accepts a sparse subset of real GPS Records in MOVING, CANDIDATE_STOP, and STATIONARY states. This separates parsing from persistence, keeps Trip and Stop semantics on the server, and never creates averaged, interpolated, or repeated Raw GPS coordinates. All motion-state transitions persist immediately; a transition into MOVING backfills up to five seconds of real observations so a Trip does not begin late.

Stationary Devices persist a real-observation heartbeat every 120 seconds while a fix exists, which leaves margin below the server's 300-second GPS-gap threshold even if one heartbeat is lost. A loss of GPS fix persists nothing and remains a real server-visible gap. The policy deliberately trades up to the fsync cadence's newest data for fewer SD writes; storage must not block GPS acquisition when it is slow.

## Consequences

The storage boundary receives a motion state so batches rotate at 60 seconds while MOVING or CANDIDATE_STOP and 300 seconds while STATIONARY. Firmware records persistence reasons for tests and telemetry, while server-side quality classification, GPS Gap, Evidence Hole, Trip, and Stop remain authoritative.

The firmware composition lives in `lifetrail_recorder`: the GPS task owns the collector and policy, and the storage task exclusively owns the Batch writer and open SD file. A bounded 256-entry RAM queue drops the oldest waiting record on overflow; its short critical sections never include file I/O. The overflow counter is exposed in `lt_recorder_health()` and warnings are limited to once per minute. Storage startup failure leaves acquisition running while the storage task retries mount/recovery.

The data file stays open with a 4 KiB stdio buffer. It flushes every 5 seconds and fsyncs every 15 seconds when dirty; rotation, explicit shutdown, and the controlled-restart hook force durability. FatFS immediate fsync is disabled and long filenames are enabled for UUID Batch names. SD counters include manifest durability operations. Sudden power loss may discard the unsynced tail, while existing recovery still verifies complete records and quarantines corruption.

Graceful shutdown persists the latest real non-jump observation once. At fix recovery, the policy first preserves an unpersisted observation from immediately before the gap, then the first real observation after it. Both retain their original GPS timestamps; the gap contains no invented records. These boundary observations keep Stop endpoints within the acceptance tolerance and prevent a delayed stationary heartbeat from shifting the visible end of a real GPS Gap.

Shutdown preserves that observation even if GPS fix has already been lost; observation age never changes its original timestamp. After storage delay, queued observations rotate Batches by their own timestamps; wall-clock idle rotation runs only after the queue drains. GPS and storage tasks register for a configurable progress watchdog (default 30 seconds, panic/restart), reset it between bounded loop iterations, and deregister on graceful exit. SD operations exceeding that deadline trigger recovery by restart rather than silently leaving a stalled task.

After confirmed movement, candidate-stop detection requires a fresh complete detection window in MOVING. Evidence from the preceding Stop cannot immediately switch slow walking back into CANDIDATE_STOP. Named policy limits define the 30-valid-epoch stationary center and three-epoch/two-epoch movement evidence requirements; jump observations are excluded from the center.

Phase 2 acceptance confirms that `short_failure_max_s = 10` limits bridging of **rejected observations**, rather than gaps between usable records. Valid 30-second candidate records and 120-second stationary heartbeats therefore need neither server policy changes nor uploaded firmware motion metadata. Runtime state averaging is restricted to the stationary center and never enters Raw GPS.
