# Rust server

This modular-monolith crate owns provisioning, device ingestion, persisted Raw GPS and Daily View APIs. It must implement the contracts in [`../protocol/`](../protocol/README.md), not redefine them.

## Local loop

Start the acceptance topology from the repository root:

```sh
docker compose -f deploy/docker-compose.yml up --build
```

Nginx Web is published at `http://localhost:8080`; PostgreSQL remains on the Compose network. During Web development, run `npm run dev` from `web/`; Vite proxies `/api` to the local Nginx entrypoint.

Provision the single Owner, then a Device. The Device command prints its 256-bit `lt_dev_` token exactly once; save it for serial provisioning.

```sh
export LT_DATABASE_URL=postgres://lifetrail:lifetrail_dev_only@localhost:5432/lifetrail
cargo run -- owner create --display-name "LifeTrail Owner" --timezone Asia/Ho_Chi_Minh
cargo run -- device create --owner-id <owner-uuid> --name "GPS Recorder"
```

Run the real PostGIS integration test with:

```sh
docker compose -f deploy/docker-compose.yml --profile test run --rm server-tests
```

Phase 2 processing uses local quality-filtered GPS only. The worker requires PostgreSQL/PostGIS and no external geometry provider. Migration 0015 retires committed matcher configuration, preserves existing immutable publications and queues processed-GPS replacements.
