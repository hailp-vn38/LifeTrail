# Phase 2 Web/Server Map Performance — firmware-adaptive data

Scenario: **05/10/2026 00:00 → 06/10/2026 00:20**, `Asia/Ho_Chi_Minh`.
Synthetic HCMC-region coordinates, generated offline; these polylines are not
road-matched tracks. See [scenario](../../scenarios/phase2-web-server-map-performance.json)
and [measured counts / activity times](summary.json).

| Archive | GPS Records | Batches | Purpose |
| --- | ---: | ---: | --- |
| `firmware.zip` | 3,268 | 342 | Current compiled firmware persistence policy |
| `dense.zip` | 87,000 | 1,450 | Legacy 1 Hz comparator on the same observations |

Both contain uploadable `gps/1` `.ndjson.ready` / `.manifest` pairs with
deterministic UUIDv4 identities, SHA-256, record counts and LF framing. Each
firmware record is an unchanged source acquisition observation, selected by the
real C policy through `gps_policy_filter`; no averaged coordinates or firmware
motion metadata are uploaded. Firmware batching uses the existing 60-second
moving/candidate and 300-second stationary adapter. This fixture exercises the
policy and batch format, not SD latency, RAM overflow or power-loss recovery.
The summary records the firmware policy source fingerprint and persist reasons.

The 96.24% reduction is **Raw record count**, not a measured Daily JSON or map
render improvement. Measure display simplification against canonical playback
separately; do not require the same payload reduction from sparse and dense data.

## Generate and verify

From the repository root, with Python 3, CMake and a C compiler:

```sh
PYTHONPATH=tools python3 tools/generate_map_performance_data.py
PYTHONPATH=tools python3 -m unittest tools/tests/test_map_performance_data.py
```

Regeneration invokes the current firmware policy; counts can change when that
policy changes. Identical inputs and policy produce byte-identical archives.
Batch IDs include their content, so a changed policy never reuses an ID for a
different body. `LT_GPS_POLICY_FILTER`, if set, selects an external policy binary;
leave it unset to compile the repository firmware represented by the fingerprint.

## Load into Daily

Use two new Devices with the Owner timezone above: **Map Performance FW** and
**Map Performance Dense**. Upload each archive only to its corresponding Device;
merging them would give the firmware Device dense evidence and defeat comparison.
The generator only creates files; it does not provision Devices or upload data.

The server must run the new Phase 2 code and migration 16. Historical v1
publications need `lifetrail-server reproject-schema`; see the rollout in
[the performance spec](../../../docs/web/lifetrail-phase2-web-server-map-performance.md).

Example for the first firmware import, after provisioning its Device token:

```sh
unzip -q tools/fixtures/phase2-web-server-map-performance/firmware.zip -d runtime/map-performance-fw
for manifest in runtime/map-performance-fw/*.manifest; do
  PYTHONPATH=tools python3 tools/batch_acceptance.py commit-and-replay \
    --body "${manifest%.manifest}.ndjson.ready" --manifest "$manifest" \
    --endpoint http://localhost:8081/api/v1/device/batches \
    --token "$LT_MAP_PERF_FW_TOKEN" || break
done
```

For an already imported archive use `replay` instead of `commit-and-replay`.
For the dense archive use a separate extraction directory and its Device token.
After ingestion, wait for processing to publish both local days, then open
`/devices/<device-id>/day/2026-10-05` and `/devices/<device-id>/day/2026-10-06`.

## Checks on the processed API and web

The scenario defines physical evidence, not exact server Trip IDs or transport
classifications. The server remains responsible for activity inference.

| Evidence / action | Check |
| --- | --- |
| Overnight and office dwell | 120-second firmware heartbeats should not become GPS Gaps with the default 300-second threshold |
| Walks and drives with turns | Daily renders `display_geometry`, fits bounds and shows Stop dots before Play |
| 06:50:00–06:50:20 traffic pause | Below the default 180-second Stop threshold; must not create a qualifying Stop |
| 17:30–17:40 GNSS loss | True GPS Gap; no line or playback interpolation across it |
| 23:55–00:10 drive | Cross-day clipping, display endpoints and canonical progress remain consistent |
| Final dwell through 00:19:59 | Ends without later departure evidence; preserve open-boundary semantics |
| Normal Daily fetch | `projection_schema_version == 2`; no canonical arrays or `source_record_ids` |
| Initial map load | No playback request; display route and activity locations visible |
| First Play / export | Fetch canonical playback pinned to the displayed manifest |
| Sparse vs dense Device | Compare Stop/Trip/Gap boundaries, canonical distance, display vertex count, JSON bytes and render timing |
| Raw GPS mode | Each Device retains its own original accepted GPS Records |

This fixture does not force an Evidence Hole: firmware can reject poor-quality
epochs before persistence, so a server Evidence Hole must be tested with
explicit persisted low-quality evidence, such as the existing
[Phase 2 master suite](../../lifetrail-phase2-testdata/README.md).
