import type { DailyView } from "../../api/queries/daily-view.query";
import { tripDistanceM, type MovementSegment } from "../activity/model";
import { formatDistance, formatDuration, formatTimestamp } from "../../lib/format";
import { describeGap } from "./evidence";
import type { TimelineEvent } from "./types";

/** Ordered Movement Segment modes; `unknown` is shown as undetermined. */
function describeSegments(segments: MovementSegment[]): string {
  const modes: string[] = [];
  for (const segment of segments) {
    const mode = segment.mode === "unknown" ? "chưa xác định" : segment.mode;
    if (modes[modes.length - 1] !== mode) modes.push(mode);
  }
  return `Phương tiện: ${modes.join(" → ")}`;
}

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
    // The server publishes chronological Trip/Stop items, so their order is kept.
    return (dailyView.timeline ?? []).map((activity) => {
      const observed = [
        `Quan sát: ${formatTimestamp(activity.observed_from_at, dailyView.timezone)} – ${formatTimestamp(activity.observed_until_at, dailyView.timezone)} (${formatDuration(activity.observed_duration_s)})`,
        `Trong ngày: ${formatDuration(activity.daily_observed_duration_s)}`,
      ].join(" · ");
      if (activity.kind === "gap") {
        // A Gap is an absence of observations: its own item, with no coordinate
        // to focus and no implied travel.
        return {
          id: activity.id,
          kind: activity.kind,
          ...describeGap(activity, dailyView.timezone),
          recordedAtMs: Date.parse(activity.visible_from_at),
        };
      }
      if (activity.kind === "stop") {
        return {
          id: activity.id,
          kind: activity.kind,
          title: "Stop",
          subtitle: [
            observed,
            `Đến: ${activity.actual_start_at ? formatTimestamp(activity.actual_start_at, dailyView.timezone) : "chưa xác định"}`,
            `Rời: ${activity.actual_end_at ? formatTimestamp(activity.actual_end_at, dailyView.timezone) : "chưa xác định"}`,
          ].join(" · "),
          coordinate: [activity.center[0], activity.center[1]],
          recordedAtMs: Date.parse(activity.visible_from_at),
        };
      }
      return {
        id: activity.id,
        kind: activity.kind,
        title: "Trip",
        subtitle: [
          observed,
          // Trip time includes pauses below the Stop criteria, so it is never
          // presented as physical moving duration.
          `Trong Trip (bao gồm dừng ngắn): ${formatDuration(activity.daily_observed_duration_s)}`,
          `Quãng đường: ${formatDistance(tripDistanceM(dailyView, activity.id))}`,
          describeSegments(activity.movement_segments),
        ].join(" · "),
        recordedAtMs: Date.parse(activity.visible_from_at),
      };
    });
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
