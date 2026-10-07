import type { DailyView } from "../../api/queries/daily-view.query";
import type { PlaybackFrame } from "../../map/route-playback/types";
import { INTERRUPTION_TEXT } from "../../map/route-playback/messages";

export async function loadMapLogo(signal: AbortSignal): Promise<HTMLImageElement> {
  const response = await fetch(`${import.meta.env.BASE_URL}map-attribution/maptiler.svg`, { signal });
  if (!response.ok) throw new Error("Không thể tải logo bản đồ.");
  const url = URL.createObjectURL(await response.blob());
  try {
    const image = new Image();
    image.src = url;
    await image.decode();
    signal.throwIfAborted();
    return image;
  } finally { URL.revokeObjectURL(url); }
}

export function createCompositor(view: DailyView) {
  const canvas = document.createElement("canvas");
  canvas.width = 1280;
  canvas.height = 720;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("Không thể tạo khung hình video.");
  const clock = new Intl.DateTimeFormat("vi-VN", {
    timeZone: view.timezone, hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false,
  });
  return {
    canvas,
    draw(mapCanvas: HTMLCanvasElement, frame: PlaybackFrame, startMs: number, attribution: string, logo?: HTMLImageElement) {
      context.drawImage(mapCanvas, 0, 0, canvas.width, canvas.height);
      context.fillStyle = "rgba(15, 23, 42, .85)";
      context.fillRect(0, 0, 1280, frame.interruption ? 102 : 64);
      context.fillStyle = "white";
      context.font = "bold 24px sans-serif";
      context.fillText(`LifeTrail · ${view.date} · ${clock.format(startMs + frame.routeTimeMs)}`, 24, 40);
      if (frame.interruption) {
        context.font = "18px sans-serif";
        context.fillText(INTERRUPTION_TEXT[frame.interruption], 24, 80, 1232);
      }
      context.fillStyle = "rgba(255,255,255,.95)";
      context.fillRect(0, 662, 1280, 58);
      if (logo) context.drawImage(logo, 16, 672, 120, 28);
      context.fillStyle = "#0f172a";
      context.font = "16px sans-serif";
      context.textAlign = "right";
      context.fillText(attribution || "LifeTrail", 1264, 698, logo ? 1100 : 1232);
      context.textAlign = "left";
    },
  };
}
