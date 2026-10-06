import { expect, it } from 'vitest';
import { progressFeatures } from './progress';
import type { PlaybackFrame, PlaybackPoint } from './types';

it('retains completed Route Parts without joining disconnected geometry', () => {
  const points: PlaybackPoint[] = [
    { coordinate: [1, 1], recordedAtMs: 100 },
    { coordinate: [2, 2], recordedAtMs: 200, breakToNext: 'gps-gap' },
    { coordinate: [4, 4], recordedAtMs: 400 },
    { coordinate: [5, 5], recordedAtMs: 500 },
  ];
  const frame: PlaybackFrame = { state: 'paused', routeTimeMs: 450, durationMs: 500, progress: .9, position: [4.5, 4.5], vertexIndex: 2, segmentRatio: .5, bearing: 0 };
  expect(progressFeatures(points, frame, 450).features.map(feature => feature.geometry.coordinates)).toEqual([[[1, 1], [2, 2]], [[4, 4], [4.5, 4.5]]]);
  expect(progressFeatures(points, { ...frame, vertexIndex: 0, position: [1, 1] }, 0).features).toEqual([]);
});
