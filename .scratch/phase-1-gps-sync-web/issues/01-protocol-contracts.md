# 01 — Initialize Git, monorepo skeleton, and protocol contracts

Status: resolved
Type: task

## Goal

Initialize the LifeTrail repository and its complete Phase 1 monorepo skeleton, then create canonical protocol documents and fixtures that firmware, server, and Web/OpenAPI work consume without duplicating or redefining wire semantics.

## Scope

- Initialize Git and add a root `.gitignore`, `.editorconfig`, and concise `README.md` linking to the architecture/spec documentation.
- Create the complete top-level repository structure: `firmware/esp32/`, `server/`, `web/`, `protocol/`, `docs/architecture/`, `docs/development/`, and `tools/`.
- Move the existing architecture documents into their canonical locations, preserving content and updating internal/root README links:
  - `lifetrail-codebase.md` → `docs/project/codebase.md`
  - `lifetrail-firmware-esp32.md` → `docs/firmware/esp32.md`
  - `lifetrail-server-architecture.md` → `docs/server/architecture.md`
  - `lifetrail-web-architecture.md` → `docs/web/architecture.md`
- Do not leave duplicate root copies after verifying all moved-document links resolve.
- Add Phase 1 skeleton/placeholder files required to make ownership and future build entrypoints explicit: ESP-IDF project/component layout, Rust server crate layout, Vue/Vite app layout, protocol/OpenAPI directories, Docker Compose location, and CI workflow directory.
- Do not implement GPS, storage, sync, database handlers, or Web features in this ticket; later tickets own executable behavior.
- Add `protocol/gps-record-v1.md` for required/optional fields, ranges, Navigation Epoch provenance, strict in-batch `ts_ms` ordering, and UTF-8/LF NDJSON framing.
- Add `protocol/sync-batch-v1.md` for UUIDv4 batch IDs, manifest semantics, required headers, SHA-256 exact-byte definition, error/success responses, replay/hash conflict, and retry classification.
- Add valid and invalid fixtures: final-LF body, CRLF, blank line, missing final LF, malformed JSON, out-of-order timestamp, bad range, and same-ID/different-body examples.
- Define a single API-v1 error envelope in the protocol/OpenAPI-facing contract.

## Acceptance criteria

- `git status` works at the repository root and the initial project skeleton is committed as a distinct baseline commit.
- The root tree contains all Phase 1 ownership boundaries without source imports between firmware, server, and web.
- README links to the canonical architecture documents, Phase 1 spec, and protocol directory; generated/runtime/secrets artifacts are ignored.
- All four existing LifeTrail architecture documents exist only at their canonical `docs/` paths, preserve their intended source-of-truth roles, and contain no broken internal links caused by the move.
- `gps/1` names exactly `ts_ms`, `lat`, `lon`, `alt_m`, `speed_mps`, `course_deg`, `fix_quality`, `satellites`, and `hdop` with Phase 1 semantics.
- The document states that poor GPS quality is accepted Raw GPS while structural/range violations reject the whole Batch.
- The sync document names every `X-LifeTrail-*` request header and its byte-level verification rule.
- Fixture tests can be consumed by both firmware-side and server-side validators.

## Blocked by

None.

## Comments

- Initialized the repository baseline, created the Phase 1 ownership skeleton, moved the four canonical architecture documents, and added protocol/OpenAPI contracts plus byte fixtures. Validation covered fixture invariants, moved-document links, OpenAPI parsing, and the Rust skeleton test suite.
