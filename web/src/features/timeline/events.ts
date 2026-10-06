import type { DailyView } from "../../api/queries/daily-view.query";
import { formatDuration, formatTimestamp } from "../../lib/format";
import type { TimelineEvent } from "./types";

function pointCoordinate(feature: unknown): [number, number] | undefined {
  const geometry = (feature as { geometry?: { coordinates?: unknown } })?.geometry;
  const coordinates = geometry?.coordinates;
  if (
    Array.isArray(coordinates) &&
    typeof coordinates[0] === "number" &&
    typeof coordinates[1] === "number"
  ) {
    return [coordinates[0], coordinates[1]];
  }
  return undefined;
}

function recordedAtMs(feature: unknown): number | undefined {
  const properties = (feature as { properties?: Record<string, unknown> })?.properties;
  const raw = properties?.["recorded_at"];
  if (typeof raw === "string") {
    const ms = Date.parse(raw);
    return Number.isNaN(ms) ? undefined : ms;
  }
  return undefined;
}

/** Project published activity or the established Raw Start/End view. */
export function buildTimelineEvents(dailyView: DailyView): TimelineEvent[] {
  if (dailyView.processing_state === "processed") {
    return (dailyView.timeline ?? []).map((stop) => ({
      id: stop.id, kind: stop.kind, title: "Stop",
      subtitle: [
        `Quan sát: ${formatTimestamp(stop.observed_from_at, dailyView.timezone)} – ${formatTimestamp(stop.observed_until_at, dailyView.timezone)} (${formatDuration(stop.observed_duration_s)})`,
        `Trong ngày: ${formatDuration(stop.daily_observed_duration_s)}`,
        `Đến: ${stop.actual_start_at ? formatTimestamp(stop.actual_start_at, dailyView.timezone) : "chưa xác định"}`,
        `Rời: ${stop.actual_end_at ? formatTimestamp(stop.actual_end_at, dailyView.timezone) : "chưa xác định"}`,
      ].join(" · "),
      coordinate: [stop.center[0], stop.center[1]],
      recordedAtMs: Date.parse(stop.visible_from_at),
    }));
  }
  const events: TimelineEvent[] = [];

  if (dailyView.start) {
    events.push({
      id: "start",
      kind: "start",
      title: "Start",
      subtitle: formatTimestamp(
        dailyView.summary.first_fix_at,
        dailyView.timezone,
      ),
      coordinate: pointCoordinate(dailyView.start),
      recordedAtMs: recordedAtMs(dailyView.start),
    });
  }

  if (dailyView.end) {
    events.push({
      id: "end",
      kind: "end",
      title: "End",
      subtitle: formatTimestamp(dailyView.summary.last_fix_at, dailyView.timezone),
      coordinate: pointCoordinate(dailyView.end),
      recordedAtMs: recordedAtMs(dailyView.end),
    });
  }

  return events;
}
