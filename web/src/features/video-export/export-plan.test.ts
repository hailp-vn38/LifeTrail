import { describe, expect, it } from "vitest";
import { tripView, tripPlayback, rawPart } from "../../test/fixtures/trips";
import { videoDuration, videoInput } from "./export-plan";

describe("video export plan", () => {
  it("uses the observed route interval instead of a full calendar day", () => {
    const points = videoInput(tripView(), tripPlayback());
    expect(videoDuration(points[0].recordedAtMs, points.at(-1)!.recordedAtMs, 100)).toBe(12);
  });

  it("preserves disconnected Route Parts rather than interpolating a connector", () => {
    const parts = [rawPart, {
      ...rawPart, id: "next-part", progress_anchors: rawPart.progress_anchors.map(anchor => ({
        ...anchor, at: new Date(Date.parse(anchor.at) + 3_600_000).toISOString(),
      })),
    }];
    const points = videoInput(tripView(), parts);
    expect(points.filter(point => point.breakToNext)).toHaveLength(1);
    expect(points[4].breakToNext).toBe("disconnected");
  });

  it("rejects invalid intervals, speeds and missing playback data", () => {
    for (const args of [[1, 1, 100], [2, 1, 1], [0, Infinity, 10], [0, 100, 0]]) {
      expect(() => videoDuration(args[0], args[1], args[2])).toThrow();
    }
    // Processed day without canonical playback is not a valid video source.
    expect(() => videoInput(tripView())).toThrow();
    expect(() => videoInput({ ...tripView(), route_parts: [] })).toThrow();
  });
});
