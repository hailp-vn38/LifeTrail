import { describe, expect, it } from "vitest";
import { advanceWithoutStops, playbackStops } from "./stop-time";

describe("Stop playback timing", () => {
  it("clips to the clock, rejects invalid ranges and merges overlapping Stops", () => {
    expect(playbackStops([
      { startTimeMs: 18, endTimeMs: 30 },
      { startTimeMs: 0, endTimeMs: 15 },
      { startTimeMs: 14, endTimeMs: 18 },
      { startTimeMs: NaN, endTimeMs: 20 },
      { startTimeMs: 20, endTimeMs: 19 },
    ], 10, 25)).toEqual([{ startTimeMs: 0, endTimeMs: 15 }]);
  });

  it.each([
    [0, 2, 2], [0, 3, 8], [0, 10, 17],
    [5, 0, 8], [5, 2, 10], [8, 0, 8], [15, 2, 17],
  ])("advances from %s by %s moving milliseconds to %s", (time, delta, expected) => {
    expect(advanceWithoutStops(time, delta, [
      { startTimeMs: 3, endTimeMs: 8 },
      { startTimeMs: 12, endTimeMs: 14 },
    ])).toBe(expected);
  });
});
