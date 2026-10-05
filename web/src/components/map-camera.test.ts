import { describe, expect, it } from "vitest";
import { INITIAL_MAP_ZOOM, initialMapCamera, routeFitOptions } from "./map-camera";

describe("initialMapCamera", () => {
  it("opens at the route start with a practical local zoom", () => {
    expect(
      initialMapCamera({
        routeCoordinates: [[106.700806, 10.776889]],
        startCoordinate: [106.700806, 10.776889],
      }),
    ).toEqual({ center: [106.700806, 10.776889], zoom: INITIAL_MAP_ZOOM });
  });

  it("uses the first route coordinate when start is unavailable", () => {
    expect(
      initialMapCamera({ routeCoordinates: [[106.700806, 10.776889]] }),
    ).toEqual({ center: [106.700806, 10.776889], zoom: INITIAL_MAP_ZOOM });
  });
});

describe("routeFitOptions", () => {
  it("fits the whole route with a short, fixed camera animation", () => {
    expect(routeFitOptions.duration).toBe(350);
  });
});
