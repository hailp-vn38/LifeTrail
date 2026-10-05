import { describe, expect, it } from "vitest";
import { followTarget } from "./target";
import type { PlaybackFrame, PlaybackPoint } from "./types";

const points: PlaybackPoint[] = [
  { coordinate: [0, 0], recordedAtMs: 0 },
  { coordinate: [0, 0.0001], recordedAtMs: 1000 },
  { coordinate: [0.001, 0.0001], recordedAtMs: 2000 },
];
const frame: PlaybackFrame = {
  state: "playing", routeTimeMs: 0, durationMs: 2000, progress: 0,
  position: [0, 0], bearing: 0, vertexIndex: 0, segmentRatio: 0,
};

describe("followTarget", () => {
  it("anticipates a turn using the route ahead instead of the current segment", () => {
    const target = followTarget(points, frame);
    expect(target.lookAhead[0]).toBeGreaterThan(0);
    expect(target.bearing).toBeGreaterThan(45);
    expect(target.bearing).toBeLessThan(90);
  });

  it("keeps the incoming bearing at the final coordinate", () => {
    const final = { ...frame, position: points[2].coordinate, vertexIndex: 1, bearing: 90 };
    expect(followTarget(points, final)).toEqual({
      position: points[2].coordinate, lookAhead: points[2].coordinate, bearing: 90,
    });
  });
});
