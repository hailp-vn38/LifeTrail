import { expect, it } from 'vitest';
import { tripView } from '../../../test/fixtures/trips';
import { exportDayContent } from './export-day';

it('exports each Route Part separately without fabricated matched-vertex timestamps', () => {
  const view = tripView();
  view.route_parts!.push({ ...view.route_parts![0], id: 'other' });
  const gpx = exportDayContent(view, 'gpx');
  const doc = new DOMParser().parseFromString(gpx, 'application/xml');
  expect(doc.querySelector('parsererror')).toBeNull();
  expect(doc.querySelectorAll('trkseg')).toHaveLength(2);
  expect(doc.querySelectorAll('trkpt time')).toHaveLength(0);
  expect(exportDayContent(view, 'csv')).toContain('"longitude","latitude","recorded_at"');
});
