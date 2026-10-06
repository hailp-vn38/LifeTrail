# Tickets 01–02 review

Fixed point: `7fd8ef6` (before implementation). Initial reviewed commit: `d31c90b`. Commands: `git diff 7fd8ef6...HEAD`, `git log 7fd8ef6..HEAD --oneline`. Two independent subagents performed the Standards and Spec axes required by the code-review skill, then rechecked their respective fixes.

## Standards

No documented standard breaches. Modules separate claim/capture/activation/read and routing/sampling/writing responsibilities. One heuristic finding, **Possible Duplicated Code**: encoding and writing ready bodies/manifests repeated the older simulator. Resolved by sharing `GeneratedBatch`, `encode_records` and `write_batch` in `batch_writer.py`, preserving both UUID policies and body bytes. Focused simulator tests passed; follow-up review found no remaining findings.

## Spec

One P1 finding against spec §29: “work arriving during a running job must not be lost.” An operator request arriving after capture could remain queued while the old worker set its job idle. A separate monotonic `work_generation` now advances with each enqueue under the Device control lock. Capture pins it; changed generation rejects activation and requeues without acknowledging days. The deterministic integration regression first failed with `idle` instead of `queued`, then passed after the fix, covering both captured-day and new-day requests. Follow-up review found no remaining concerns. No other actionable deviations within tickets 01–02; later activity and polling slices excluded.

Initial findings: Standards 0 hard / 1 heuristic; Spec 1 (lost operator work). Remaining after fixes: Standards 0; Spec 0.
