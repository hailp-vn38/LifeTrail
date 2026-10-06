import type { FeatureCollection, Point, Polygon } from "geojson";
import type { Map, MapLayerMouseEvent, ExpressionSpecification } from "maplibre-gl";
import type { DailyView } from "../api/queries/daily-view.query";

export const STOP_SOURCE = "activity-stops";
export const STOP_MARKERS = "stop-markers";
export const STOP_RADIUS = "stop-radius";

/** A geographic disk in meters, independent of map zoom. */
function disk(center: number[], radius: number): number[][] {
  const [lon, lat] = center.map((value) => value * Math.PI / 180);
  const angle = radius / 6_371_000;
  const ring = Array.from({ length: 64 }, (_, i) => {
    const bearing = i * Math.PI / 32;
    const latitude = Math.asin(Math.sin(lat) * Math.cos(angle) + Math.cos(lat) * Math.sin(angle) * Math.cos(bearing));
    const longitude = lon + Math.atan2(Math.sin(bearing) * Math.sin(angle) * Math.cos(lat), Math.cos(angle) - Math.sin(lat) * Math.sin(latitude));
    return [longitude * 180 / Math.PI, latitude * 180 / Math.PI];
  });
  return [...ring, ring[0]];
}

export function addStopLayers(map: Map, dailyView: DailyView): void {
  const features: FeatureCollection<Point | Polygon> = { type: "FeatureCollection", features: [] };
  for (const stop of dailyView.timeline ?? []) {
    const properties = { eventId: stop.id };
    features.features.push({ type: "Feature", properties, geometry: { type: "Point", coordinates: stop.center } });
    features.features.push({ type: "Feature", properties, geometry: { type: "Polygon", coordinates: [disk(stop.center, stop.radius_m)] } });
  }
  map.addSource(STOP_SOURCE, { type: "geojson", data: features });
  map.addLayer({ id: STOP_RADIUS, type: "fill", source: STOP_SOURCE,
    filter: ["==", ["geometry-type"], "Polygon"], paint: { "fill-color": "#2563eb", "fill-opacity": 0.15 } });
  map.addLayer({ id: "stop-radius-outline", type: "line", source: STOP_SOURCE,
    filter: ["==", ["geometry-type"], "Polygon"], paint: { "line-color": "#2563eb", "line-width": 2 } });
  map.addLayer({ id: STOP_MARKERS, type: "circle", source: STOP_SOURCE,
    filter: ["==", ["geometry-type"], "Point"],
    paint: { "circle-color": "#2563eb", "circle-radius": 7, "circle-stroke-color": "#fff", "circle-stroke-width": 2 } });
}

export function highlightStop(map: Map, selectedId: string | null): void {
  const selected: ExpressionSpecification = ["==", ["get", "eventId"], selectedId ?? "__none__"];
  map.setPaintProperty(STOP_MARKERS, "circle-radius", ["case", selected, 11, 7]);
  map.setPaintProperty(STOP_RADIUS, "fill-opacity", ["case", selected, 0.35, 0.15]);
  map.setPaintProperty("stop-radius-outline", "line-width", ["case", selected, 4, 2]);
}

export function focusStop(map: Map, dailyView: DailyView, selectedId: string | null): boolean {
  const stop = dailyView.timeline?.find((item) => item.id === selectedId);
  if (!stop) return false;
  // Reserve about 200 screen pixels for the disk, with a useful zoom for zero jitter.
  const zoom = Math.min(18, Math.max(0, Math.log2(156543 * Math.cos(stop.center[1] * Math.PI / 180) * 100 / Math.max(10, stop.radius_m))));
  map.easeTo({ center: [stop.center[0], stop.center[1]], zoom, duration: 600 });
  return true;
}

export function bindStopSelection(map: Map, select: (id: string) => void): void {
  const onClick = (event: MapLayerMouseEvent) => {
    const id = event.features?.[0]?.properties?.eventId;
    if (typeof id === "string") select(id);
  };
  map.on("click", STOP_MARKERS, onClick);
  map.on("click", STOP_RADIUS, onClick);
}
