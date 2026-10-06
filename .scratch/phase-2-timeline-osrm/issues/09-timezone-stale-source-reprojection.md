# 09: Reproject published history after timezone changes

Status: ready-for-agent
Type: task
Labels: ready-for-agent
Blocked by: 07

**What to build:** The Owner can change timezone and obtain correctly labeled daily projections of the latest published activity, even while newer Raw data is pending. Stale-source freshness is explicit and outdated projection jobs cannot overwrite the new context.

**Blocked by:** 07 — Refresh late data with atomic semantic-range publication.

## Acceptance criteria

- [ ] Current Owner IANA timezone defines the daily projection. Timezone updates have their own generation and do not advance input_generation.
- [ ] Rebuild projection/summary/clipping only; do not rerun GPS quality, activity segmentation, mode classification or OSRM.
- [ ] If a current-timezone processed snapshot is not yet available, Raw Daily View plus projection status is valid fallback; never relabel an old-timezone snapshot.
- [ ] A projection of published activity G142 under T6 may activate while Raw G143 is pending, with truthful source manifest/generation/processed-through metadata and STALE_SOURCE.
- [ ] Distinguish activity/source freshness from projection freshness; new snapshot identity does not imply all reused sources were recomputed.
- [ ] Projection-only activation verifies expected active manifest, current Owner timezone generation and valid job fencing. It never changes activity publication state.
- [ ] If M17 becomes M18 or T6 becomes T7 during projection work, reject candidate activation and queue the current combination; new activity publication later replaces affected projections atomically.
- [ ] Web query identity includes projection/timezone context and prevents old responses from replacing the current view; selection/playback reset on a new publication.
- [ ] Integration/Web tests exercise stale-source acceptance, both manifest/timezone races, current-timezone fallback, no OSRM calls, source provenance and retained prior history.
