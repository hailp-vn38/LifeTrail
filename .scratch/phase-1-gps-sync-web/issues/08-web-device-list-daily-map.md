# 08 — Build local Web Device list and Daily Route map

Status: open
Type: task
Blocked by: 07

## Goal

Build the no-auth Vue daily-route experience from OpenAPI-generated types and the canonical URL state.

## Scope

- Add Vue/Vite, router, TanStack Vue Query, generated OpenAPI client, and MapLibre integration.
- Render `/` Device list and `/devices/:deviceId/day/:date` Daily View.
- Select today's date from the Device Owner timezone; do not redirect automatically to a latest data day.
- Render summary, GeoJSON route, start/end, loading/error, and zero-data empty states.
- Configure MapTiler style/key through Vite environment values; do not treat the browser key as secret.
- Produce a static production build suitable for single-origin acceptance topology.

## Acceptance criteria

- Reloading a Daily View preserves Device and date from the URL.
- Daily View uses generated API types, not handwritten duplicate DTOs.
- MapLibre renders route with source/layer data rather than one DOM marker per raw point.
- A valid no-data response renders empty UI; nonexistent Device renders a distinct not-found/error state.
- Typecheck, unit/component tests, and static build pass.

## Blocked by

07.
