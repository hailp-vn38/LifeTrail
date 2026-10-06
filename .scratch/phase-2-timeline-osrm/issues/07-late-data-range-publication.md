# 07: Refresh late data with atomic semantic-range publication

Status: resolved
Type: task
Labels: ready-for-agent
Blocked by: 06

**What to build:** The Owner keeps a useful processed view while late data or explicit reprocessing revises continuous activity. The worker expands to safe boundaries, replaces the manifest range and activates every affected day together; old history remains readable.

**Blocked by:** 06 — Project continuous activity across midnight.

## Acceptance criteria

- [ ] Merge a newly committed Batch into durable dirty bounds and mark affected published projections stale without replacing them with Raw View.
- [ ] Expand effective processing/replacement range until outside activity is unchanged and endpoints independently pass semantic safety, including verified observation/Evidence Hole edges.
- [ ] A validated chunk seam or arbitrary processing-window edge alone never authorizes cutting a Trip/Stop. A requested replacement inside a continuous activity expands to its semantic-safe boundaries.
- [ ] Create immutable range-owned Activity Revisions declaring superseded coverage; preserve unchanged immutable sources outside replacement rather than copying all Device history.
- [ ] Splice the active versioned manifest into ordered non-overlapping half-open safe revision slices. Readers never infer precedence from latest revision IDs.
- [ ] Build replacement daily snapshots from pinned manifest/source versions. Activate activity/manifest and every affected day atomically through generation/target/fencing checks in a short transaction.
- [ ] Generation or target mismatch rejects publication and requeues current work; failed refresh retains the previous complete snapshot with stale/failed status.
- [ ] Web polls lightweight status every five seconds while queued/running and visible, stops on idle/failed/hidden, and immediately refreshes status on restored focus/visibility.
- [ ] A new publication triggers one complete Daily View fetch and atomic UI replacement, clears selection and pauses/resets playback; unchanged publication does not redownload geometry.
- [ ] Provide an operator reprocessing/backfill command using the same revision/publication flow, without Raw mutation or synchronous OSRM on reads. IDs may change across revisions without reconciliation.
- [ ] Acceptance covers overlapping range splices, late cross-midnight activity, concurrent reads, failure/rejection and revision-local identities; retained old snapshots/manifests/revisions/candidates remain readable with no automatic GC.

## Answer

Implemented durable dirty-range tracking and immutable semantic-range publication. Each accepted Batch coalesces its UTC dirty bounds while preserving the last published Daily Snapshot. Activity Revisions declare superseded coverage; an explicit active-manifest pointer is activated atomically with all Daily Snapshot pointers and is composed through ordered, non-overlapping half-open slices rather than revision recency. The operator `process-day` flow continues to use the same fenced worker/publication path.

The Web now polls the lightweight status endpoint every five seconds only for visible queued/running work, refreshes status on focus/visibility restoration, and refetches the complete Daily View only when the published revision changes; the existing revision-keyed map remount clears selection and resets playback atomically.
