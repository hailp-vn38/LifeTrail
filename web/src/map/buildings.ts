import type { FillExtrusionLayerSpecification, StyleSpecification } from "maplibre-gl";

export const BUILDINGS_LAYER = "lifetrail-3d-buildings";

function existingBuildings(style: StyleSpecification) {
  return style.layers.filter((layer) => layer.type === "fill-extrusion" &&
    /building/i.test(`${layer.id} ${layer["source-layer"] ?? ""}`));
}

/** Discover footprints from the loaded style; never guess vector source names. */
export function buildingLayer(style: StyleSpecification): FillExtrusionLayerSpecification | null {
  if (existingBuildings(style).length > 0) return null;
  const footprint = style.layers.find((layer) =>
    layer.type === "fill" &&
    /building/i.test(layer["source-layer"] ?? "") &&
    style.sources[layer.source]?.type === "vector",
  );
  if (!footprint || footprint.type !== "fill") return null;
  return {
    id: BUILDINGS_LAYER,
    type: "fill-extrusion" as const,
    source: footprint.source,
    "source-layer": footprint["source-layer"],
    minzoom: Math.max(14, footprint.minzoom ?? 0),
    filter: footprint.filter,
    paint: {
      "fill-extrusion-color": "#cbd5e1",
      "fill-extrusion-opacity": 0.8,
      "fill-extrusion-height": ["max", 0, ["coalesce", ["to-number", ["get", "render_height"], null], ["to-number", ["get", "height"], null], 0]],
      "fill-extrusion-base": ["max", 0, ["coalesce", ["to-number", ["get", "render_min_height"], null], ["to-number", ["get", "min_height"], null], 0]],
    },
  };
}

interface BuildingMap {
  getStyle(): StyleSpecification;
  addLayer(layer: FillExtrusionLayerSpecification, beforeId?: string): unknown;
  setLayoutProperty(id: string, name: string, value: unknown): unknown;
}

export function addBuildings(map: BuildingMap): void {
  const style = map.getStyle();
  for (const existing of existingBuildings(style)) {
    if (existing.layout?.visibility === "none") {
      map.setLayoutProperty(existing.id, "visibility", "visible");
    }
  }
  const layer = buildingLayer(style);
  if (!layer) return;
  const labels = style.layers.find((candidate) => candidate.type === "symbol");
  map.addLayer(layer, labels?.id);
}
