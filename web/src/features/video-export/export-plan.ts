import type { DailyView } from "../../api/queries/daily-view.query";
import type { RoutePart } from "../activity/model";
import { evidenceHoles, gapActivities } from "../activity/model";
import { buildRoutePartPlaybackInput } from "../../map/route-playback/parts";
import { buildPlaybackInput } from "../../map/route-playback/timeline";
import { ISSUE_TEXT, PART_ISSUE_TEXT } from "../../map/route-playback/messages";

export const MAX_VIDEO_SECONDS = 600;

/**
 * Playback points for the video. Processed days require the canonical Playback
 * resource (`parts`), fetched lazily by the dialog; display geometry is not a
 * valid playback source. Raw days use the Raw route directly.
 */
export function videoInput(view: DailyView, parts?: RoutePart[]) {
  if (view.route_parts?.length) {
    if (!parts?.length) throw new Error("Đang tải dữ liệu playback. Vui lòng thử lại.");
    const input = buildRoutePartPlaybackInput(parts, gapActivities(view), evidenceHoles(view));
    if (!input.ok) throw new Error(PART_ISSUE_TEXT[input.error]);
    return input.points;
  }
  const input = buildPlaybackInput(view.route);
  if (!input.ok) throw new Error(ISSUE_TEXT[input.error]);
  return input.points;
}

export function videoDuration(startMs: number, endMs: number, speed: number) {
  if (!Number.isFinite(startMs) || !Number.isFinite(endMs) || endMs <= startMs || !Number.isFinite(speed) || speed <= 0) {
    throw new Error("Khoảng thời gian hoặc tốc độ không hợp lệ.");
  }
  return (endMs - startMs) / speed / 1000;
}
