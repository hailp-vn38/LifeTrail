import { describe, expect, it } from "vitest";
import type { DailyView } from "../../api/queries/daily-view.query";
import { buildTimelineEvents } from "./events";

function dailyViewFixture(withPoints: boolean): DailyView {
  return {
    device_id: "device-1",
    date: "2026-10-05",
    timezone: "Asia/Ho_Chi_Minh",
    processing_state: "raw",
    summary: {
      point_count: withPoints ? 2 : 0,
      distance_m: 0,
      duration_s: 0,
      first_fix_at: withPoints ? "2026-10-05T10:00:00.000Z" : null,
      last_fix_at: withPoints ? "2026-10-05T10:00:30.000Z" : null,
    },
    route: null,
    start: withPoints
      ? {
          type: "Feature",
          properties: { recorded_at: "2026-10-05T10:00:00.000Z" },
          geometry: { type: "Point", coordinates: [106.7, 10.776] },
        }
      : null,
    end: withPoints
      ? {
          type: "Feature",
          properties: { recorded_at: "2026-10-05T10:00:30.000Z" },
          geometry: { type: "Point", coordinates: [106.7005, 10.7764] },
        }
      : null,
  } as DailyView;
}

describe("buildTimelineEvents", () => {
  it("returns no events for a zero-point day", () => {
    expect(buildTimelineEvents(dailyViewFixture(false))).toEqual([]);
  });

  it("builds Start/End events for Phase 1", () => {
    const events = buildTimelineEvents(dailyViewFixture(true));

    expect(events.map((event) => event.id)).toEqual(["start", "end"]);
    expect(events.map((event) => event.kind)).toEqual(["start", "end"]);
    expect(events[0]?.coordinate).toEqual([106.7, 10.776]);
    expect(events[1]?.coordinate).toEqual([106.7005, 10.7764]);
    expect(events[0]?.recordedAtMs).toBe(Date.parse("2026-10-05T10:00:00.000Z"));
  });
});

import { qualityGapView } from "../../test/fixtures/quality-gaps";
import { stationaryView } from "../../test/fixtures/stationary";
import { tripView } from "../../test/fixtures/trips";

it("renders observed Stop time and daily overlap with unknown actual boundaries", () => {
  const events = buildTimelineEvents(stationaryView());
  expect(events).toHaveLength(1);
  expect(events[0].kind).toBe("stop");
  expect(events[0].coordinate).toEqual([106.7, 10.77]);
  expect(events[0].subtitle).toContain("30 phút");
  expect(events[0].subtitle).toContain("10 phút");
  expect(events[0].subtitle).toContain("Đến: chưa xác định");
  expect(events[0].subtitle).toContain("Rời: chưa xác định");
});

it("renders a Trip with its published distance, unknown mode and pause-inclusive time", () => {
  const events = buildTimelineEvents(tripView());
  const trip = events.find((event) => event.kind === "trip");
  expect(trip?.title).toBe("Trip");
  expect(trip?.subtitle).toContain("2.20 km");
  expect(trip?.subtitle).toContain("chưa xác định");
  expect(trip?.subtitle).toContain("bao gồm dừng ngắn");
  expect(trip?.subtitle).toContain("20 phút");
  expect(trip?.coordinate).toBeUndefined();
});

it("keeps projected Trip and Stop items in the server's chronological order", () => {
  const events = buildTimelineEvents(tripView());
  expect(events.map((event) => event.kind)).toEqual(["stop", "trip"]);
  expect(events.map((event) => event.recordedAtMs)).toEqual([
    Date.parse("2026-10-05T02:30:00Z"),
    Date.parse("2026-10-05T03:00:00Z"),
  ]);
});

it("renders a GPS Gap as its own Timeline item without a coordinate", () => {
  const events = buildTimelineEvents(qualityGapView());

  expect(events).toHaveLength(1);
  expect(events[0].kind).toBe("gap");
  expect(events[0].title).toBe("GPS Gap");
  expect(events[0].subtitle).toContain("Thiếu quan sát GPS");
  expect(events[0].subtitle).toContain("11 phút");
  expect(events[0].subtitle).toContain("Không suy ra di chuyển");
  // Absence of observations has no place to focus the map on.
  expect(events[0].coordinate).toBeUndefined();
});
