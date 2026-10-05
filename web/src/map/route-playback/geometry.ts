import type { MapCoordinate, PlaybackPoint } from "./types";

/**
 * Pure spatial helpers. No Vue imports. No map lifecycle management.
 */

const EARTH_RADIUS_M = 6_371_000;

/** Movement shorter than this is treated as GPS jitter, not travel direction. */
export const MIN_BEARING_DISTANCE_M = 5;

export function toRadians(degrees: number): number {
  return (degrees * Math.PI) / 180;
}

export function toDegrees(radians: number): number {
  return (radians * 180) / Math.PI;
}

/** Great-circle distance between two [longitude, latitude] pairs, in meters. */
export function haversineDistanceM(from: MapCoordinate, to: MapCoordinate): number {
  const lat1 = toRadians(from[1]);
  const lat2 = toRadians(to[1]);
  const deltaLat = toRadians(to[1] - from[1]);
  const deltaLon = toRadians(to[0] - from[0]);
  const halfChord =
    Math.sin(deltaLat / 2) ** 2 + Math.cos(lat1) * Math.cos(lat2) * Math.sin(deltaLon / 2) ** 2;
  return 2 * EARTH_RADIUS_M * Math.atan2(Math.sqrt(halfChord), Math.sqrt(1 - halfChord));
}

/** Normalize any bearing into the [0, 360) range MapLibre expects. */
export function normalizeBearing(degrees: number): number {
  return ((degrees % 360) + 360) % 360;
}

/**
 * Compass bearing from `from` to `to` in degrees, normalized to [0, 360).
 * Returns `null` when the two points are closer than MIN_BEARING_DISTANCE_M,
 * so callers keep their previous stable bearing instead of flipping on jitter.
 */
export function bearingBetween(from: MapCoordinate, to: MapCoordinate): number | null {
  if (haversineDistanceM(from, to) < MIN_BEARING_DISTANCE_M) {
    return null;
  }
  const lat1 = toRadians(from[1]);
  const lat2 = toRadians(to[1]);
  const deltaLon = toRadians(to[0] - from[0]);
  const y = Math.sin(deltaLon) * Math.cos(lat2);
  const x = Math.cos(lat1) * Math.sin(lat2) - Math.sin(lat1) * Math.cos(lat2) * Math.cos(deltaLon);
  return normalizeBearing(toDegrees(Math.atan2(y, x)));
}

export interface RouteBounds {
  min: MapCoordinate;
  max: MapCoordinate;
}

/** Bounding box of the route, or null when there are no points. */
export function routeBounds(points: PlaybackPoint[]): RouteBounds | null {
  if (points.length === 0) return null;
  let minLon = points[0].coordinate[0];
  let minLat = points[0].coordinate[1];
  let maxLon = minLon;
  let maxLat = minLat;
  for (const point of points) {
    const [lon, lat] = point.coordinate;
    if (lon < minLon) minLon = lon;
    if (lat < minLat) minLat = lat;
    if (lon > maxLon) maxLon = lon;
    if (lat > maxLat) maxLat = lat;
  }
  return { min: [minLon, minLat], max: [maxLon, maxLat] };
}

/**
 * Coordinates for the painted progress line: every fully completed vertex
 * plus the current interpolated position as the final coordinate, so the
 * marker never moves ahead of the painted line.
 */
export function progressCoordinates(
  points: PlaybackPoint[],
  vertexIndex: number,
  position: MapCoordinate,
): MapCoordinate[] {
  const completed = points
    .slice(0, Math.min(vertexIndex + 1, points.length))
    .map((point) => [point.coordinate[0], point.coordinate[1]] as MapCoordinate);
  const last = completed[completed.length - 1];
  if (!last || last[0] !== position[0] || last[1] !== position[1]) {
    completed.push([position[0], position[1]]);
  }
  return completed;
}
