import type { FeatureCollection, Point } from 'geojson';
import { Popup, type Map, type MapLayerMouseEvent, type FilterSpecification, type ExpressionSpecification } from 'maplibre-gl';
import type { TimelineEvent } from '../features/timeline/types';

const SOURCE = 'timeline-event-dots';
export const EVENT_DOTS = 'timeline-event-dots-circle';
const NUMBERS = 'timeline-event-dot-numbers';
const HALO = 'timeline-event-dot-halo';

export function eventDotFeatures(events: TimelineEvent[]): FeatureCollection<Point> {
  return {
    type: 'FeatureCollection',
    features: events.filter(event => event.coordinate).sort((a, b) => (a.recordedAtMs ?? 0) - (b.recordedAtMs ?? 0)).map((event, index) => ({
      type: 'Feature', geometry: { type: 'Point', coordinates: event.coordinate! },
      properties: { eventId: event.id, kind: event.kind, number: index + 1, revealedAtMs: event.recordedAtMs ?? 0 },
    })),
  };
}

export function revealFilter(cursorEpochMs: number | null): FilterSpecification {
  return ['<=', ['get', 'revealedAtMs'], cursorEpochMs ?? Number.MAX_SAFE_INTEGER];
}

export function addEventDots(map: Map, events: TimelineEvent[], cursorEpochMs: number | null) {
  map.addSource(SOURCE, { type: 'geojson', data: eventDotFeatures(events) });
  const filter = revealFilter(cursorEpochMs);
  map.addLayer({ id: HALO, type: 'circle', source: SOURCE, filter,
    paint: { 'circle-radius': 18, 'circle-color': '#60a5fa', 'circle-blur': 0.6, 'circle-opacity': 0 } });
  map.addLayer({ id: EVENT_DOTS, type: 'circle', source: SOURCE, filter,
    paint: { 'circle-radius': 11, 'circle-stroke-color': '#fff', 'circle-stroke-width': 2,
      'circle-color': ['match', ['get', 'kind'], 'start', '#16a34a', 'end', '#dc2626', 'photo', '#9333ea', 'audio', '#f59e0b', '#2563eb'] } });
  if (map.getStyle().glyphs) {
    map.addLayer({ id: NUMBERS, type: 'symbol', source: SOURCE, filter,
      layout: { 'text-field': ['to-string', ['get', 'number']], 'text-size': 12, 'text-allow-overlap': true, 'text-ignore-placement': true },
      paint: { 'text-color': '#fff' } });
  }
}

export function revealEventDots(map: Map, cursorEpochMs: number | null) {
  for (const layer of [EVENT_DOTS, NUMBERS, HALO]) {
    if (map.getLayer(layer)) map.setFilter(layer, revealFilter(cursorEpochMs));
  }
}

export function highlightEventDot(map: Map, selectedId: string | null) {
  const selected: ExpressionSpecification = ['==', ['get', 'eventId'], selectedId ?? '__none__'];
  map.setPaintProperty(EVENT_DOTS, 'circle-radius', ['case', selected, 14, 11]);
  map.setPaintProperty(EVENT_DOTS, 'circle-stroke-width', ['case', selected, 4, 2]);
  map.setPaintProperty(HALO, 'circle-opacity', ['case', selected, 0.7, 0]);
}

/** Tooltip uses text nodes because place names can originate outside the app. */
export function bindEventDots(map: Map, events: TimelineEvent[], select: (id: string) => void) {
  let popup: Popup | undefined;
  let shownEvent: TimelineEvent | undefined;
  const close = () => { popup?.remove(); popup = undefined; shownEvent = undefined; };
  const show = (event: TimelineEvent) => {
    close();
    if (!event.coordinate) return;
    const content = document.createElement('div');
    content.className = 'event-dot-tooltip';
    const title = document.createElement('strong');
    title.textContent = `${({ stop: '⏸', start: '⚑', end: '⚑', photo: '▣', audio: '♫', trip: '↗', gap: '' })[event.kind]} ${event.title}`;
    const time = document.createElement('p');
    time.textContent = [event.timeLabel ?? event.subtitle, event.durationLabel].filter(Boolean).join(' · ');
    content.append(title, time);
    shownEvent = event;
    popup = new Popup({ closeButton: false, offset: 18 }).setLngLat(event.coordinate).setDOMContent(content).addTo(map);
  };
  const find = (event: MapLayerMouseEvent) => events.find(item => item.id === event.features?.[0]?.properties?.eventId);
  map.on('mouseenter', EVENT_DOTS, event => { map.getCanvas().style.cursor = 'pointer'; const item = find(event); if (item) show(item); });
  map.on('mouseleave', EVENT_DOTS, () => { map.getCanvas().style.cursor = ''; close(); });
  map.on('click', EVENT_DOTS, event => { const item = find(event); if (item) { select(item.id); show(item); } });
  return {
    showSelected: (id: string | null) => { const event = events.find(item => item.id === id); if (event) show(event); else close(); },
    reveal: (cursor: number) => { if (shownEvent?.recordedAtMs !== undefined && shownEvent.recordedAtMs > cursor) close(); },
    dispose: close,
  };
}
