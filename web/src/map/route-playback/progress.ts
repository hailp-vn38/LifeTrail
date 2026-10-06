import type { FeatureCollection, LineString } from 'geojson';
import type { PlaybackFrame, PlaybackPoint, MapCoordinate } from './types';

/** Retain completed parts while never drawing a connector across a break. */
export function progressFeatures(points: PlaybackPoint[], frame: PlaybackFrame, cursorEpochMs: number): FeatureCollection<LineString> {
  const lines: MapCoordinate[][] = [];
  let line: MapCoordinate[] = [];
  for (let index = 0; index <= frame.vertexIndex && index < points.length; index++) {
    const point = points[index];
    if (point.recordedAtMs > cursorEpochMs) break;
    line.push(point.coordinate);
    if (point.breakToNext && index < frame.vertexIndex) { if (line.length >= 2) lines.push(line); line = []; }
  }
  if (cursorEpochMs >= points[0].recordedAtMs) line.push(frame.position);
  if (line.length >= 2) lines.push(line);
  return { type: 'FeatureCollection', features: lines.map(coordinates => ({ type: 'Feature', properties: {}, geometry: { type: 'LineString', coordinates } })) };
}
