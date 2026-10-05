# Web app

This is the Vue/Vite boundary for the read-oriented local-network Web UI. It consumes generated types from [`../protocol/openapi/lifetrail-v1.yaml`](../protocol/openapi/lifetrail-v1.yaml); it does not own or duplicate API wire semantics.

Copy `.env.example` to `.env.local` and set the MapTiler values for a production basemap. The browser key is public configuration and must be origin-restricted, not treated as a secret.
