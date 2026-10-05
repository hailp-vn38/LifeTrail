import { afterEach, describe, expect, it, vi } from "vitest";
import { EMPTY_MAP_STYLE, resolveMapStyle } from "./map-style";
import { VERSATILES_STYLE_PRESETS } from "../map/versatiles-presets";

afterEach(() => vi.unstubAllEnvs());

describe("resolveMapStyle", () => {
  it.each(VERSATILES_STYLE_PRESETS)("loads $id without a MapTiler key", (preset) => {
    expect(resolveMapStyle({ presetId: preset.id })).toBe(preset.url);
    expect(resolveMapStyle({ presetId: preset.id, mapTilerKey: "test-key" })).toBe(preset.url);
  });

  it("keeps local styles under the Vite deployment base", () => {
    vi.stubEnv("BASE_URL", "/lifetrail/");
    expect(resolveMapStyle({ presetId: "versatiles-natural-dark" })).toBe(
      "/lifetrail/map-styles/versatiles/natural-dark.json",
    );
  });
  it("resolves a selected preset before the configured default", () => {
    expect(resolveMapStyle({
      presetId: "streets-3d", styleUrl: "https://maps.example.test/custom.json", mapTilerKey: "test-key",
    })).toBe("https://api.maptiler.com/maps/streets-v4/style.json?key=test-key");
    expect(resolveMapStyle({ presetId: "streets-3d" })).toBe(EMPTY_MAP_STYLE);
  });
  it("uses a local style instead of requesting MapTiler without a key", () => {
    expect(resolveMapStyle({})).toBe(EMPTY_MAP_STYLE);
    expect(
      resolveMapStyle({
        styleUrl: "https://api.maptiler.com/maps/hybrid/style.json",
      }),
    ).toBe(EMPTY_MAP_STYLE);
    expect(resolveMapStyle({ styleUrl: "" })).toBe(EMPTY_MAP_STYLE);
    expect(
      resolveMapStyle({
        styleUrl:
          "https://api.maptiler.com/maps/hybrid/style.json?key=replace-me",
      }),
    ).toBe(EMPTY_MAP_STYLE);
  });

  it("adds the configured MapTiler key to the default style URL", () => {
    expect(resolveMapStyle({ mapTilerKey: "test-key" })).toBe(
      "https://api.maptiler.com/maps/hybrid/style.json?key=test-key",
    );
  });

  it("preserves a configured non-MapTiler style URL", () => {
    const styleUrl = "https://maps.example.test/style.json";

    expect(resolveMapStyle({ styleUrl })).toBe(styleUrl);
  });
});
