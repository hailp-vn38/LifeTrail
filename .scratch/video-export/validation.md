# Video export validation

Validated 2026-10-07.

- Web suite: 35 files, 201 tests passed. The seven export plan/recorder tests also passed in a focused rerun.
- `npm run typecheck` and `npm run build` passed. Vite retains its existing large-chunk warning.
- Chromium headless with software WebGL, using the Vite web origin on an ordinary HTTP LAN IP (`isSecureContext = false`). The browser API responses were mocked with the existing `tripView()` fixture; the configured basemap was fetched normally.
- Header action menu opened Video; the refresh button was absent.
- Exported a playable MP4, 1280×720, approximately 12.9 seconds for twenty minutes of Route coverage at 100x, including setup/finish timing around the recording.
- Inspected a decoded middle frame: advancing Owner-local GPS clock, moving heading puck, route progress, basemap, MapTiler logo and source attribution were visible. The compositor advanced from 10:00:00 through 10:20:00.
- Cancel during recording succeeded; starting another session succeeded. Simulated a hidden-tab visibility change and confirmed the controlled cancellation message. No browser page errors were reported.
- Actual frame rate on software rendering was below the 30 FPS target; the output format and performance remain browser/device dependent. Safari, Firefox, all style presets and physical mobile devices have not been exercised.

Browser tooling and video/screenshot artifacts were kept under `/tmp/lifetrail-video-smoke`, outside the repository. No test dependencies were added to the project and no services were redeployed.
