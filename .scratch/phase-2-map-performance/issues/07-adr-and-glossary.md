# 07: ADR 0008 + amendments + CONTEXT glossary

Status: resolved (docs-first)
Type: docs
Labels: phase-2, docs, adr
Blocked by: None

## Context

Spec §29/§40 + quyết định Q8/Q9. Thay đổi này không chỉ là optimization: nó
tách canonical Activity Revision khỏi Daily View representation và Playback
representation → cần ADR mới + amend ADR cũ.

## Scope

- **ADR 0008 — Separate Daily Display Projection from Canonical Playback
  Projection**:
  1. Activity Revision giữ canonical Route Part geometry/progress.
  2. Daily View chỉ publish display-oriented Route Parts.
  3. Canonical geometry/progress expose on-demand qua Playback API.
  4. `display_geometry` visualization-only.
  5. Distance/time authority không bao giờ suy từ `display_geometry`.
  6. Canonical payload nặng không nằm trong initial Daily Map load.
- **Amend 0004**: canonical RoutePart vẫn là authority; không còn hàm ý Daily
  View publish toàn bộ canonical object.
- **Amend 0005**: Daily Snapshot schema v2 chứa display projection.
- **Amend 0007**: semantics processed/raw GPS không đổi; đây là projection/API
  concern, không phải GPS processing.
- **CONTEXT.md**: thêm `Display Geometry`, `Playback Payload`,
  `Effective Tolerance`; amend `Route Part`, `Daily View`. KHÔNG thêm
  `Projection Schema Version`.
- Flag duplicate ADR `0001` (không renumber trong scope này).

## Acceptance

- ADR 0008 tồn tại, đúng format `docs/adr/`.
- 0004/0005/0007 có section amendment trỏ về 0008.
- CONTEXT.md có 3 term mới + 2 term amend.

## Verification

- Docs written: yes (ADR 0008 + amendments 0004/0005/0007 + CONTEXT glossary)
- Docs review: pending human review

## Out of scope

- Renumber ADR 0001 (flag only).
