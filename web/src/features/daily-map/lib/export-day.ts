import type { DailyView } from '../../../api/queries/daily-view.query';
import { buildTimelineEvents } from '../../timeline/events';

const csvCell = (value: unknown) => `"${String(value ?? '').replaceAll('"', '""')}"`;
const xml = (value: string) => value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;').replaceAll("'", '&apos;');

export function exportDayContent(view: DailyView, format: 'csv' | 'gpx'): string {
  const lines = view.route_parts?.length ? view.route_parts.map(part => ({ coordinates: part.geometry.coordinates, timestamps: [] as string[] })) : view.route ? [{ coordinates: view.route.geometry.coordinates, timestamps: Array.isArray(view.route.properties.timestamps) ? view.route.properties.timestamps as string[] : [] }] : [];
  const events = buildTimelineEvents(view).filter(event => event.coordinate);
  if (format === 'csv') {
    const rows: unknown[][] = [['type', 'id', 'longitude', 'latitude', 'recorded_at', 'place']];
    lines.forEach((line, part) => line.coordinates.forEach((point, index) => rows.push(['route', `${part}:${index}`, point[0], point[1], line.timestamps[index], ''])));
    events.forEach(event => rows.push([event.kind, event.id, ...event.coordinate!, event.recordedAtMs === undefined ? '' : new Date(event.recordedAtMs).toISOString(), event.title]));
    return '\uFEFF' + rows.map(row => row.map(csvCell).join(',')).join('\r\n');
  }
  const waypoint = events.map(event => `<wpt lat="${event.coordinate![1]}" lon="${event.coordinate![0]}"><name>${xml(event.title)}</name>${event.recordedAtMs === undefined ? '' : `<time>${new Date(event.recordedAtMs).toISOString()}</time>`}</wpt>`).join('');
  const segments = lines.map(line => `<trkseg>${line.coordinates.map((point, index) => `<trkpt lat="${point[1]}" lon="${point[0]}">${line.timestamps[index] ? `<time>${xml(line.timestamps[index])}</time>` : ''}</trkpt>`).join('')}</trkseg>`).join('');
  return `<?xml version="1.0" encoding="UTF-8"?><gpx version="1.1" creator="LifeTrail" xmlns="http://www.topografix.com/GPX/1/1">${waypoint}<trk><name>${xml(view.date)}</name>${segments}</trk></gpx>`;
}

export function downloadDay(view: DailyView, format: 'csv' | 'gpx') {
  const url = URL.createObjectURL(new Blob([exportDayContent(view, format)], { type: format === 'csv' ? 'text/csv;charset=utf-8' : 'application/gpx+xml' }));
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = `lifetrail-${view.device_id}-${view.date}.${format}`;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
