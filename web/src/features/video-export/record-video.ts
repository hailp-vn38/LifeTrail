import type { DailyView } from "../../api/queries/daily-view.query";
import type { MapStyleId } from "../../map/style-presets";
import { PlaybackController } from "../../map/route-playback/controller";
import { createCompositor, loadMapLogo } from "./compositor";
import { MAX_VIDEO_SECONDS, videoDuration, videoInput } from "./export-plan";
import { createExportMap } from "./map-renderer";
import { createRecording, recordingMime } from "./media-recorder";

export interface VideoOptions {
  view: DailyView;
  container: HTMLElement;
  styleId: MapStyleId;
  startMs: number;
  endMs: number;
  speed: number;
  follow: boolean;
  signal: AbortSignal;
  onState: (state: "preparing" | "recording" | "finalizing") => void;
  onProgress: (progress: number) => void;
}

export async function recordVideo(options: VideoOptions): Promise<Blob> {
  const { view, signal, startMs, endMs } = options;
  signal.throwIfAborted();
  const mime = recordingMime();
  const points = videoInput(view);
  if (startMs < points[0].recordedAtMs || endMs > points.at(-1)!.recordedAtMs) throw new Error("Khoảng xuất nằm ngoài dữ liệu GPS.");
  if (videoDuration(startMs, endMs, options.speed) > MAX_VIDEO_SECONDS) throw new Error("Video tối đa 10 phút. Hãy tăng tốc độ hoặc rút ngắn khoảng xuất.");
  const composition = createCompositor(view);
  const scene = createExportMap(options.container, view, points, options.styleId);
  let recording: ReturnType<typeof createRecording> | undefined;
  let controller: PlaybackController | undefined;
  let removeRender: (() => void) | undefined;
  let heartbeat: ReturnType<typeof setInterval> | undefined;
  const hidden = () => { if (document.hidden) cancel(new Error("Đã dừng xuất vì tab bị ẩn. Hãy giữ tab mở khi xuất video.")); };
  let cancel: (reason: unknown) => void = () => {};
  const visibilityAbort = new AbortController();
  const activeSignal = visibilityAbort.signal;
  cancel = reason => visibilityAbort.abort(reason);
  const externalAbort = () => cancel(signal.reason);
  signal.addEventListener("abort", externalAbort, { once: true });
  scene.onFailure(cancel);
  document.addEventListener("visibilitychange", hidden);
  try {
    if (document.hidden) throw new Error("Hãy giữ tab mở khi xuất video.");
    options.onState("preparing");
    await scene.prepare(activeSignal);
    const logo = scene.isMapTiler() ? await loadMapLogo(activeSignal) : undefined;
    removeRender = scene.onRender((canvas, frame, attribution) => {
      try { composition.draw(canvas, frame, startMs, attribution, logo); }
      catch { cancel(new Error("Không thể đọc canvas bản đồ. Hãy thử kiểu bản đồ khác.")); }
    });
    controller = new PlaybackController({
      points, startTimeMs: startMs, endTimeMs: endMs, speed: options.speed,
      events: { onFrame: frame => { scene.apply(frame, startMs, options.follow); options.onProgress(frame.progress); } },
    });
    controller.restart();
    await scene.idle(activeSignal);
    scene.repaint();
    // Checking pixel access catches a tainted compositor before recording starts.
    composition.canvas.getContext("2d")!.getImageData(0, 0, 1, 1);
    recording = createRecording(composition.canvas, mime);
    const completion = new Promise<void>((resolve, reject) => {
      const abort = () => { cleanup(); reject(activeSignal.reason); };
      const cleanup = () => { clearInterval(poll); activeSignal.removeEventListener("abort", abort); };
      const poll = setInterval(() => {
        if (controller?.currentState === "finished") { cleanup(); resolve(); }
      }, 50);
      activeSignal.addEventListener("abort", abort, { once: true });
      if (activeSignal.aborted) abort();
    });
    // Also observe encoder errors while waiting for route completion.
    const encoderFailure = recording.finished.then(() => { throw new Error("Phiên ghi video đã kết thúc sớm."); });
    const completed = Promise.race([completion, encoderFailure]);
    // start() can throw synchronously; cleanup still rejects the outstanding wait.
    void completed.catch(() => {});
    recording.start();
    options.onState("recording");
    controller.play();
    heartbeat = setInterval(() => scene.repaint(), 1000 / 30);
    await completed;
    options.onState("finalizing");
    await scene.idle(activeSignal);
    // Hold the final GPS position long enough to include it in the stream.
    await new Promise<void>((resolve, reject) => {
      const abort = () => { clearTimeout(timer); reject(activeSignal.reason); };
      const timer = setTimeout(() => { activeSignal.removeEventListener("abort", abort); resolve(); }, 500);
      activeSignal.addEventListener("abort", abort, { once: true });
      if (activeSignal.aborted) abort();
    });
    recording.stop();
    const blob = await recording.finished;
    activeSignal.throwIfAborted();
    if (!blob.size) throw new Error("Video không có dữ liệu. Hãy thử lại.");
    return blob;
  } finally {
    // Stop outstanding completion waits even after an encoder/setup failure.
    cancel(new DOMException("Phiên xuất đã kết thúc.", "AbortError"));
    document.removeEventListener("visibilitychange", hidden);
    signal.removeEventListener("abort", externalAbort);
    if (heartbeat) clearInterval(heartbeat);
    controller?.dispose();
    recording?.dispose();
    removeRender?.();
    scene.dispose();
  }
}
