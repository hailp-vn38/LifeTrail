import type { DailyView } from "../../api/queries/daily-view.query";
import { formatTimestamp } from "../../lib/format";
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

/**
 * Build Phase 1 timeline events from a Daily View: Start and End only.
 * Phase 2 will append trip/stop/photo/audio events here; the panel, list
 * and map selection already speak the generic `TimelineEvent` shape.
 */
export function buildTimelineEvents(dailyView: DailyView): TimelineEvent[] {
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
