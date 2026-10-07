# Firmware GPS persistence is stateful and preserves Raw GPS provenance

Firmware continues to acquire Navigation Epochs at 1 Hz, but a dedicated persistence policy accepts a sparse subset of real GPS Records in MOVING, CANDIDATE_STOP, and STATIONARY states. This separates parsing from persistence, keeps Trip and Stop semantics on the server, and never creates averaged, interpolated, or repeated Raw GPS coordinates. All motion-state transitions persist immediately; a transition into MOVING backfills up to five seconds of real observations so a Trip does not begin late.

Stationary Devices persist a real-observation heartbeat every 120 seconds while a fix exists, which leaves margin below the server's 300-second GPS-gap threshold even if one heartbeat is lost. A loss of GPS fix persists nothing and remains a real server-visible gap. The policy deliberately trades up to the fsync cadence's newest data for fewer SD writes; storage must not block GPS acquisition when it is slow.

## Consequences

The storage boundary receives a motion state so batches rotate at 60 seconds while MOVING or CANDIDATE_STOP and 300 seconds while STATIONARY. Firmware records persistence reasons for tests and telemetry, while server-side quality classification, GPS Gap, Evidence Hole, Trip, and Stop remain authoritative.
