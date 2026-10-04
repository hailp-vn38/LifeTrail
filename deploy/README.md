# Deployment

`docker-compose.yml` is the canonical Compose topology. It runs PostgreSQL/PostGIS only on the internal Compose network and publishes the Rust server at port `8080`. The server image builds the Vue static app and serves it from the same LAN origin, so `/` and `/api/v1/...` share `http://<LAN-IP>:8080`.

The `server-tests` profile runs the migration/provisioning integration test against the real Compose PostGIS service.
