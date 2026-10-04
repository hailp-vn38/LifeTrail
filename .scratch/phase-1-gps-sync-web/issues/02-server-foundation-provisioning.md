# 02 — Build local server foundation and CLI provisioning

Status: resolved
Type: task
Blocked by: 01

## Goal

Create the local/dev server baseline, database schema, Docker Compose topology, and CLI provisioning for the single-Owner data model.

## Scope

- Bootstrap Rust/Axum, SQLx, PostgreSQL/PostGIS migrations, tracing/request IDs, health endpoint, and local Docker Compose.
- Implement `users` (`id`, `display_name`, `timezone`, timestamps) and `devices` (`id`, `owner_user_id`, `name`, `token_digest`, timestamps) without Web auth fields.
- Implement CLI commands to create an Owner and Device; generate one 256-bit `lt_dev_` token, show plaintext only once, persist unique SHA-256 digest.
- Provide `GET /api/v1/devices` and Device lookup behavior appropriate for the local unauthenticated Web.
- Establish developer loop and acceptance topology: Vite proxy during development, built static web plus server on one LAN origin for acceptance.

## Acceptance criteria

- Migrations run against real PostgreSQL/PostGIS in automated integration tests.
- CLI-created Device appears in the device list and its raw token is not persisted in plaintext.
- Server resolves a valid Bearer token by digest and rejects an unknown token with the canonical error envelope.
- Postgres is not LAN-exposed in the Compose topology.

## Blocked by

01.

## Comments

- Implemented the Rust/Axum foundation, PostGIS migration, single-Owner CLI provisioning, token-digest resolution, local Web read routes, Vite proxy, and same-origin Compose topology. Local Rust tests, strict Clippy, Vite typecheck/build, Compose config, and whitespace checks pass. The real PostGIS integration test is automated in the `server-tests` Compose profile but remains unverified because the local Docker daemon was unavailable; ticket stays `claimed` until that gate runs.
- Verified the real PostgreSQL/PostGIS integration gate with `docker compose -f deploy/docker-compose.yml --profile test run --rm server-tests`: 1 passed, 0 failed. Docker Desktop on ARM runs the official amd64-only PostGIS image through the explicit Compose platform setting; PostgreSQL still has no published host/LAN port.
