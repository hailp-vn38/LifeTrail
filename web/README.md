# Web app

This is the Vue/Vite boundary for the read-oriented local-network Web UI. It consumes generated types from [`../protocol/openapi/lifetrail-v1.yaml`](../protocol/openapi/lifetrail-v1.yaml); it does not own or duplicate API wire semantics.

Copy `.env.example` to `.env.local` and set the MapTiler values for a production basemap. The browser key is public configuration and must be origin-restricted, not treated as a secret.

For the Compose production build, export the same variables before rebuilding so Vite embeds the public browser configuration:

```sh
VITE_MAPTILER_KEY='<your-origin-restricted-key>' \
docker compose -f deploy/docker-compose.yml up --build -d
```

Without a configured MapTiler key, the Daily View renders its Route and markers on a plain local background rather than requesting a MapTiler style URL that would return HTTP 403.

Settings → Kiểu bản đồ offers the configured default, Streets 3D, Streets, and satellite imagery. The selection is saved in this browser. Streets 3D uses the MapTiler Streets style with an initial 55° pitch and building extrusions when the loaded style provides building data. Choosing the configured default restores `VITE_MAP_STYLE_URL`. MapTiler presets use `VITE_MAPTILER_KEY`.

The VersaTiles group adds Colorful, Natural, Muted, Gray, and Toner, each in light and dark variants. These use [vendored VersaTiles v6.1.1 styles](public/map-styles/versatiles/README.md) and public `tiles.versatiles.org` tiles/fonts/sprites, without a MapTiler key. The existing playback camera and browser preference persistence also apply to these presets.
