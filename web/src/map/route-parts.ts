/**
 * Daily Map Route Part rendering.
 *
 * The Daily Snapshot carries `display_geometry`: simplified, visualization-only
 * coordinates scoped to the day. Distance and playback never read these
 * coordinates, so nothing here drives the playback clock or the export.
 */
import type { FeatureCollection, LineString } from "geojson";
import { LngLatBounds, type Map, type MapLayerMouseEvent, type ExpressionSpecification } from "maplibre-gl";
import type { DailyView } from "../api/queries/daily-view.query";
import { tripActivities, tripDisplayParts, type DailyRoutePart } from "../features/activity/model";
import { routeFitOptions } from "../components/map-camera";

export const PART_SOURCE = "activity-route-parts";
export const PART_LINE = "trip-route-parts";

function lineFeatures(dailyView: DailyView): FeatureCollection<LineString> {
  const features = (dailyView.route_parts ?? []).map((part) => ({
    type: "Feature" as const,
    // Selecting a part selects its Trip Timeline item.
    properties: { eventId: part.trip_id, partId: part.id },
    geometry: { type: "LineString" as const, coordinates: part.display_geometry.coordinates },
  }));
  return { type: "FeatureCollection", features };
}

export function addRoutePartLayers(map: Map, dailyView: DailyView): void {
  map.addSource(PART_SOURCE, { type: "geojson", data: lineFeatures(dailyView) });
  map.addLayer({
    id: PART_LINE, type: "line", source: PART_SOURCE,
    paint: { "line-color": "#0f766e", "line-width": 4, "line-opacity": 0.85 },
  });
}

export function highlightRoutePart(map: Map, selectedId: string | null): void {
  const selected: ExpressionSpecification = ["==", ["get", "eventId"], selectedId ?? "__none__"];
  map.setPaintProperty(PART_LINE, "line-width", ["case", selected, 8, 4]);
  map.setPaintProperty(PART_LINE, "line-color", ["case", selected, "#f97316", "#0f766e"]);
  map.setPaintProperty(PART_LINE, "line-opacity", ["case", selected, 1, 0.85]);
}

/** Fit the visible geometry of the selected Trip. */
export function focusTripPart(map: Map, dailyView: DailyView, selectedId: string | null): boolean {
  const trip = tripActivities(dailyView).find((item) => item.id === selectedId);
  if (!trip) return false;
  const parts = tripDisplayParts(dailyView, trip.id);
  const coordinates = parts.flatMap((part) => part.display_geometry.coordinates);
  const [first] = coordinates;
  if (!first) return false;
  const bounds = coordinates.reduce(
    (current, coordinate) => current.extend(coordinate as [number, number]),
    new LngLatBounds(first as [number, number], first as [number, number]),
  );
  map.fitBounds(bounds, routeFitOptions);
  return true;
}

export function bindRoutePartSelection(map: Map, select: (id: string) => void): void {
  map.on("click", PART_LINE, (event: MapLayerMouseEvent) => {
    const id = event.features?.[0]?.properties?.eventId;
    if (typeof id === "string") select(id);
  });
}

/** Visible display geometry of one Trip, for callers that need its coordinates. */
export function visibleTripCoordinates(dailyView: DailyView, tripId: string): number[][] {
  return tripDisplayParts(dailyView, tripId).flatMap(
    (part: DailyRoutePart) => part.display_geometry.coordinates,
  );
}
