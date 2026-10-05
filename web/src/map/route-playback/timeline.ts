import type { MapCoordinate, PlaybackPoint, SegmentLocation } from "./types";

/**
 * Pure GPS-time playback math. No Vue imports. No MapLibre imports.
 *
 * All timing is derived from per-point GPS timestamps, never from vertex
 * index or distance, so irregular sampling keeps its real durations.
 */

export type PlaybackValidationError =
  | "not-linestring"
  | "too-few-coordinates"
  | "missing-timestamps"
  | "timestamp-count-mismatch"
  | "invalid-timestamp"
  | "non-monotonic-timestamps";

export type PlaybackInput =
  | { ok: true; points: PlaybackPoint[] }
  | { ok: false; error: PlaybackValidationError };

/** Minimal structural shape of the Daily View route feature. */
export interface RouteFeatureInput {
  type: string;
  properties: { timestamps?: unknown } | null;
  geometry: { type: string; coordinates: unknown };
}

/**
 * Validate a route feature and normalize it into playback points.
 * Never reorders coordinates and never fabricates timestamps: any problem
 * is reported as a controlled error so the UI can fall back to the static map.
 */
export function buildPlaybackInput(feature: RouteFeatureInput | null | undefined): PlaybackInput {
  if (!feature || feature.type !== "Feature" || feature.geometry?.type !== "LineString") {
    return { ok: false, error: "not-linestring" };
  }
  const coordinates = feature.geometry.coordinates;
  if (!Array.isArray(coordinates) || coordinates.length < 2) {
    return { ok: false, error: "too-few-coordinates" };
  }
  const timestamps = feature.properties?.timestamps;
  if (!Array.isArray(timestamps)) {
    return { ok: false, error: "missing-timestamps" };
  }
  if (timestamps.length !== coordinates.length) {
    return { ok: false, error: "timestamp-count-mismatch" };
  }

  const points: PlaybackPoint[] = [];
  for (let index = 0; index < coordinates.length; index += 1) {
    const raw = coordinates[index];
    if (
      !Array.isArray(raw) ||
      typeof raw[0] !== "number" ||
      typeof raw[1] !== "number" ||
      !Number.isFinite(raw[0]) ||
      !Number.isFinite(raw[1])
    ) {
      return { ok: false, error: "not-linestring" };
    }
    const recordedAtMs = Date.parse(timestamps[index]);
    if (!Number.isFinite(recordedAtMs)) {
      return { ok: false, error: "invalid-timestamp" };
    }
    if (points.length > 0 && recordedAtMs < points[points.length - 1].recordedAtMs) {
      return { ok: false, error: "non-monotonic-timestamps" };
    }
    const coordinate: MapCoordinate = [raw[0], raw[1]];
    points.push({ coordinate, recordedAtMs });
  }
  return { ok: true, points };
}

/** Logical route duration in milliseconds: last timestamp minus first. */
export function routeDurationMs(points: PlaybackPoint[]): number {
  if (points.length < 2) return 0;
  return Math.max(0, points[points.length - 1].recordedAtMs - points[0].recordedAtMs);
}

/** Clamp a logical route time into [0, durationMs]. */
export function clampRouteTime(routeTimeMs: number, durationMs: number): number {
  if (!Number.isFinite(routeTimeMs)) return 0;
  return Math.min(Math.max(routeTimeMs, 0), Math.max(durationMs, 0));
}

/**
 * Locate the playback cursor for a logical route time using binary search,
 * so seeking never replays frames from the start.
 *
 * Duplicate timestamps are handled without division by zero: the search finds
 * the *last* vertex at or before the requested time, which selects the later
 * point of a zero-duration segment ("immediately advanced").
 */
export function locateSegment(points: PlaybackPoint[], routeTimeMs: number): SegmentLocation {
  const first = points[0];
  const lastIndex = points.length - 1;
  const durationMs = Math.max(0, points[lastIndex].recordedAtMs - first.recordedAtMs);
  // Work in logical time relative to the first point.
  const time = clampRouteTime(routeTimeMs, durationMs);
  const relativeMs = (index: number) => points[index].recordedAtMs - first.recordedAtMs;

  let low = 0;
  let high = lastIndex;
  while (low < high) {
    const mid = Math.floor((low + high + 1) / 2);
    if (relativeMs(mid) <= time) {
      low = mid;
    } else {
      high = mid - 1;
    }
  }

  if (low >= lastIndex) {
    const last = points[lastIndex].coordinate;
    return {
      vertexIndex: Math.max(lastIndex - 1, 0),
      segmentRatio: 1,
      position: [last[0], last[1]],
    };
  }

  const start = points[low];
  const end = points[low + 1];
  const spanMs = relativeMs(low + 1) - relativeMs(low);
  if (spanMs <= 0) {
    // Zero-duration segment: immediately advanced to the later point.
    return {
      vertexIndex: low,
      segmentRatio: 1,
      position: [end.coordinate[0], end.coordinate[1]],
    };
  }
  const ratio = Math.min(Math.max((time - relativeMs(low)) / spanMs, 0), 1);
  return {
    vertexIndex: low,
    segmentRatio: ratio,
    position: interpolateCoordinate(start.coordinate, end.coordinate, ratio),
  };
}

/** Linear interpolation between two coordinates. */
export function interpolateCoordinate(
  from: MapCoordinate,
  to: MapCoordinate,
  ratio: number,
): MapCoordinate {
  const clamped = Math.min(Math.max(ratio, 0), 1);
  return [
    from[0] + (to[0] - from[0]) * clamped,
    from[1] + (to[1] - from[1]) * clamped,
  ];
}

/** Playback progress as a 0..1 ratio of logical route time. */
export function progressRatio(routeTimeMs: number, durationMs: number): number {
  if (!(durationMs > 0)) return 1;
  return Math.min(Math.max(routeTimeMs / durationMs, 0), 1);
}
