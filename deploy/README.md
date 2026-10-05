# Deployment

`docker-compose.yml` is the canonical Compose topology. Three runtime services:

- `web` — Nginx serving the built Vue SPA. It owns host port `8080` and is the
  only public LAN entrypoint (`http://<LAN-IP>:8080`). `/` and `/assets/*`
  serve the SPA (with an SPA fallback for Vue Router deep links); `/api/*`
  is reverse-proxied to the `server` service (see `deploy/nginx.conf`).
- `server` — Rust/Axum API on the internal Compose network (`server:8080`).
  The server image no longer builds or contains web assets; static web
  hosting is opt-in via `LT_STATIC_DIR` only.
- `postgres` — PostgreSQL/PostGIS, internal only, with a persistent volume.

Browser and ESP32 devices both use the same origin:

```text
http://<LAN-IP>:8080/                        # Web SPA
http://<LAN-IP>:8080/api/v1/devices          # API through Nginx
http://<LAN-IP>:8080/api/v1/device/batches   # ESP32 batch upload (unchanged)
```

Build arguments for the web image (`VITE_MAP_STYLE_URL`, `VITE_MAPTILER_KEY`)
are read from the environment at build time:

```sh
VITE_MAPTILER_KEY=... scripts/lifetrail build web
```

The `server-tests` profile runs the migration/provisioning integration test
against the real Compose PostGIS service.
