# Ticket 03 review

Fixed point: `9dc684f6d0c334b9c2e319f6ee82485c9f5be1f9` (before this implementation). Commands: `git diff 9dc684f6d0c334b9c2e319f6ee82485c9f5be1f9...HEAD`, `git log 9dc684f6d0c334b9c2e319f6ee82485c9f5be1f9..HEAD --oneline`. Initial implementation: `2e49605`; reviewed fixes: `be28888`. Two independent subagents reviewed the Standards and Spec axes, then rechecked their respective fixes.

## Standards

No documented standards violations. One low-priority heuristic, **Possible Mysterious Name**: internal collections named `holes` also held missing observations and unresolved activity. Renamed to `unresolved_intervals` / `intervals` to preserve the domain distinction. Public daily fields separately expose genuine Evidence Holes and unresolved activity/absence intervals. Follow-up found no remaining documented violations or actionable smells. The Raw-view fix reuses the existing projector, Vue Query cache and centralized query-key factory, with local UI mode state.

## Spec

One P1 finding against ticket 03: “the established empty/Raw views remaining usable.” Publishing every dense day with no movement geometry initially hid the existing Raw map and Start/End Timeline. Added the public `?view=raw` option through the original Raw projector, generated API contract, distinct query identity and Daily Map Raw GPS/activity toggle. Map remounting includes representation, clearing selection/playback on a switch. The real ingestion/worker/API regression first failed (`processed` instead of `raw`), then passed, including a moving zero-speed trace with no qualifying Stop. API-client/component tests exercise the query and selector. Follow-up found no remaining Spec findings within ticket 03; broader movement/Gap/matching, incremental splice/reuse and projection-only timezone work remain later tickets.

Initial findings: Standards 0 hard / 1 heuristic; Spec 1 (Raw-view regression). Remaining after fixes: Standards 0; Spec 0.
