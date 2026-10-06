# MEMORY — implement-spec: Phase 2 ticket 05 CLOSED

Last updated: 2026-10-06 (ticket 05 closed; integration branch tip `505666b`)

## What was asked

`@implement-spec thực hiện ticket "05: Distinguish quality failures, GPS Gaps and Evidence Holes"`, then `đọc MEMORY.md, tiếp tục công việc trước` — which finished the four remaining items below.

## Repo / conventions

- Repo: `/home/hailp/LifeTrail` (Rust/Axum server + Vue web + PostGIS). Issue tracker = local Markdown in `.scratch/`.
- `AGENTS.md`: modularise by responsibility, keep files short; issues in `.scratch/`; domain docs per `docs/agents/domain.md`.
- Domain vocabulary in `CONTEXT.md` (GPS Record, Raw GPS, Trip, Stop, GPS Gap, Evidence Hole, Route Part). Use exact terms.
- Integration branch for the Phase 2 spec: **`feat/phase-2-timeline-osrm`**.

## Status: ticket 05 is DONE and resolved

- `9e19a33` — merge of the original ticket 05 work (work commit `4caef4b`).
- `481a30a` — merge of the code-review fixes (work commit `af1c866`, branch `fix/ticket-05-review-fixes`, now deleted).
- `505666b` — ticket file closed: `Status: resolved`, ticked criteria, `## Answer` + `## Comments`.
- Acceptance notes: `docs/development/phase-2-quality-gaps-acceptance.md`.
- Implementer worktree `/tmp/opencode/lifetrail-05` and both feature branches are removed.

### What the review found and fixed

Two real defects (both independently mutation-verified after the fix):

1. **Three-way classification published as two ways.** `excluded_point_count` was `point_count - usable_count`, so low-quality records were reported as *impossible*. Now `snapshot.rs` publishes `low_quality_point_count` and the real `Excluded` count; the three always partition `point_count`. New test `impossible_jump_and_poor_quality_publish_distinct_classes`. Mutation (restore the subtraction) fails 4 tests.
2. **"No DOM marker per GPS Record" was vacuous.** `RouteMap.test.ts` compared only source/layer counts and the processed fixture has no Raw Records. Now it counts GeoJSON features per source (`featureCounts`) and pins the marker sources to published Route Parts / Stops. Mutation (one Point per record) fails 3 tests.

Standards fixes: single `gaps::absent` shared with `holes` (was duplicated verbatim); `evidence.rs` → `holes.rs`; `Target` owns `quality::Policy` via `#[sqlx(flatten)]` (works in sqlx 0.8) and `Policy::from_target` is gone; `Observation::usable()` middle man deleted; class-to-reason vocabulary moved to `quality::hole_reason`; `evidence.ts` uses the generated `EvidenceHole` type with an exhaustive `Record<EvidenceHole["reason"], string>`; `unsupported_classification` dropped from the OpenAPI enum (nothing could emit it); `quality_gap_processing.rs` (541 lines) split into `tests/quality_classification.rs` (269) + `tests/gap_evidence.rs` (280) with the harness in `tests/support/scenarios.rs`, and `at()` deduplicated out of `trip_processing.rs`.

Deliberately not fixed, documented in the ticket Comments: `TARGET_COLUMNS` as an interpolated `&str` (pre-existing pattern, no ticket behind a refactor); the Trip-ending-at-a-Gap boundary staying open (ADR-0001 needs evidence on both sides; the criterion permits open boundaries).

### Verification on `505666b` (re-run by me, not just the sub-agent)

- `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` clean.
- `LT_TEST_DATABASE_URL=… cargo test --all-targets -- --include-ignored --test-threads=1` → **26 passed, 0 failed** across 14 binaries. (MEMORY's old "28" was wrong; 24 before the review, 26 after.)
- `npm run typecheck` clean; `npm test` → **167 passed / 24 files**; `npm run build` OK.
- `PYTHONPATH=tools python3 -m unittest discover -s tools/tests` → 8 passed.

### Environment notes for re-running tests

- Disposable PostGIS `lt-ticket05-db` on `127.0.0.1:55433`, URL `postgres://lifetrail:lifetrail_dev_only@127.0.0.1:55433/lifetrail_test`. Restart with:
  `docker run -d --name lt-ticket05-db -e POSTGRES_DB=lifetrail_test -e POSTGRES_USER=lifetrail -e POSTGRES_PASSWORD=lifetrail_dev_only -p 55433:5432 postgis/postgis:17-3.5`
- Integration tests are `#[ignore = "requires PostgreSQL/PostGIS"]` → run with `--include-ignored` and `--test-threads=1` (they `TRUNCATE users CASCADE`).
- `cargo` is not on `PATH` in a bare shell: `export PATH="$HOME/.cargo/bin:$PATH"`. Use `CARGO_TARGET_DIR=/home/hailp/LifeTrail/server/target` to reuse the cache.
- No JSON Schema validator / PyYAML in this environment and no validation script in `tools/`, so any OpenAPI contract check must parse the YAML as text. The older acceptance note's claim of full OpenAPI 3.1 validation is not reproducible here.

## Frontier of the Phase 2 task graph

Next ready tickets: **06** (cross-midnight daily projection), **07** (late data range publication), **08** (worker recovery/fencing/coalescing), **09** (timezone stale-source reprojection), **11** (short-trace OSRM match), **12** (chunked hybrid matching), **14** (realistic OSRM scenarios), **15** (full-system acceptance benchmark). Tickets 10 and 13 were blocked by 05 and may now be unblocked — re-scan `.scratch/phase-2-timeline-osrm/issues/` before picking the next one.

Workflow that worked: code-review skill with fixed point `2c972dd` → aggregate both axes → one `general` implementer sub-agent with an explicit numbered fix list and an explicit "do NOT fix" list → verify independently (including re-running the mutations yourself) → merge with `--no-ff` → close the ticket file → update this file.