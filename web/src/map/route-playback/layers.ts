import type { Feature, FeatureCollection, LineString, Point } from "geojson";
import type { Map as MapLibreMap } from "maplibre-gl";
import type { DailyView } from "../../api/queries/daily-view.query";
import { headingPuckRing } from "./geometry";
import type { MapCoordinate, PlaybackPoint } from "./types";

const SOURCE_FULL = "daily-route-full";
export const SOURCE_PROGRESS = "daily-route-progress";
const SOURCE_START = "daily-route-start";
const SOURCE_END = "daily-route-end";
export const SOURCE_CURRENT = "daily-route-current";

const LAYER_FULL = "daily-route-full-line";
const LAYER_PROGRESS = "daily-route-progress-line";
export const LAYER_START = "daily-route-start-point";
export const LAYER_END = "daily-route-end-point";
const LAYER_PROGRESS_GLOW = "daily-route-progress-glow";
const LAYER_CURRENT_DOT = "daily-route-current-dot";
const LAYER_CURRENT_ARROW = "daily-route-current-arrow";

export function lineFeature(coordinates: MapCoordinate[]): Feature<LineString> {
  return {
    type: "Feature",
    properties: {},
    geometry: { type: "LineString", coordinates },
  };
}
/**
 * Start/end point tagged with the timeline event id. The shared selection
 * store (`selectedEventId`) is the only bridge between Timeline and Map:
 * clicking a map point writes the id, clicking a timeline item writes the
 * id, and each side reacts independently.
 */
function eventPointFeature(feature: Feature<Point>, eventId: string): Feature<Point> {
  return {
    ...feature,
    properties: { ...(feature.properties ?? {}), eventId },
  };
}

/**
 * Heading-aware current position marker (navigation puck) matching the
 * course-up design: a blue dot with a white arrow pointing along the
 * bearing. The arrow is a flat triangle on the map, so under the course-up
 * camera it always points up-screen along the direction of travel. No
 * sprite or glyph assets are needed.
 */
export function puckFeatures(position: MapCoordinate, bearing: number): FeatureCollection {
  return {
    type: "FeatureCollection",
    features: [
      {
        type: "Feature",
        properties: { kind: "dot" },
        geometry: { type: "Point", coordinates: position },
      },
      {
        type: "Feature",
        properties: { kind: "arrow" },
        geometry: {
          type: "Polygon",
          coordinates: [headingPuckRing(position, bearing, 22)],
        },
      },
    ],
  };
}

export function addRouteLayers(map: MapLibreMap, dailyView: DailyView, playbackPoints: PlaybackPoint[]) {
  const { route, start, end } = dailyView;
  const canPlay = playbackPoints.length > 0;

  if (route) {
    map.addSource(SOURCE_FULL, { type: "geojson", data: route });
    map.addLayer({
      id: LAYER_FULL,
      type: "line",
      source: SOURCE_FULL,
      paint: { "line-color": "#2563eb", "line-width": 4, "line-opacity": 0.35 },
    });
  }
  if (canPlay && playbackPoints.length > 0) {
    const startCoordinate = playbackPoints[0].coordinate;
    map.addSource(SOURCE_PROGRESS, {
      type: "geojson",
      data: lineFeature([startCoordinate, startCoordinate]),
    });
    map.addLayer({
      id: LAYER_PROGRESS_GLOW,
      type: "line",
      source: SOURCE_PROGRESS,
      paint: {
        "line-color": "#60a5fa",
        "line-width": 12,
        "line-opacity": 0.45,
        "line-blur": 5,
      },
    });
    map.addLayer({
      id: LAYER_PROGRESS,
      type: "line",
      source: SOURCE_PROGRESS,
      paint: { "line-color": "#2563eb", "line-width": 5.5, "line-opacity": 1 },
    });
    map.addSource(SOURCE_CURRENT, {
      type: "geojson",
      data: puckFeatures(startCoordinate, 0),
    });
    map.addLayer({
      id: LAYER_CURRENT_DOT,
      type: "circle",
      source: SOURCE_CURRENT,
      filter: ["==", ["get", "kind"], "dot"],
      paint: {
        "circle-radius": 13,
        "circle-color": "#2563eb",
        "circle-stroke-width": 3,
        "circle-stroke-color": "#ffffff",
      },
    });
    map.addLayer({
      id: LAYER_CURRENT_ARROW,
      type: "fill",
      source: SOURCE_CURRENT,
      filter: ["==", ["get", "kind"], "arrow"],
      paint: { "fill-color": "#ffffff" },
    });
  }
  if (start) {
    map.addSource(SOURCE_START, {
      type: "geojson",
      data: eventPointFeature(start, "start"),
    });
    map.addLayer({
      id: LAYER_START,
      type: "circle",
      source: SOURCE_START,
      paint: {
        "circle-radius": 7,
        "circle-color": "#16a34a",
        "circle-stroke-width": 2,
        "circle-stroke-color": "#ffffff",
      },
    });
  }
  if (end) {
    map.addSource(SOURCE_END, {
      type: "geojson",
      data: eventPointFeature(end, "end"),
    });
    map.addLayer({
      id: LAYER_END,
      type: "circle",
      source: SOURCE_END,
      paint: {
        "circle-radius": 7,
        "circle-color": "#dc2626",
        "circle-stroke-width": 2,
        "circle-stroke-color": "#ffffff",
      },
    });
  }
}
