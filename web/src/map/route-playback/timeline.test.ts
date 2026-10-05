import { describe, expect, it } from "vitest";
import {
  buildPlaybackInput,
  clampRouteTime,
  locateSegment,
  progressRatio,
  routeDurationMs,
  type RouteFeatureInput,
} from "./timeline";
import type { PlaybackPoint } from "./types";

const COORDS: [number, number][] = [
  [106.7, 10.776],
  [106.7005, 10.7764],
  [106.701, 10.777],
  [106.7015, 10.7774],
];

/** Intentionally irregular GPS sampling: 3s, 22s, 5s. */
const TIMES_MS = [0, 3_000, 25_000, 30_000];

function featureOf(coordinates: [number, number][], timestamps: unknown): RouteFeatureInput {
  return {
    type: "Feature",
    properties: { timestamps },
    geometry: { type: "LineString", coordinates },
  };
}

function isoFeature(): RouteFeatureInput {
  const base = Date.parse("2026-10-05T10:00:00.000Z");
  return featureOf(
    COORDS,
    TIMES_MS.map((offset) => new Date(base + offset).toISOString()),
  );
}

function points(): PlaybackPoint[] {
  const input = buildPlaybackInput(isoFeature());
  if (!input.ok) throw new Error(`fixture invalid: ${input.error}`);
  return input.points;
}

describe("buildPlaybackInput", () => {
  it("normalizes timestamps into playback points", () => {
    const input = buildPlaybackInput(isoFeature());
    expect(input.ok).toBe(true);
    if (!input.ok) return;
    expect(input.points).toHaveLength(4);
    expect(input.points.map((point) => point.recordedAtMs)).toEqual(TIMES_MS.map((t) => t + Date.parse("2026-10-05T10:00:00.000Z")));
    expect(input.points[0].coordinate).toEqual([106.7, 10.776]);
  });

  it("rejects routes with fewer than 2 coordinates", () => {
    const input = buildPlaybackInput(featureOf([[106.7, 10.776]], ["2026-10-05T10:00:00Z"]));
    expect(input).toEqual({ ok: false, error: "too-few-coordinates" });
  });

  it("rejects missing timestamps", () => {
    const input = buildPlaybackInput({
      type: "Feature",
      properties: {},
      geometry: { type: "LineString", coordinates: COORDS },
    });
    expect(input).toEqual({ ok: false, error: "missing-timestamps" });
  });

  it("rejects timestamp/coordinate length mismatch", () => {
    const input = buildPlaybackInput(
      featureOf(COORDS, ["2026-10-05T10:00:00Z", "2026-10-05T10:00:03Z"]),
    );
    expect(input).toEqual({ ok: false, error: "timestamp-count-mismatch" });
  });

  it("rejects an unparseable timestamp", () => {
    const input = buildPlaybackInput(
      featureOf(COORDS, [
        "2026-10-05T10:00:00Z",
        "not-a-date",
        "2026-10-05T10:00:25Z",
        "2026-10-05T10:00:30Z",
      ]),
    );
    expect(input).toEqual({ ok: false, error: "invalid-timestamp" });
  });

  it("rejects non-monotonic timestamps without reordering the track", () => {
    const input = buildPlaybackInput(
      featureOf(COORDS, [
        "2026-10-05T10:00:00Z",
        "2026-10-05T10:00:25Z",
        "2026-10-05T10:00:03Z",
        "2026-10-05T10:00:30Z",
      ]),
    );
    expect(input).toEqual({ ok: false, error: "non-monotonic-timestamps" });
  });

  it("rejects a non-LineString feature", () => {
    const input = buildPlaybackInput({
      type: "Feature",
      properties: { timestamps: [] },
      geometry: { type: "Point", coordinates: [106.7, 10.776] },
    });
    expect(input).toEqual({ ok: false, error: "not-linestring" });
  });
});

describe("locateSegment", () => {
  it("handles regular intervals", () => {
    const regular: PlaybackPoint[] = [0, 10_000, 20_000, 30_000].map((recordedAtMs, index) => ({
      coordinate: [106.7 + index * 0.001, 10.776] as [number, number],
      recordedAtMs,
    }));
    const location = locateSegment(regular, 15_000);
    expect(location.vertexIndex).toBe(1);
    expect(location.segmentRatio).toBeCloseTo(0.5, 6);
    expect(location.position[0]).toBeCloseTo(106.7015, 6);
  });

  it("keeps real durations for irregular intervals (22s middle segment)", () => {
    const pts = points();
    // The cursor stays inside the 3s..25s segment for the whole 22s window.
    expect(locateSegment(pts, 3_000).vertexIndex).toBe(1);
    expect(locateSegment(pts, 14_000).vertexIndex).toBe(1);
    expect(locateSegment(pts, 24_999).vertexIndex).toBe(1);
    expect(locateSegment(pts, 25_000).vertexIndex).toBe(2);

    const middle = locateSegment(pts, 14_000);
    expect(middle.segmentRatio).toBeCloseTo((14_000 - 3_000) / 22_000, 6);
    // Position interpolates between the 2nd and 3rd coordinates only.
    expect(middle.position[0]).toBeGreaterThan(COORDS[1][0]);
    expect(middle.position[0]).toBeLessThan(COORDS[2][0]);
  });

  it("treats duplicate timestamps as immediately advanced without dividing by zero", () => {
    const duplicated: PlaybackPoint[] = [
      { coordinate: [106.7, 10.776], recordedAtMs: 0 },
      { coordinate: [106.7005, 10.7764], recordedAtMs: 3_000 },
      { coordinate: [106.701, 10.777], recordedAtMs: 3_000 },
      { coordinate: [106.7015, 10.7774], recordedAtMs: 30_000 },
    ];
    const location = locateSegment(duplicated, 3_000);
    // Selects the later point of the zero-duration segment.
    expect(location.vertexIndex).toBe(2);
    expect(location.position).toEqual([106.701, 10.777]);
    expect(Number.isFinite(location.position[0])).toBe(true);
    expect(Number.isFinite(location.position[1])).toBe(true);
  });

  it("seeks exactly onto a vertex", () => {
    const location = locateSegment(points(), 3_000);
    expect(location.vertexIndex).toBe(1);
    expect(location.segmentRatio).toBe(0);
    expect(location.position).toEqual(COORDS[1]);
  });

  it("seeks between vertices", () => {
    const location = locateSegment(points(), 1_500);
    expect(location.vertexIndex).toBe(0);
    expect(location.segmentRatio).toBeCloseTo(0.5, 6);
    expect(location.position[0]).toBeCloseTo((COORDS[0][0] + COORDS[1][0]) / 2, 9);
  });

  it("clamps a seek before the start", () => {
    const location = locateSegment(points(), -5_000);
    expect(location.vertexIndex).toBe(0);
    expect(location.segmentRatio).toBe(0);
    expect(location.position).toEqual(COORDS[0]);
  });

  it("clamps a seek after the end", () => {
    const location = locateSegment(points(), 9_999_999);
    expect(location.vertexIndex).toBe(2);
    expect(location.segmentRatio).toBe(1);
    expect(location.position).toEqual(COORDS[3]);
  });
});

describe("routeDurationMs / clampRouteTime / progressRatio", () => {
  it("measures duration from the GPS timestamps", () => {
    expect(routeDurationMs(points())).toBe(30_000);
  });

  it("clamps route time into [0, duration]", () => {
    expect(clampRouteTime(-10, 30_000)).toBe(0);
    expect(clampRouteTime(99_999, 30_000)).toBe(30_000);
    expect(clampRouteTime(Number.NaN, 30_000)).toBe(0);
  });

  it("reports progress as a 0..1 ratio", () => {
    expect(progressRatio(15_000, 30_000)).toBe(0.5);
    expect(progressRatio(30_000, 30_000)).toBe(1);
    expect(progressRatio(0, 0)).toBe(1);
  });
});
