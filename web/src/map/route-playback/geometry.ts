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
 * Destination point: travel `distanceM` from `from` along `bearingDeg`.
 * Used to build heading-aware marker geometry and look-ahead targets.
 */
export function destinationPoint(
  from: MapCoordinate,
  bearingDeg: number,
  distanceM: number,
): MapCoordinate {
  if (!(distanceM > 0)) {
    return [from[0], from[1]];
  }
  const angular = distanceM / EARTH_RADIUS_M;
  const theta = toRadians(bearingDeg);
  const phi1 = toRadians(from[1]);
  const lambda1 = toRadians(from[0]);
  const sinPhi2 =
    Math.sin(phi1) * Math.cos(angular) +
    Math.cos(phi1) * Math.sin(angular) * Math.cos(theta);
  const phi2 = Math.asin(Math.min(Math.max(sinPhi2, -1), 1));
  const lambda2 =
    lambda1 +
    Math.atan2(
      Math.sin(theta) * Math.sin(angular) * Math.cos(phi1),
      Math.cos(angular) - Math.sin(phi1) * Math.sin(phi2),
    );
  return [toDegrees(lambda2), toDegrees(phi2)];
}

/**
 * Point `distanceM` ahead of `position`, walking forward along the route
 * from `vertexIndex`. Clamps to the final coordinate when the route ends
 * sooner. This is the look-ahead camera target: the camera aims ahead of
 * the marker so the upcoming road stays visible.
 */
export function pointAheadOnRoute(
  points: PlaybackPoint[],
  vertexIndex: number,
  position: MapCoordinate,
  distanceM: number,
): MapCoordinate {
  if (!(distanceM > 0) || points.length === 0) {
    return [position[0], position[1]];
  }
  let remaining = distanceM;
  let cursor: MapCoordinate = [position[0], position[1]];
  const lastIndex = points.length - 1;
  for (let i = Math.max(vertexIndex, 0); i < lastIndex; i += 1) {
    const next = points[i + 1].coordinate;
    const legM = haversineDistanceM(cursor, next);
    if (legM >= remaining) {
      const ratio = legM === 0 ? 1 : remaining / legM;
      return [
        cursor[0] + (next[0] - cursor[0]) * ratio,
        cursor[1] + (next[1] - cursor[1]) * ratio,
      ];
    }
    remaining -= legM;
    cursor = [next[0], next[1]];
  }
  const last = points[lastIndex].coordinate;
  return [last[0], last[1]];
}

/**
 * Navigation-puck triangle as a closed GeoJSON linear ring, centered near
 * `position` and pointing along `bearingDeg`. Rendered flat on the map
 * (fill layer), so under a course-up camera it always points up-screen
 * along the direction of travel — a heading-aware marker with no sprites
 * or glyph dependencies.
 */
export function headingPuckRing(
  position: MapCoordinate,
  bearingDeg: number,
  lengthM = 26,
): MapCoordinate[] {
  const tip = destinationPoint(position, bearingDeg, lengthM * 0.55);
  const left = destinationPoint(position, bearingDeg + 150, lengthM * 0.5);
  const right = destinationPoint(position, bearingDeg - 150, lengthM * 0.5);
  return [tip, left, right, tip];
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
  let firstIndex = 0;
  for (let index = Math.min(vertexIndex - 1, points.length - 2); index >= 0; index -= 1) {
    if (points[index].breakToNext) {
      firstIndex = index + 1;
      break;
    }
  }
  const completed = points
    .slice(firstIndex, Math.min(vertexIndex + 1, points.length))
    .map((point) => [point.coordinate[0], point.coordinate[1]] as MapCoordinate);
  const last = completed[completed.length - 1];
  if (!last || last[0] !== position[0] || last[1] !== position[1]) {
    completed.push([position[0], position[1]]);
  }
  return completed;
}
