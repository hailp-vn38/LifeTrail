import { expect, it, vi } from 'vitest';
import { tripView, tripPlayback } from '../../../test/fixtures/trips';
import { exportDayContent } from './export-day';

it('exports each Route Part separately without fabricated matched-vertex timestamps', () => {
  const view = tripView();
  const parts = [...tripPlayback(), { ...tripPlayback()[0], id: 'other' }];
  const gpx = exportDayContent(view, 'gpx', parts);
  const doc = new DOMParser().parseFromString(gpx, 'application/xml');
  expect(doc.querySelector('parsererror')).toBeNull();
  expect(doc.querySelectorAll('trkseg')).toHaveLength(2);
  expect(doc.querySelectorAll('trkpt time')).toHaveLength(0);
  expect(exportDayContent(view, 'csv', parts)).toContain('"longitude","latitude","recorded_at"');
});

it('prefers canonical playback geometry over simplified display geometry', () => {
  const view = tripView();
  const gpx = exportDayContent(view, 'gpx', tripPlayback());
  const doc = new DOMParser().parseFromString(gpx, 'application/xml');
  // The canonical part keeps all five vertices; display geometry has three.
  expect(doc.querySelectorAll('trkpt')).toHaveLength(5);
});

it('refuses to export a processed day without canonical playback', () => {
  expect(() => exportDayContent(tripView(), 'gpx')).toThrow();
});

it('fetches canonical playback lazily when downloading a processed day', async () => {
  vi.resetModules();
  const getPlayback = vi.fn().mockResolvedValue({ route_parts: tripPlayback() });
  vi.doMock('../../../api/queries/playback.query', () => ({ getPlayback }));
  const { downloadDay } = await import('./export-day');
  const createObjectURL = vi.fn(() => 'blob:day');
  const revokeObjectURL = vi.fn();
  vi.stubGlobal('URL', { ...URL, createObjectURL, revokeObjectURL });

  await downloadDay(tripView(), 'gpx');

  expect(getPlayback).toHaveBeenCalledWith(
    'device-1',
    '2026-10-05',
    '01900000-0000-7000-8000-000000000020',
  );
  expect(createObjectURL).toHaveBeenCalledOnce();
  vi.unstubAllGlobals();
  vi.doUnmock('../../../api/queries/playback.query');
});
