# 02 — Build local server foundation and CLI provisioning

Status: open
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
