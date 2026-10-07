# Browser video export research

Researched 2026-10-07. Primary specifications and provider documentation; no implementation changes.

## Decision for current deployment

Start with **a separate export canvas + `captureStream(30)` + `MediaRecorder`**, using a dedicated MapLibre export scene and drawing the title/time/attribution onto the export canvas. This is an architectural recommendation, not a tested browser guarantee. It keeps the Rust service and Docker topology unchanged and supports the project's documented `http://<LAN-IP>:8080` deployment. Canvas capture and MediaRecorder specifications expose these interfaces without a SecureContext restriction; they do not require camera/screen access. Feature-detect both APIs on actual target devices. [Canvas capture specification](https://w3c.github.io/mediacapture-fromelement/), [MediaRecorder specification](https://www.w3.org/TR/mediastream-recording/).

WebCodecs `VideoEncoder` explicitly requires a secure context. Loopback/localhost are potentially trustworthy, but a normal HTTP private LAN IP has no such exception: testing on localhost does **not** establish that WebCodecs will run in the deployed app. Require trusted HTTPS before making WebCodecs the primary encoder. [WebCodecs specification](https://www.w3.org/TR/webcodecs/#videoencoder-interface), [Secure Contexts specification](https://www.w3.org/TR/secure-contexts/#is-origin-trustworthy).

## Options

| Option | Benefits | Limits / project implication |
| --- | --- | --- |
| Canvas + MediaRecorder | Small integration, native encoding, downloadable file in browser | Capture is real-time: a 60-second output normally takes about 60 seconds to record. Keep export duration short; speed/duration must be explicitly chosen. |
| Explicit frame rendering + WebCodecs + muxer | Set timestamps per frame; potentially render faster/slower than video duration without altering output timing; precise frame count | Trusted HTTPS, supported encoder configuration, muxer dependency, resource/backpressure handling; additional engineering to make map and camera deterministic. |
| Headless browser frame renderer + native FFmpeg | Controlled MP4 output independent of user's codec support; can execute as background job | Add Chromium/FFmpeg worker, job lifecycle, concurrency limits, temporary storage/download cleanup and authorization. Rendering still needs map tiles/WebGL. Too much infrastructure for an initial short-video feature. |

Canvas capture defines a real-time video stream; `captureStream(0)` allows manual `requestFrame()`, but this does not provide custom media timestamps or make MediaRecorder an offline deterministic encoder. [Canvas capture specification](https://w3c.github.io/mediacapture-fromelement/).

WebCodecs accepts timestamped `VideoFrame`s and produces encoded chunks, not a finished MP4/WebM. Mux those chunks into a container; bound `encodeQueueSize`, release each frame with `close()`, and flush/close on completion or cancellation. Faster-than-real-time export is a possibility, not a guarantee: tile loading, GPU drawing and encoding remain bottlenecks. [WebCodecs specification](https://www.w3.org/TR/webcodecs/), [Chrome implementation guide](https://developer.chrome.com/docs/web-platform/best-practices/webcodecs).

FFmpeg can encode numbered image sequences or image pipes with explicit input framerate. The proposed headless job service and operational costs are an inference from requiring a server renderer plus encoder, rather than existing project infrastructure. [FFmpeg formats](https://ffmpeg.org/ffmpeg-formats.html#image2), [FFmpeg FAQ](https://www.ffmpeg.org/faq.html#How-do-I-encode-single-pictures-into-movies_003f).

## Required implementation details

- **Negotiate output format at runtime.** Probe candidate MP4/H.264 and WebM/VP8 MIME types with `MediaRecorder.isTypeSupported`; use the actual chosen MIME/container for the extension. A successful probe still cannot guarantee enough resources to record. Do not promise MP4 on every browser. [MediaRecorder specification](https://www.w3.org/TR/mediastream-recording/#dom-mediarecorder-istypesupported).
- **Capture the scene, compose the presentation.** `map.getCanvas()` returns only the map canvas. HTML controls, popups, playback bar and attribution UI are outside its bitmap; draw desired text/branding/credits separately. Existing GL route/puck layers are naturally part of the scene. [MapLibre Map API](https://maplibre.org/maplibre-gl-js/docs/API/classes/Map/#getcanvas), [canvas surface capture](https://w3c.github.io/mediacapture-fromelement/).
- **Verify framebuffer timing.** MapLibre's documented default is `preserveDrawingBuffer: false`. For a separate exporter, evaluate capture during the render callback versus enabling preservation on that instance; test black/blank frames before selecting the approach. Do not enable preservation globally merely for export. [MapLibre MapOptions](https://maplibre.org/maplibre-gl-js/docs/API/type-aliases/MapOptions/#canvascontextattributes).
- **Check origin cleanliness.** A tainted canvas cannot be captured and may mute an already-created stream. Validate every external raster/sprite/logo asset with CORS; cross-origin does not itself imply taint when assets are fetched correctly. [Canvas capture specification](https://w3c.github.io/mediacapture-fromelement/#html-canvas-element-media-capture-extensions).
- **Burn attribution into the video.** MapTiler explicitly allows attribution in a video corner or credits and requires provider/data text; Free accounts additionally require the logo. VersaTiles prebuilt styles contain attribution, but their hosted OSM tiles also include ESA WorldCover data. Read current source attribution rather than hardcoding only OSM. [MapTiler attribution guide](https://docs.maptiler.com/guides/map-design/attribution/add-attribution/), [VersaTiles tile guide](https://docs.versatiles.org/guides/use_tiles_versatiles_org).
- **Cap output and clean up.** Proposed first release: 1280×720, 30 fps, 15/30/60-second presets, no sound; allow cancel, collect final recorder data before downloading, stop capture tracks and revoke object URLs. Resource caps are design recommendations; encoder bitrates are hints and large buffered recordings consume memory. [MediaRecorder specification](https://www.w3.org/TR/mediastream-recording/#resource-exhaustion).

## Validation before shipping

Run real browser acceptance tests on the deployed LAN HTTP origin: Chrome/Edge desktop, Firefox, Safari/iOS if supported. Confirm MIME detection, first/last frames, GL puck/line, burnt-in overlay/credits, CORS with every shipped style, cancellation/navigation cleanup and handling tab visibility changes. These are proposed checks, not tests performed during this analysis.
