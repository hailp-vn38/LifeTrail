# Align Phase 2 with processed GPS geometry

Status: resolved
Type: task
Date: 2026-10-06

Implement [the post-implementation alignment guide](../../../docs/lifetrail-phase2-post-implementation-alignment.md). Remove routing dependencies from the runtime and worker while preserving immutable Raw GPS, activity semantics, progress, revision manifests and atomic publication.

## Changes

- Remove matcher client, HTTP dependency, routing Compose services/profiles and application commands.
- Retire committed matcher schema with additive migration 0015; preserve historical immutable publications and queue replacement processing.
- Publish processed GPS for every transport mode and keep millisecond epochs. Resolve exact timestamp duplicates deterministically for geometry only.
- Persist bounded replacement revisions and compose Daily Views from authoritative manifest slices.
- Preserve geometry/distance across mode boundaries, and use one millisecond-precise Gap policy for Stops and Trips.
- Keep source quality counts and reducer provenance during timezone-only projection using generic processing audit.
- Refresh the initial Raw view on the first processed publication; skip status refresh for hidden-page events.
- Update API/generated types, current spec, CONTEXT, ADRs, deployment/tools docs and fixture matrix. Defer historical routing tools and fixtures explicitly.

## Validation

See [the acceptance report](../../../docs/development/phase-2-processed-gps-alignment-acceptance.md) for final results and scale measurements.
