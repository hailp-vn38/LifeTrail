# Timezone reprojection may publish stale activity sources

Accepted on 2026-10-06. When Owner timezone changes, projection-only work may build a new-timezone Daily Snapshot from the latest published activity even while newer Raw input awaits processing. The snapshot records source manifest/generation, processed-through generation, timezone generation and projection provenance, distinguishing activity freshness from projection freshness. It must disclose STALE_SOURCE rather than claim the newer Raw input was processed.

Projection-only activation verifies the expected active manifest version, current timezone generation and valid job fencing authority; it never modifies activity publication state. If either source manifest or timezone changes while the job runs, activation is rejected and current projection work is queued. Activity publication later replaces affected projections atomically under the activity generation/target/fencing protocol.

Waiting for all Raw refresh work to finish before showing a processed view in the new timezone was rejected because the last published activity remains useful. The separate freshness axes and activation conditions preserve that availability without mislabeling source data or permitting stale writes.
