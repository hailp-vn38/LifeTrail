import type { Map } from 'maplibre-gl';
import { LAYER_START, LAYER_END } from './route-playback/layers';

/** Daily Map uses a single event-dot source; legacy marker layers stay hidden. */
export function prepareDailyLayers(map: Map, canPlay: boolean) {
  for (const layer of [LAYER_START, LAYER_END, 'stop-markers', 'stop-radius', 'stop-radius-outline']) {
    if (map.getLayer(layer)) map.setLayoutProperty(layer, 'visibility', 'none');
  }
  if (!canPlay) return;
  for (const layer of ['daily-route-full-line', 'trip-route-parts']) {
    if (!map.getLayer(layer)) continue;
    map.setPaintProperty(layer, 'line-color', '#94a3b8');
    map.setPaintProperty(layer, 'line-opacity', 0.6);
    map.setPaintProperty(layer, 'line-dasharray', [2, 2]);
  }
}
