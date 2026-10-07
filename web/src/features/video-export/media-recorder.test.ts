import { afterEach, describe, expect, it, vi } from "vitest";
import { createRecording, recordingMime } from "./media-recorder";

const originalCaptureStream = Object.getOwnPropertyDescriptor(HTMLCanvasElement.prototype, "captureStream");
afterEach(() => {
  vi.unstubAllGlobals(); vi.restoreAllMocks();
  if (originalCaptureStream) Object.defineProperty(HTMLCanvasElement.prototype, "captureStream", originalCaptureStream);
  else Reflect.deleteProperty(HTMLCanvasElement.prototype, "captureStream");
});

describe("video recorder", () => {
  it("reports unsupported browsers before starting a capture", () => {
    vi.stubGlobal("MediaRecorder", undefined);
    expect(() => recordingMime()).toThrow("chưa hỗ trợ");
  });

  it("falls back to a supported container", () => {
    vi.stubGlobal("MediaRecorder", { isTypeSupported: (value: string) => value === "video/webm;codecs=vp8" });
    Object.defineProperty(HTMLCanvasElement.prototype, "captureStream", { configurable: true, value: vi.fn() });
    expect(recordingMime()).toBe("video/webm;codecs=vp8");
  });

  it("includes the final chunk and releases capture tracks", async () => {
    const stopTrack = vi.fn();
    const stream = { getTracks: () => [{ stop: stopTrack }] };
    const canvas = { captureStream: vi.fn(() => stream) } as unknown as HTMLCanvasElement;
    class FakeRecorder {
      state = "inactive";
      mimeType = "video/webm";
      ondataavailable?: (event: { data: Blob }) => void;
      onstop?: () => void;
      start() { this.state = "recording"; this.ondataavailable?.({ data: new Blob(["first"]) }); }
      stop() { this.state = "inactive"; this.ondataavailable?.({ data: new Blob(["last"]) }); this.onstop?.(); }
    }
    vi.stubGlobal("MediaRecorder", FakeRecorder);
    const recording = createRecording(canvas, "video/webm");
    recording.start();
    recording.stop();
    const blob = await recording.finished;
    expect(blob.size).toBe(9);
    expect(blob.type).toBe("video/webm");
    recording.dispose();
    expect(stopTrack).toHaveBeenCalledOnce();
  });

  it("releases the stream when encoder construction fails", () => {
    const stopTrack = vi.fn();
    const canvas = { captureStream: () => ({ getTracks: () => [{ stop: stopTrack }] }) } as unknown as HTMLCanvasElement;
    vi.stubGlobal("MediaRecorder", class { constructor() { throw new Error("no encoder"); } });
    expect(() => createRecording(canvas, "video/webm")).toThrow("no encoder");
    expect(stopTrack).toHaveBeenCalledOnce();
  });
});
