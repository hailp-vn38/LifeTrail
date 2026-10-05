import { describe, expect, it } from "vitest";
import {
  bearingBetween,
  destinationPoint,
  haversineDistanceM,
  headingPuckRing,
  normalizeBearing,
  pointAheadOnRoute,
  progressCoordinates,
  routeBounds,
} from "./geometry";
import type { MapCoordinate, PlaybackPoint } from "./types";

function point(coordinate: MapCoordinate, recordedAtMs: number): PlaybackPoint {
  return { coordinate, recordedAtMs };
}

describe("bearingBetween", () => {
  it("points north for increasing latitude", () => {
    expect(bearingBetween([106.7, 10.776], [106.7, 10.786])).toBeCloseTo(0, 1);
  });

  it("points east for increasing longitude", () => {
    expect(bearingBetween([106.7, 10.776], [106.71, 10.776])).toBeCloseTo(90, 1);
  });

  it("points south for decreasing latitude", () => {
    expect(bearingBetween([106.7, 10.786], [106.7, 10.776])).toBeCloseTo(180, 1);
  });

  it("points west for decreasing longitude", () => {
    expect(bearingBetween([106.71, 10.776], [106.7, 10.776])).toBeCloseTo(270, 1);
  });

  it("returns null for the same coordinate", () => {
    expect(bearingBetween([106.7, 10.776], [106.7, 10.776])).toBeNull();
  });

  it("returns null for jitter below the distance threshold (~1m)", () => {
    // ~1.1m north: real GPS noise, not travel direction.
    expect(bearingBetween([106.7, 10.776], [106.7, 10.77601])).toBeNull();
  });

  it("trusts movement above the distance threshold (~50m)", () => {
    const bearing = bearingBetween([106.7, 10.776], [106.7, 10.77645]);
    expect(bearing).not.toBeNull();
    expect(bearing).toBeCloseTo(0, 0);
  });
});

describe("normalizeBearing", () => {
  it("wraps bearings into [0, 360)", () => {
    expect(normalizeBearing(-45)).toBe(315);
    expect(normalizeBearing(360)).toBe(0);
    expect(normalizeBearing(725)).toBe(5);
    expect(normalizeBearing(90)).toBe(90);
  });
});

describe("haversineDistanceM", () => {
  it("measures roughly 111km per degree of latitude", () => {
    expect(haversineDistanceM([0, 0], [0, 1])).toBeCloseTo(111_194.9, 0);
  });

  it("returns 0 for identical points", () => {
    expect(haversineDistanceM([106.7, 10.776], [106.7, 10.776])).toBe(0);
  });
});

describe("routeBounds", () => {
  it("covers all route points", () => {
    const bounds = routeBounds([
      point([106.7, 10.776], 0),
      point([106.7015, 10.7774], 1_000),
      point([106.7005, 10.7764], 2_000),
    ]);
    expect(bounds).toEqual({ min: [106.7, 10.776], max: [106.7015, 10.7774] });
  });

  it("returns null without points", () => {
    expect(routeBounds([])).toBeNull();
  });
});

describe("progressCoordinates", () => {
  const pts = [
    point([106.7, 10.776], 0),
    point([106.7005, 10.7764], 3_000),
    point([106.701, 10.777], 25_000),
  ];

  it("includes completed vertices plus the interpolated position", () => {
    const coords = progressCoordinates(pts, 1, [106.7008, 10.7767]);
    expect(coords).toEqual([
      [106.7, 10.776],
      [106.7005, 10.7764],
      [106.7008, 10.7767],
    ]);
  });

  it("does not duplicate the vertex when the ratio is 0", () => {
    const coords = progressCoordinates(pts, 1, [106.7005, 10.7764]);
    expect(coords).toEqual([
      [106.7, 10.776],
      [106.7005, 10.7764],
    ]);
  });
});

describe("destinationPoint", () => {
  it("travels north for bearing 0", () => {
    const [lon, lat] = destinationPoint([106.7, 10.776], 0, 111_194.9);
    expect(lon).toBeCloseTo(106.7, 4);
    expect(lat).toBeCloseTo(11.776, 3);
  });

  it("travels east for bearing 90", () => {
    const [lon, lat] = destinationPoint([106.7, 10.776], 90, 10_000);
    expect(lon).toBeGreaterThan(106.7);
    expect(lat).toBeCloseTo(10.776, 4);
  });

  it("stays put for zero distance", () => {
    expect(destinationPoint([106.7, 10.776], 45, 0)).toEqual([106.7, 10.776]);
  });
});

describe("pointAheadOnRoute", () => {
  // Straight north line: each degree of latitude is ~111.2 km.
  const line: PlaybackPoint[] = [0, 1_000, 2_000, 3_000].map((recordedAtMs, index) => ({
    coordinate: [106.7, 10.776 + index * 0.001] as MapCoordinate,
    recordedAtMs,
  }));
  const start: MapCoordinate = [106.7, 10.776];

  it("walks forward along the route from the current position", () => {
    // 30 m ahead from the start of the first segment.
    const ahead = pointAheadOnRoute(line, 0, start, 30);
    expect(ahead[0]).toBeCloseTo(106.7, 6);
    expect(ahead[1]).toBeCloseTo(10.776 + 30 / 111_194.9, 6);
  });

  it("continues across vertices", () => {
    // 150 m ahead crosses the first vertex (~111.2 m away).
    const ahead = pointAheadOnRoute(line, 0, start, 150);
    expect(ahead[1]).toBeCloseTo(10.776 + 150 / 111_194.9, 6);
  });

  it("clamps to the final coordinate when the route ends sooner", () => {
    const ahead = pointAheadOnRoute(line, 2, [106.7, 10.778], 10_000);
    expect(ahead).toEqual([106.7, 10.779]);
  });

  it("returns the position for non-positive distances", () => {
    expect(pointAheadOnRoute(line, 0, start, 0)).toEqual(start);
  });
});

describe("headingPuckRing", () => {
  it("builds a closed triangle pointing along the bearing", () => {
    const ring = headingPuckRing([106.7, 10.776], 0);
    expect(ring).toHaveLength(4);
    expect(ring[0]).toEqual(ring[3]);
    const [tip, left, right] = ring;
    // Tip is north of the position; base corners are south of the tip.
    expect(tip[1]).toBeGreaterThan(10.776);
    expect(left[1]).toBeLessThan(tip[1]);
    expect(right[1]).toBeLessThan(tip[1]);
    expect(left[0]).toBeGreaterThan(tip[0]);
    expect(right[0]).toBeLessThan(tip[0]);
  });

  it("rotates with the bearing", () => {
    const east = headingPuckRing([106.7, 10.776], 90);
    const [tip] = east;
    expect(tip[0]).toBeGreaterThan(106.7);
    expect(tip[1]).toBeCloseTo(10.776, 4);
  });
});
