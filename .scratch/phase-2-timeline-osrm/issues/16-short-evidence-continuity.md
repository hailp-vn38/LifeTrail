# 16: Preserve activity across short rejected GPS observations

Status: resolved
Type: task

## Problem

The master fixture at 18:35 alternates usable HDOP 3.5/4.5 with rejected
HDOP 5.5/6.5. Per-record movement boundaries generate many tiny Trips and
four-record Evidence Holes. Web renders unknown historical reason codes as
`undefined`.

## Policy

- Retain immutable Raw GPS and its original quality classes/counts.
- Bridge a rejected run only between usable observations, with positive elapsed
  time no greater than configurable `short_failure_max_s` (default 10 seconds),
  no true Raw GPS Gap, and endpoint displacement within the configured maximum
  implied speed. Zero disables bridging.
- Exclude bridged records from dwell geometry, mode evidence, Route Parts and
  OSRM inputs; preserve original source counts/IDs in activity metadata.
- Disclose a hole unless both the bridge evidence and enclosing derived activity
  support the interval. Long failures and unbounded observation edges remain
  Evidence Holes with open activity boundaries.
- Preserve true GPS Gaps, matcher fallback and UNKNOWN mode behavior.
- Retain policy/algorithm/reducer provenance; a policy change retires the
  current publication until explicit reprocessing, without deleting snapshots.
- Display a neutral fallback for reason codes outside the current API enum.

## Validation

Public ingestion-to-publication tests reproduce alternating HDOP, an isolated
one-second jump, the actual master 18:35 fixture, a stationary dwell, sustained
quality failures, unbounded edges and disabling the policy. Existing minute-scale
jump and sustained quality tests continue to require Evidence Holes.

## Related correction

Revision-size measurement now casts PostgreSQL INT4 to BIGINT before decoding
into Rust i64; otherwise a fresh processing session fails before publication.

## Answer

Implemented bounded evidence continuity in its own processing module, captured
policy and migration 0014, reducer version 5, usable-only matcher selection and
safe historical reason labels. Updated spec/fixture documentation and repaired
the stale DailyMapPage query mock.

Validation: all 14 real-PostGIS quality/gap/short-evidence tests pass, including
the full 300-record 18:35 master Batch plus its next usable observation. All
173 Web tests, Web typecheck/build, nine Rust unit tests and formatting pass.

Broader checks retain a pre-existing repeated-visit Route Part count failure
(`2` versus `3` source records), reproduced against the prior code in an isolated
baseline crate. Strict Clippy also reports existing obfuscated-if-else,
unnecessary-lazy-evaluation and too-many-arguments findings in capture/matcher;
all-target Clippy passes when those existing categories are allowed.

Existing deployments require the new server/Web build and migration, then an
explicit activity refresh; immutable historical snapshots are not rewritten.
