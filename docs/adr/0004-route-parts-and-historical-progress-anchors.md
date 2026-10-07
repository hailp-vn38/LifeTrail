# Route Parts with historical progress anchors

Phase 2 alignment: matcher-specific policies superseded by [ADR-0007](0007-phase2-processed-gps.md). The text below records the earlier decision.


Accepted on 2026-10-06. Ordered Route Parts are the canonical processed geometry and playback contract. A Movement Segment can contain several contiguous parts after partial or split matching. Each part uses historical GPS timestamps mapped to non-decreasing distance along its geometry, with strictly increasing anchor times and anchors at both ends. Daily clipping uses the same mapping and creates synthetic anchors at calendar boundaries.

A daily MultiLineString may be derived for overview rendering but is not the playback contract. One timestamp per OSRM vertex and one part per Movement Segment were rejected because matching changes vertex count and may split geometry. Anchors preserve historical timing and deterministic seeking without fabricating GPS observations or interpolating through disconnected parts.

Round 3 confirmed on 2026-10-06: a matched Route Part is publishable only with valid geometry, known observed temporal coverage and valid monotonic anchors. Reject otherwise plausible geometry that cannot map historical timing. Safe matched/raw hybrid parts may share one Movement Segment; when partial temporal boundaries cannot be trusted, use raw fallback for the entire segment. Distance sums only published Route Parts, excluding rejected geometry and invented connectors. A matcher failure or split never creates a GPS Gap.

Round 4 confirmed on 2026-10-06: the server publishes cumulative vertex_distance_m and owns the progress metric. Web uses it rather than recomputing route lengths. Daily clipping takes differences in the same anchored progress measure, conserving distance across adjacent days. Same-second input selection preserves source-record mapping and original timestamps; chunk overlap seams use shared source/time/progress evidence with deterministic ordering. Unsafe seams apply the safe-boundary fallback policy, never invented connectors.

Phase 2 alignment: [ADR-0008](0008-daily-display-projection-vs-canonical-playback.md) keeps the canonical Route Part geometry and progress authoritative but no longer publishes them as the default Daily View payload. Daily View publishes a display-oriented projection; canonical geometry and progress are served on demand through the Playback API. The server-owned distance/progress metric and anchored clipping above remain the authority and are never derived from display geometry.
