# Focused realistic OSRM variants

Use the same generation command and routing-version evidence as the main day. These
scenario fragments deliberately describe only Raw inputs/observation conditions;
the expected processed assertions belong to the Phase 2 test-data oracle.

| Variant | Activity/degradation change | Processing assertion location |
| --- | --- | --- |
| Mode transfer | Adjacent `foot` then `car` moves with no intervening `stop` activity | `master/oracle.json` `multimodal_trip` |
| Cross-midnight | Start a movement or long stationary activity at `23:50` and run through `00:20` | `master/oracle.json` `cross_midnight_stop` |
| Interior Evidence Hole | Add `{ "kind": "poor_quality", "from_s": 300, "until_s": 600 }` to a movement | `master/oracle.json` `evidence_hole` |
| Late upload | Generate the omitted middle time interval into a separate output directory and upload it after the base directory | `master/stage-02-late-midnight-gap-fill/` |
| Degraded route processing | Reuse generated Raw Batches with frozen matcher failure evidence; do not alter Raw coordinates to encode an outage | `matcher-fixtures/07-transient-timeout.json`, `08-no-match.json` |

The committed master suite contains the uploadable Batches/manifests for these
variants. This page keeps the simulator boundary explicit: missing observations,
poor observations and matcher dependency outcomes are separate facts.
