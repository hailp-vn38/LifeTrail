# Separate daily display projection from canonical playback projection

Accepted on 2026-10-07. The canonical processed geometry and temporal progress of
an Activity Revision (ADR-0004, ADR-0007) stay authoritative and are no longer
published as the default Daily View payload. Daily View instead publishes a
display-oriented Route Part projection: server-simplified geometry with rounded
coordinates, the tolerance actually used and an over-budget flag, plus
`distance_m` and `visible_distance_m` derived from the canonical geometry. The
display geometry is a visualization artifact only; it is never the source of
distance, duration or progress. Distance and time authority remain with the
canonical Activity Revision and are never recomputed from display geometry.

Canonical geometry and progress are exposed on demand through a dedicated
Playback API rather than being embedded in the Daily View response. Consumers
that genuinely need canonical data — playback, GPX and route CSV export, video
export — fetch it lazily from the playback endpoint. Web never silently falls
back to display geometry when canonical data is required; a failed playback
fetch is an error, not a downgrade.

This keeps the initial Daily Map load lightweight: heavy canonical payloads are
no longer part of it, and clients pay for canonical data only when they ask for
it. The trade-off is an extra request and an explicit separation between two
representations of the same Activity Revision, which is why the representations
are named and kept distinct rather than merged into one shape.

Daily Snapshots are immutable (ADR-0005). Moving to projection schema v2
therefore does not rewrite or delete existing snapshot bodies; historical days
are reprojected by creating new v2 snapshots and atomically switching
`daily_publications` to them. The migration acceptance is that no publication
points at a v1 snapshot, not that no v1 row remains. Projection-only reprojection
uses its own requeue seam and does not advance Raw input generation; it is
distinct from the timezone reprojection seam of ADR-0006.

Playback resolves canonical data against an immutable Activity Manifest. A
request may pin a manifest version: an existing manifest returns the pinned
payload, a missing manifest returns `410 Gone`, and an unpinned request returns
the current publication. The server does not return `409` for unpinned playback
because it cannot conclude that a publication changed relative to a client it
never pinned; the Daily View / playback manifest mismatch is a client-side
consistency concern, not an HTTP status on the server.
