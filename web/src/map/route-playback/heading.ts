import { bearingBetween } from "./geometry";
import type { MapCoordinate, PlaybackPoint } from "./types";

/**
 * Use a spatial baseline instead of just the next GPS record: dense sampling
 * must not be mistaken for stationary jitter. At the end, use the incoming
 * direction so seeking there does not depend on which frames were played.
 */
export function routeBearing(
  points: PlaybackPoint[],
  vertexIndex: number,
  position: MapCoordinate,
): number | null {
  const segmentBearing = bearingBetween(
    points[vertexIndex].coordinate,
    points[vertexIndex + 1].coordinate,
  );
  if (segmentBearing !== null) return segmentBearing;
  for (let i = vertexIndex + 1; i < points.length; i += 1) {
    const bearing = bearingBetween(position, points[i].coordinate);
    if (bearing !== null) return bearing;
  }
  for (let i = vertexIndex; i >= 0; i -= 1) {
    const bearing = bearingBetween(points[i].coordinate, position);
    if (bearing !== null) return bearing;
  }
  return null;
}
