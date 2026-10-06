import { describe, expect, it } from "vitest";
import { tripDistanceM, stopActivities, tripActivities, tripParts } from "./model";
import { tripView, openTrip, rawPart } from "../../test/fixtures/trips";
import { stationaryView, openStop } from "../../test/fixtures/stationary";

describe("published activity model", () => {
  it("separates Trip and Stop items from the mixed published timeline", () => {
    const view = tripView();
    expect(tripActivities(view)).toHaveLength(1);
    expect(tripActivities(view)[0].id).toBe(openTrip.id);
    expect(stopActivities(view)).toHaveLength(1);
    expect(stopActivities(stationaryView())[0].id).toBe(openStop.id);
  });

  it("returns the published Route Parts of one Trip in movement order", () => {
    const parts = tripParts(tripView(), openTrip.id);
    expect(parts).toEqual([rawPart]);
    expect(parts[0].vertex_distance_m[0]).toBe(0);
    expect(tripParts(tripView(), "missing")).toEqual([]);
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
        ...rawPart,
        // This is the after-midnight projection of a longer source Part. The
        // source distance belongs to UTC history; this Daily View owns only the
        // progress between its visible clipping endpoints.
        distance_m: 2204,
        visible_distance_m: 735,
        visible_from_at: "2026-10-06T00:00:00Z",
        visible_until_at: "2026-10-06T00:10:00Z",
        continues_before: true,
      },
    ];

    expect(tripDistanceM(view, openTrip.id)).toBe(735);
  });
});
