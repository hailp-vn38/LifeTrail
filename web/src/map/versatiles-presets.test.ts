import { validateStyleMin } from "@maplibre/maplibre-gl-style-spec";
import type { StyleSpecification } from "maplibre-gl";
import { describe, expect, it } from "vitest";
import { VERSATILES_STYLE_PRESETS } from "./versatiles-presets";

const assets = import.meta.glob<StyleSpecification>("../../public/map-styles/versatiles/*.json", {
  eager: true,
  import: "default",
});

describe("vendored VersaTiles styles", () => {
  it.each(VERSATILES_STYLE_PRESETS)("ships a valid, attributed style for $id", (preset) => {
    const style = assets[`../../public${preset.url}`];
    expect(style).toBeDefined();
    expect(validateStyleMin(style)).toEqual([]);
    expect(style.projection).toEqual({ type: "mercator" });
    expect(style.layers.length).toBeGreaterThan(0);
    expect(style.glyphs).toMatch(/^https:\/\/tiles\.versatiles\.org\//);
    for (const source of Object.values(style.sources)) {
      expect(source.type).toBe("vector");
      if (source.type !== "vector") continue;
      expect(source.attribution).toContain("OpenStreetMap");
      expect(source.url).toBeUndefined();
      expect(source.tiles?.length).toBeGreaterThan(0);
      for (const tile of source.tiles ?? []) expect(tile).toMatch(/^https:\/\//);
    }
  });
});
