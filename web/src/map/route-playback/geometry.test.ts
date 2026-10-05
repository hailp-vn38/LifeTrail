import { describe, expect, it } from "vitest";
import {
  bearingBetween,
  haversineDistanceM,
  normalizeBearing,
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
