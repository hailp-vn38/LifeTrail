import type { StyleSpecification } from "maplibre-gl";
import { describe, expect, it, vi } from "vitest";
import { addBuildings, buildingLayer } from "./buildings";

function styleFixture(): StyleSpecification {
  return {
    version: 8,
    sources: { city: { type: "vector", url: "https://example.com/tiles.json" } },
    layers: [{
      id: "footprints", type: "fill", source: "city", "source-layer": "city_buildings",
      filter: ["==", ["get", "underground"], false],
    }],
  };
}

describe("buildingLayer", () => {
  it("enables building extrusions supplied but hidden by the basemap", () => {
    const style = styleFixture();
    style.layers.push({
      id: "Building 3D", type: "fill-extrusion", source: "city",
      "source-layer": "city_buildings", layout: { visibility: "none" },
    });
    const map = { getStyle: () => style, addLayer: vi.fn(), setLayoutProperty: vi.fn() };
    addBuildings(map);
    expect(map.setLayoutProperty).toHaveBeenCalledWith("Building 3D", "visibility", "visible");
    expect(map.addLayer).not.toHaveBeenCalled();
  });

  it("does not mistake other extrusions for buildings", () => {
    const style = styleFixture();
    style.layers.push({ id: "bridges", type: "fill-extrusion", source: "city", "source-layer": "bridges" });
    expect(buildingLayer(style)).not.toBeNull();
  });
  it("uses the actual footprint source, source layer and filter", () => {
    const style = styleFixture();
    expect(buildingLayer(style)).toMatchObject({
      type: "fill-extrusion", source: "city", "source-layer": "city_buildings",
      filter: ["==", ["get", "underground"], false],
    });
  });

  it("reuses styles with existing extrusions instead of duplicating buildings", () => {
    const style = styleFixture();
    style.layers.push({ id: "existing-3d", type: "fill-extrusion", source: "city", "source-layer": "city_buildings" });
    expect(buildingLayer(style)).toBeNull();
  });

  it("skips raster-only and empty styles", () => {
    expect(buildingLayer({ version: 8, sources: {}, layers: [] })).toBeNull();
    const style = styleFixture();
    style.sources.city = { type: "raster", tiles: ["https://example.com/{z}/{x}/{y}"] };
    expect(buildingLayer(style)).toBeNull();
  });
});
