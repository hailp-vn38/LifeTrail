# Immutable range revisions composed into daily snapshots

Accepted on 2026-10-06. An Activity Revision owns the derived output of one continuous processing range and explicitly declares its superseded range. Replacement cuts follow stable processing boundaries rather than calendar days. Activity revisions are immutable after successful creation; published Daily Snapshots are immutable and reuse unchanged activity versions outside the replacement range.

Activity revision identity describes where an event/segment/part was created. Daily published revision identity describes the activated Daily View snapshot; the two IDs are distinct. This avoids copying full Device history on each late Batch while retaining cross-midnight event identity, auditability and consistent multi-day activation.

Evidence Holes are separate read-model intervals, not mandatory Timeline events. They can leave activities open on both sides inside otherwise observed history. This avoids falsely asserting continuity or manufacturing Stop/Gap events where Raw observations exist but reliable activity cannot be established.

Round 5 confirmed on 2026-10-06: immutable versioned Activity Manifests are the sole authority for revision range membership. Ordered non-overlapping half-open slices are replaced by explicit splice operations; readers never infer precedence from revision recency. Daily Snapshots pin manifest versions and source provenance. Existing revisions can supply unchanged safe slices outside replacement ranges. Replacement endpoints require machine-checkable semantic safety.

Phase 2 does not automatically delete revisions, manifests, snapshots, matcher candidates/evidence or rejected/superseded candidates. Historical references remain readable during normal processing. Storage growth is measured; future GC must be reference-aware reachability rather than simple age-based deletion. This retains auditability and avoids dangling historical snapshots at the cost of storage growth.

Q31 confirmed on 2026-10-06: chunk seams are technical RoutePart assembly boundaries, not ActivityRevision boundaries. A validated seam alone never authorizes replacement inside continuous activity; the endpoint must independently be a semantic activity boundary, verified Evidence Hole boundary or verified observation edge. Otherwise expand/reprocess the replacement range. A processing-window edge also requires independent semantic safety. A seam coinciding with a Trip boundary qualifies because of that Trip boundary, not the seam.
