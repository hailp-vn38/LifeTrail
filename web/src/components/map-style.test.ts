import { describe, expect, it } from "vitest";
import { EMPTY_MAP_STYLE, resolveMapStyle } from "./map-style";

describe("resolveMapStyle", () => {
  it("uses a local style instead of requesting MapTiler without a key", () => {
    expect(resolveMapStyle({})).toBe(EMPTY_MAP_STYLE);
    expect(
      resolveMapStyle({
        styleUrl: "https://api.maptiler.com/maps/streets-v2/style.json",
      }),
    ).toBe(EMPTY_MAP_STYLE);
    expect(resolveMapStyle({ styleUrl: "" })).toBe(EMPTY_MAP_STYLE);
    expect(
      resolveMapStyle({
        styleUrl:
          "https://api.maptiler.com/maps/streets-v2/style.json?key=replace-me",
      }),
    ).toBe(EMPTY_MAP_STYLE);
  });

  it("adds the configured MapTiler key to the default style URL", () => {
    expect(resolveMapStyle({ mapTilerKey: "test-key" })).toBe(
      "https://api.maptiler.com/maps/streets-v2/style.json?key=test-key",
    );
  });

  it("preserves a configured non-MapTiler style URL", () => {
    const styleUrl = "https://maps.example.test/style.json";

    expect(resolveMapStyle({ styleUrl })).toBe(styleUrl);
  });
});
