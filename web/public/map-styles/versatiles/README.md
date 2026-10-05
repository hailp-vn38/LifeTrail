# VersaTiles styles

These ten MapLibre style assets are taken from [versatiles-style v6.1.1](https://github.com/versatiles-org/versatiles-style/releases/tag/v6.1.1), `styles.tar.gz`, using each palette's `style.json` (local-language labels). They are served locally so style definitions remain pinned and do not require a JavaScript runtime dependency or a MapTiler API key.

The only style change is setting the projection to `mercator` to preserve LifeTrail's route camera behavior. JSON is minified. The sky, fonts, sprites, source URLs, and OpenStreetMap / ESA attribution are preserved. Tiles, glyphs, and sprites are still downloaded from `tiles.versatiles.org`; these basemaps require internet access.

Source code license: [MIT](LICENSE.md). Upstream sprite assets are CC0. Map data attribution remains in each source and is displayed by MapLibre's attribution control.

To update, download a specific upstream release's `styles.tar.gz`, copy `colorful`, `natural`, `muted`, `gray`, `toner` and their `-dark` variants' `style.json` files to the matching local filenames, apply the Mercator projection, and retain the upstream attribution and license. Published styles already contain absolute tile URLs; do not substitute unresolved TileJSON references.
