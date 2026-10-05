# Web app

This is the Vue/Vite boundary for the read-oriented local-network Web UI. It consumes generated types from [`../protocol/openapi/lifetrail-v1.yaml`](../protocol/openapi/lifetrail-v1.yaml); it does not own or duplicate API wire semantics.

Copy `.env.example` to `.env.local` and set the MapTiler values for a production basemap. The browser key is public configuration and must be origin-restricted, not treated as a secret.

For the Compose production build, export the same variables before rebuilding so Vite embeds the public browser configuration:

```sh
VITE_MAPTILER_KEY='<your-origin-restricted-key>' \
docker compose -f deploy/docker-compose.yml up --build -d
```

Without a configured MapTiler key, the Daily View renders its Route and markers on a plain local background rather than requesting a MapTiler style URL that would return HTTP 403.
