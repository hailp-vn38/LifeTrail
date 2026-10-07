import { describe, expect, it } from "vitest";
import { tripDistanceM, stopActivities, tripActivities, tripDisplayParts } from "./model";
import { tripView, openTrip, dailyPart } from "../../test/fixtures/trips";
import { stationaryView, openStop } from "../../test/fixtures/stationary";

describe("published activity model", () => {
  it("separates Trip and Stop items from the mixed published timeline", () => {
    const view = tripView();
    expect(tripActivities(view)).toHaveLength(1);
    expect(tripActivities(view)[0].id).toBe(openTrip.id);
    expect(stopActivities(view)).toHaveLength(1);
    expect(stopActivities(stationaryView())[0].id).toBe(openStop.id);
  });

  it("returns the published display Route Parts of one Trip in movement order", () => {
    const parts = tripDisplayParts(tripView(), openTrip.id);
    expect(parts).toEqual([dailyPart]);
    expect(parts[0].display_geometry.coordinates.length).toBeGreaterThanOrEqual(2);
    expect(tripDisplayParts(tripView(), "missing")).toEqual([]);
  });

  it("sums the server-published daily part lengths instead of recomputing geometry", () => {
    // The fixture's own coordinates span a different total than distance_m, so a
    // recomputation would disagree with the published metric.
    expect(tripDistanceM(tripView(), openTrip.id)).toBe(2204);
  });

  it("uses a crossing Part's clipped daily distance rather than its UTC length", () => {
    const view = tripView();
    view.route_parts = [
      {
        ...dailyPart,
        // This is the after-midnight projection of a longer source Part. The
        // source distance belongs to UTC history; this Daily View owns only the
        // progress between its visible clipping endpoints.
        distance_m: 2204,
        visible_distance_m: 735,
        visible_from_at: "2026-10-06T00:00:00Z",
        visible_until_at: "2026-10-06T00:10:00Z",
      },
    ];

    expect(tripDistanceM(view, openTrip.id)).toBe(735);
  });
});
