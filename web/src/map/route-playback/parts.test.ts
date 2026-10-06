import { describe, expect, it } from "vitest";
import { buildRoutePartPlaybackInput } from "./parts";

const part = (id: string, from: string, until: string, coordinates: number[][]) => ({
  id,
  geometry: { type: "LineString", coordinates },
  vertex_distance_m: [0, 100],
  progress_anchors: [
    { at: from, distance_m: 0 },
    { at: until, distance_m: 100 },
  ],
});

describe("buildRoutePartPlaybackInput", () => {
  it("uses published anchors and server vertex distances, not vertex timing or Web distance", () => {
    const input = buildRoutePartPlaybackInput([
      part("matched", "2026-10-05T10:00:00Z", "2026-10-05T10:10:00Z", [[106.7, 10.7], [106.9, 10.9]]),
    ]);
    expect(input.ok).toBe(true);
    if (!input.ok) return;
    expect(input.points).toMatchObject([
      { recordedAtMs: Date.parse("2026-10-05T10:00:00Z"), coordinate: [106.7, 10.7] },
      { recordedAtMs: Date.parse("2026-10-05T10:10:00Z"), coordinate: [106.9, 10.9] },
    ]);
  });

  it("holds across a GPS Gap and jumps only at the following observed timestamp", () => {
    const input = buildRoutePartPlaybackInput([
      part("before", "2026-10-05T10:00:00Z", "2026-10-05T10:01:00Z", [[106.7, 10.7], [106.71, 10.71]]),
      part("after", "2026-10-05T10:05:00Z", "2026-10-05T10:06:00Z", [[106.8, 10.8], [106.81, 10.81]]),
    ], [{ kind: "gap", observed_from_at: "2026-10-05T10:01:00Z", observed_until_at: "2026-10-05T10:05:00Z" }]);
    expect(input.ok).toBe(true);
    if (!input.ok) return;
    expect(input.points[1]).toMatchObject({ breakToNext: "gps-gap" });
  });

  it("marks an Evidence Hole separately while never inventing movement across disconnected parts", () => {
    const input = buildRoutePartPlaybackInput([
      part("before", "2026-10-05T10:00:00Z", "2026-10-05T10:01:00Z", [[106.7, 10.7], [106.71, 10.71]]),
      part("after", "2026-10-05T10:05:00Z", "2026-10-05T10:06:00Z", [[106.8, 10.8], [106.81, 10.81]]),
    ], [], [{ observed_from_at: "2026-10-05T10:01:00Z", observed_until_at: "2026-10-05T10:05:00Z" }]);
    expect(input.ok).toBe(true);
    if (!input.ok) return;
    expect(input.points[1]).toMatchObject({ breakToNext: "evidence-hole" });
  });
});
