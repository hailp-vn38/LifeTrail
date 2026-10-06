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

  it("sums the server-published part lengths instead of recomputing geometry", () => {
    // The fixture's own coordinates span a different total than distance_m, so a
    // recomputation would disagree with the published metric.
    expect(tripDistanceM(tripView(), openTrip.id)).toBe(2204);
  });
});
