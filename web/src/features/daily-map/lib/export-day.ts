import type { DailyView } from '../../../api/queries/daily-view.query';
import type { RoutePart } from '../../activity/model';
import { getPlayback } from '../../../api/queries/playback.query';
import { buildTimelineEvents } from '../../timeline/events';

const csvCell = (value: unknown) => `"${String(value ?? '').replaceAll('"', '""')}"`;
const xml = (value: string) => value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;').replaceAll("'", '&apos;');

export function exportDayContent(view: DailyView, format: 'csv' | 'gpx', playbackParts?: RoutePart[]): string {
  // Canonical playback geometry is the export source. A processed day without it
  // is an error, not a silent downgrade to the simplified display geometry.
  if (view.route_parts?.length && !playbackParts?.length) {
    throw new Error('Dữ liệu playback chưa sẵn sàng để xuất.');
  }
  const lines = playbackParts?.length
    ? playbackParts.map(part => ({ coordinates: part.geometry.coordinates, timestamps: [] as string[] }))
    : view.route
    ? [{ coordinates: view.route.geometry.coordinates, timestamps: Array.isArray(view.route.properties.timestamps) ? view.route.properties.timestamps as string[] : [] }]
    : [];
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

export async function downloadDay(view: DailyView, format: 'csv' | 'gpx') {
  // Canonical playback geometry is the export source. It is fetched lazily here
  // so a Daily Map visit never pays for it unless the user exports.
  const parts = await loadExportParts(view);
  const url = URL.createObjectURL(new Blob([exportDayContent(view, format, parts)], { type: format === 'csv' ? 'text/csv;charset=utf-8' : 'application/gpx+xml' }));
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = `lifetrail-${view.device_id}-${view.date}.${format}`;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/**
 * Canonical parts for export. Raw days carry their geometry directly; processed
 * days require the Playback resource, and a failed fetch is surfaced, not
 * silently downgraded to display geometry.
 */
async function loadExportParts(view: DailyView): Promise<RoutePart[] | undefined> {
  if (!view.route_parts?.length) return undefined;
  const playback = await getPlayback(view.device_id, view.date, view.provenance?.manifest_version);
  return playback.route_parts;
}
