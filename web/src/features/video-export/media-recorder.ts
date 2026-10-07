export function recordingMime(): string {
  if (typeof MediaRecorder === "undefined" || !HTMLCanvasElement.prototype.captureStream) {
    throw new Error("Trình duyệt này chưa hỗ trợ xuất video. Hãy thử Chrome hoặc Edge trên máy tính.");
  }
  const mime = ["video/mp4;codecs=avc1.42E01E", "video/webm;codecs=vp8", "video/webm", "video/mp4"]
    .find(value => MediaRecorder.isTypeSupported(value));
  if (!mime) throw new Error("Trình duyệt không có định dạng video phù hợp.");
  return mime;
}

export function createRecording(canvas: HTMLCanvasElement, mime: string) {
  const stream = canvas.captureStream(30);
  let recorder: MediaRecorder;
  try { recorder = new MediaRecorder(stream, { mimeType: mime, videoBitsPerSecond: 4_000_000 }); }
  catch (error) { stream.getTracks().forEach(track => track.stop()); throw error; }
  const chunks: Blob[] = [];
  const finished = new Promise<Blob>((resolve, reject) => {
    recorder.ondataavailable = event => { if (event.data.size) chunks.push(event.data); };
    recorder.onerror = () => reject(new Error("Không thể ghi video trên thiết bị này."));
    recorder.onstop = () => resolve(new Blob(chunks, { type: recorder.mimeType }));
  });
  return {
    finished,
    start: () => recorder.start(1000),
    stop: () => { if (recorder.state !== "inactive") recorder.stop(); },
    dispose: () => {
      if (recorder.state !== "inactive") recorder.stop();
      stream.getTracks().forEach(track => track.stop());
    },
  };
}

export function downloadVideo(blob: Blob, name: string) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `${name}.${blob.type.startsWith("video/mp4") ? "mp4" : "webm"}`;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 30_000);
}
