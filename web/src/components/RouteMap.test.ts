import { mount } from "@vue/test-utils";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { createPinia } from "pinia";
import type { Feature, FeatureCollection, LineString, Polygon } from "geojson";
import type { DailyView } from "../api/queries/daily-view.query";

const { MockMap } = vi.hoisted(() => {
  class MockSource {
    setData = vi.fn();
  }
  class MockMap {
    static instances: MockMap[] = [];
    sources = new Map<string, MockSource>();
    addedLayers: unknown[] = [];
    removed = false;
    easeTo = vi.fn();
    fitBounds = vi.fn();
    stop = vi.fn();
    constructor(public readonly options: unknown) {
      MockMap.instances.push(this);
    }
    loaded() {
      return true;
    }
    once() {}
    addSource(id: string) {
      this.sources.set(id, new MockSource());
    }
    addLayer(layer: unknown) {
      this.addedLayers.push(layer);
    }
    getSource(id: string) {
      return this.sources.get(id);
    }
    getContainer() {
      return { clientHeight: 600 };
    }
    cameraForBounds() {
      return undefined;
    }
    addControl = vi.fn();
    on = vi.fn();
    off = vi.fn();
    setPaintProperty = vi.fn();
    remove() {
      this.removed = true;
    }
  }
  return { MockMap };
});

vi.mock("maplibre-gl", () => ({
  LngLatBounds: class {
    extend() {
      return this;
    }
  },
  Map: MockMap,
  NavigationControl: class {},
}));

import RouteMap from "./RouteMap.vue";

const COORDINATES = [
  [106.7, 10.776],
  [106.7005, 10.7764],
  [106.701, 10.777],
  [106.7015, 10.7774],
];
const TIMESTAMPS = [
  "2026-10-05T10:00:00.000Z",
  "2026-10-05T10:00:03.000Z",
  "2026-10-05T10:00:25.000Z",
  "2026-10-05T10:00:30.000Z",
];

function dailyViewFixture(timestamps: string[] | undefined): DailyView {
  return {
    device_id: "8f467f83-f8e0-4d91-8f73-26e77e8fb0b5",
    date: "2026-10-05",
    timezone: "Asia/Ho_Chi_Minh",
    processing_state: "raw",
    summary: {
      point_count: 4,
      distance_m: 120,
      duration_s: 30,
      first_fix_at: TIMESTAMPS[0],
      last_fix_at: TIMESTAMPS[3],
    },
    route: {
      type: "Feature",
      properties: timestamps ? { timestamps } : {},
      geometry: { type: "LineString", coordinates: COORDINATES },
    },
    start: {
      type: "Feature",
      properties: { recorded_at: TIMESTAMPS[0] },
      geometry: { type: "Point", coordinates: COORDINATES[0] },
    },
    end: {
      type: "Feature",
      properties: { recorded_at: TIMESTAMPS[3] },
      geometry: { type: "Point", coordinates: COORDINATES[3] },
    },
  } as DailyView;
}

function lastMap(): InstanceType<typeof MockMap> {
  const instances = MockMap.instances;
  return instances[instances.length - 1];
}

function mountRouteMap(dailyView: DailyView) {
  return mount(RouteMap, {
    props: { dailyView },
    global: { plugins: [createPinia()] },
  });
}

beforeEach(() => {
  MockMap.instances.length = 0;
});

describe("RouteMap", () => {
  it("initializes the map with full, progress and current sources", () => {
    const wrapper = mountRouteMap(dailyViewFixture(TIMESTAMPS));

    const map = lastMap();
    expect(map).toBeDefined();
    for (const sourceId of [
      "daily-route-full",
      "daily-route-progress",
      "daily-route-start",
      "daily-route-end",
      "daily-route-current",
    ]) {
      expect(map.sources.has(sourceId)).toBe(true);
    }
    const layerIds = map.addedLayers.map((layer) => (layer as { id: string }).id);
    expect(layerIds).toContain("daily-route-full-line");
    expect(layerIds).toContain("daily-route-progress-line");
    expect(layerIds).toContain("daily-route-current-dot");
    expect(layerIds).toContain("daily-route-current-arrow");
    expect(layerIds).toContain("daily-route-progress-glow");

    // Heading-up compass is added top-left.
    expect(map.addControl).toHaveBeenCalledTimes(1);
    expect(map.addControl).toHaveBeenCalledWith(expect.anything(), "top-left");
    expect(map.fitBounds).toHaveBeenCalled();

    wrapper.unmount();
  });

  it("updates source data on playback frames instead of re-creating layers", async () => {
    const wrapper = mountRouteMap(dailyViewFixture(TIMESTAMPS));
    await wrapper.vm.$nextTick();
    const map = lastMap();
    const layerCount = map.addedLayers.length;

    await wrapper.find('button[aria-label="Phát"]').trigger("click");

    // Scrub to the middle of the route: the progress line must then contain
    // completed vertices plus the interpolated position.
    await wrapper.find('input[type="range"]').setValue("500");

    expect(map.addedLayers).toHaveLength(layerCount);
    const progressSource = map.sources.get("daily-route-progress");
    const currentSource = map.sources.get("daily-route-current");
    expect(progressSource?.setData).toHaveBeenCalled();
    expect(currentSource?.setData).toHaveBeenCalled();

    const progressCalls = (progressSource?.setData as ReturnType<typeof vi.fn>).mock.calls;
    const progressData = progressCalls[progressCalls.length - 1][0] as Feature<LineString>;
    expect(progressData.geometry.type).toBe("LineString");
    // Seek to 50% of 30s = 15s, inside the 3s..25s segment at ratio 12/22.
    const coordinates = progressData.geometry.coordinates;
    expect(coordinates).toHaveLength(3);
    expect(coordinates[0]).toEqual(COORDINATES[0]);
    expect(coordinates[1]).toEqual(COORDINATES[1]);
    expect(coordinates[2][0]).toBeCloseTo(106.7005 + (0.0005 * 12) / 22, 9);
    expect(coordinates[2][1]).toBeCloseTo(10.7764 + (0.0006 * 12) / 22, 9);
    const currentCalls = (currentSource?.setData as ReturnType<typeof vi.fn>).mock.calls;
    const currentData = currentCalls[currentCalls.length - 1][0] as FeatureCollection;
    // Heading puck: a blue dot (Point) plus a white heading arrow (Polygon).
    expect(currentData.type).toBe("FeatureCollection");
    expect(currentData.features).toHaveLength(2);
    expect(currentData.features[0]?.geometry.type).toBe("Point");
    expect(currentData.features[1]?.geometry.type).toBe("Polygon");

    wrapper.unmount();
  });

  it("pauses playback from the controls", async () => {
    const wrapper = mountRouteMap(dailyViewFixture(TIMESTAMPS));
    await wrapper.vm.$nextTick();

    await wrapper.find('button[aria-label="Phát"]').trigger("click");
    expect(wrapper.find('button[aria-label="Tạm dừng"]').exists()).toBe(true);

    await wrapper.find('button[aria-label="Tạm dừng"]').trigger("click");
    expect(wrapper.find('button[aria-label="Phát"]').exists()).toBe(true);

    wrapper.unmount();
  });

  it("disposes playback and removes the map on unmount", async () => {
    const wrapper = mountRouteMap(dailyViewFixture(TIMESTAMPS));
    await wrapper.vm.$nextTick();
    const map = lastMap();

    await wrapper.find('button[aria-label="Phát"]').trigger("click");
    wrapper.unmount();

    expect(map.removed).toBe(true);
    expect(map.stop).toHaveBeenCalled();
  });

  it("renders the static route when timestamps are invalid", () => {
    const wrapper = mountRouteMap(dailyViewFixture(undefined));

    const map = lastMap();
    expect(map.sources.has("daily-route-full")).toBe(true);
    expect(map.sources.has("daily-route-progress")).toBe(false);
    expect(map.sources.has("daily-route-current")).toBe(false);
    // Static markers still render.
    expect(map.sources.has("daily-route-start")).toBe(true);
    expect(map.sources.has("daily-route-end")).toBe(true);
    // Controls explain why playback is disabled.
    expect(wrapper.find(".playback-note").exists()).toBe(true);
    expect(wrapper.find('button[aria-label="Phát"]').attributes("disabled")).toBeDefined();

    wrapper.unmount();
  });
});
