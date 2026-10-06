import type { PlaybackBreak, PlaybackPoint } from "./types";

interface ProgressAnchorInput {
  at: string;
  distance_m: number;
}

interface RoutePartInput {
  id: string;
  geometry: { type: string; coordinates: number[][] };
  vertex_distance_m: number[];
  progress_anchors: ProgressAnchorInput[];
}

interface IntervalInput {
  observed_from_at: string;
  observed_until_at: string;
}

export type RoutePartPlaybackInput =
  | { ok: true; points: PlaybackPoint[] }
  | { ok: false; error: "invalid-route-parts" | "too-few-observations" };

/**
 * Turns canonical Route Parts into a historical playback path. Anchor
 * distances and vertex distances are both supplied by the server: this module
 * deliberately does not calculate lengths or assign times to vertices.
 */
export function buildRoutePartPlaybackInput(
  parts: RoutePartInput[],
  gaps: Array<IntervalInput & { kind: "gap" }> = [],
  holes: IntervalInput[] = [],
): RoutePartPlaybackInput {
  const partPoints: Array<{ point: PlaybackPoint; partId: string }> = [];
  for (const part of parts) {
    const points = pointsForPart(part);
    if (!points) return { ok: false, error: "invalid-route-parts" };
    for (const point of points) partPoints.push({ point, partId: part.id });
  }
  partPoints.sort((left, right) => left.point.recordedAtMs - right.point.recordedAtMs);
  if (partPoints.length < 2) return { ok: false, error: "too-few-observations" };

  const points = partPoints.map(({ point }) => ({ ...point }));
  for (let index = 0; index < points.length - 1; index += 1) {
    const current = points[index];
    const next = points[index + 1];
    const samePart = partPoints[index].partId === partPoints[index + 1].partId;
    if (!samePart) {
      current.breakToNext = interruptionBetween(current.recordedAtMs, next.recordedAtMs, gaps, holes);
    }
  }
  return { ok: true, points };
}

function pointsForPart(part: RoutePartInput): PlaybackPoint[] | null {
  const { coordinates } = part.geometry;
  if (
    part.geometry.type !== "LineString" ||
    coordinates.length < 2 ||
    part.vertex_distance_m.length !== coordinates.length ||
    part.progress_anchors.length < 2
  ) return null;
  const points: PlaybackPoint[] = [];
  let previousTime = -Infinity;
  let previousDistance = -Infinity;
  for (const anchor of part.progress_anchors) {
    const recordedAtMs = Date.parse(anchor.at);
    if (!Number.isFinite(recordedAtMs) || recordedAtMs <= previousTime || anchor.distance_m < previousDistance) return null;
    const coordinate = coordinateAtDistance(coordinates, part.vertex_distance_m, anchor.distance_m);
    if (!coordinate) return null;
    points.push({ coordinate, recordedAtMs });
    previousTime = recordedAtMs;
    previousDistance = anchor.distance_m;
  }
  return points;
}

function coordinateAtDistance(
  coordinates: number[][],
  vertexDistances: number[],
  distance: number,
): [number, number] | null {
  if (!Number.isFinite(distance) || distance < vertexDistances[0] || distance > vertexDistances.at(-1)!) return null;
  for (let index = 0; index < vertexDistances.length - 1; index += 1) {
    const fromDistance = vertexDistances[index];
    const untilDistance = vertexDistances[index + 1];
    if (distance > untilDistance) continue;
    const from = coordinates[index];
    const until = coordinates[index + 1];
    if (!validCoordinate(from) || !validCoordinate(until) || untilDistance < fromDistance) return null;
    const ratio = untilDistance === fromDistance ? 1 : (distance - fromDistance) / (untilDistance - fromDistance);
    return [from[0] + (until[0] - from[0]) * ratio, from[1] + (until[1] - from[1]) * ratio];
  }
  const last = coordinates.at(-1);
  return last && validCoordinate(last) ? [last[0], last[1]] : null;
}

function validCoordinate(value: number[] | undefined): value is [number, number] {
  return Boolean(value && Number.isFinite(value[0]) && Number.isFinite(value[1]));
}

function interruptionBetween(
  fromMs: number,
  untilMs: number,
  gaps: Array<IntervalInput & { kind: "gap" }>,
  holes: IntervalInput[],
): PlaybackBreak {
  const overlaps = (interval: IntervalInput) =>
    Date.parse(interval.observed_from_at) <= untilMs && Date.parse(interval.observed_until_at) >= fromMs;
  if (gaps.some(overlaps)) return "gps-gap";
  if (holes.some(overlaps)) return "evidence-hole";
  return "disconnected";
}
