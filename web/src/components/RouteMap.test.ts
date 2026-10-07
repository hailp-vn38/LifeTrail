import { mount, flushPromises } from "@vue/test-utils";
import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { createPinia } from "pinia";
import type { Feature, FeatureCollection, LineString, Polygon } from "geojson";
import type { DailyView } from "../api/queries/daily-view.query";
import { MAP_STYLE_STORAGE_KEY, useMapPreferencesStore } from "../stores/map-preferences.store";

const { MockMap } = vi.hoisted(() => {
  class MockSource {
    constructor(readonly data?: unknown) {}
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
    dragRotate = { isEnabled: vi.fn(() => true), disable: vi.fn(), enable: vi.fn() };
    touchPitch = { isEnabled: vi.fn(() => true), disable: vi.fn(), enable: vi.fn() };
    getMinPitch = vi.fn(() => 0);
    getMaxPitch = vi.fn(() => 60);
    setMinPitch = vi.fn();
    setMaxPitch = vi.fn();
    setStyle = vi.fn(() => {
      this.sources.clear();
      this.addedLayers = [];
    });
    getStyle = vi.fn(() => ({ version: 8, sources: {}, layers: [] }));
    constructor(public readonly options: unknown) {
      MockMap.instances.push(this);
    }
    loaded() {
      return true;
    }
    once() {}
    addSource(id: string, source?: { data?: unknown }) {
      this.sources.set(id, new MockSource(source?.data));
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
    cameraForBounds = vi.fn((): { center: [number, number]; zoom: number } | undefined => undefined);
    addControl = vi.fn();
    on = vi.fn();
    off = vi.fn();
    setPaintProperty = vi.fn();
    setFilter = vi.fn();
    setLayoutProperty = vi.fn();
    getLayer(id: string) { return this.addedLayers.find(layer => (layer as { id: string }).id === id); }
    remove() {
      this.removed = true;
    }
  }
  return { MockMap };
});

vi.mock("maplibre-gl", () => ({
  LngLat: class {
    static convert(input: [number, number]) { return { lng: input[0], lat: input[1] }; }
  },
  LngLatBounds: class {
    extend() {
      return this;
    }
  },
  Map: MockMap,
  NavigationControl: class {},
  Popup: class { setLngLat() { return this; } setDOMContent() { return this; } addTo() { return this; } remove() {} },
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

function mountRouteMap(dailyView: DailyView, playbackParts?: ReturnType<typeof tripPlayback>) {
  return mount(RouteMap, {
    props: { dailyView, ...(playbackParts ? { playbackParts } : {}) },
    global: { plugins: [createPinia()] },
  });
}

beforeEach(() => {
  localStorage.clear();
  MockMap.instances.length = 0;
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllEnvs();
});

describe("RouteMap", () => {
  it("opens the saved 3D style with a tilted camera", () => {
    vi.stubEnv("VITE_MAPTILER_KEY", "test-key");
    localStorage.setItem(MAP_STYLE_STORAGE_KEY, "streets-3d");
    const wrapper = mountRouteMap(dailyViewFixture(TIMESTAMPS));
    expect(lastMap().options).toMatchObject({
      style: "https://api.maptiler.com/maps/streets-v4/style.json?key=test-key", pitch: 55,
    });
    expect(lastMap().fitBounds.mock.calls.at(-1)?.[1]).toMatchObject({ pitch: 55 });
    wrapper.unmount();
  });

  it("restores route sources and the playback position after replacing the style", async () => {
    vi.stubEnv("VITE_MAPTILER_KEY", "test-key");
    const wrapper = mountRouteMap(dailyViewFixture(TIMESTAMPS));
    try {
      await wrapper.vm.$nextTick();
      await wrapper.find('button[aria-label="Phát"]').trigger("click");
      await wrapper.find('input[type="range"]').setValue("500");
      const map = lastMap();
      const previousPuck = map.sources.get("daily-route-current")?.setData.mock.calls.at(-1)?.[0];
      useMapPreferencesStore().selectStyle("streets-3d");
      await wrapper.vm.$nextTick();
      expect(map.setStyle).toHaveBeenCalledWith("https://api.maptiler.com/maps/streets-v4/style.json?key=test-key", { diff: false });
      expect(wrapper.find('button[aria-label="Phát"]').attributes("disabled")).toBeDefined();
      const loaded = map.on.mock.calls.find(([event]) => event === "style.load")?.[1];
      loaded();
      await wrapper.vm.$nextTick();
      expect(map.sources.has("daily-route-full")).toBe(true);
      expect(map.sources.get("daily-route-current")?.setData).toHaveBeenLastCalledWith(previousPuck);
      expect(map.easeTo.mock.calls.at(-1)?.[0]).toMatchObject({ pitch: 55 });
      expect(wrapper.find('button[aria-label="Phát"]').attributes("disabled")).toBeUndefined();
    } finally {
      wrapper.unmount();
    }
  });
  it("holds the final navigation frame then returns to a top-down route overview", async () => {
    vi.useFakeTimers();
    const wrapper = mountRouteMap(dailyViewFixture(TIMESTAMPS));
    try {
      await wrapper.vm.$nextTick();
      const map = lastMap();
      map.cameraForBounds.mockReturnValue({ center: [106.7007, 10.7767], zoom: 14 });
      await wrapper.find('button[aria-label="Phát"]').trigger("click");
      vi.advanceTimersByTime(30_016);
      expect(map.easeTo.mock.calls.at(-1)?.[0]).toMatchObject({
        center: COORDINATES.at(-1), pitch: 55, zoom: 17,
      });
      expect(map.cameraForBounds).not.toHaveBeenCalled();
      expect(map.dragRotate.enable).toHaveBeenCalled();
      vi.advanceTimersByTime(700);
      expect(map.easeTo.mock.calls.at(-1)?.[0]).toMatchObject({
        center: [106.7007, 10.7767], pitch: 0, bearing: 0, zoom: 14, duration: 1800,
      });
    } finally {
      wrapper.unmount();
    }
  });

  it("releases navigation controls when a user gesture exits follow mode", async () => {
    vi.useFakeTimers();
    const wrapper = mountRouteMap(dailyViewFixture(TIMESTAMPS));
    try {
      await wrapper.vm.$nextTick();
      await wrapper.find('button[aria-label="Phát"]').trigger("click");
      const map = lastMap();
      const dragHandler = map.on.mock.calls.find(([event]) => event === "dragstart")?.[1];
      dragHandler({ originalEvent: new Event("mousedown") });
      expect(map.dragRotate.enable).toHaveBeenCalled();
      expect(map.touchPitch.enable).toHaveBeenCalled();
      expect(map.setMinPitch).toHaveBeenLastCalledWith(0);
      map.easeTo.mockClear();
      vi.advanceTimersByTime(1000);
      expect(map.easeTo).not.toHaveBeenCalled();
    } finally {
      wrapper.unmount();
    }
  });

  it("rotates the UI camera on a densely sampled turn and follows the final position", async () => {
    vi.useFakeTimers();
    const coordinates = [
      ...Array.from({ length: 11 }, (_, i) => [106.7 + i * 0.00002, 10.776]),
      ...Array.from({ length: 10 }, (_, i) => [106.7002, 10.776 - (i + 1) * 0.00002]),
    ];
    const dailyView = dailyViewFixture(TIMESTAMPS);
    dailyView.route!.geometry.coordinates = coordinates;
    dailyView.route!.properties.timestamps = coordinates.map((_, i) =>
      new Date(Date.parse(TIMESTAMPS[0]) + i * 1000).toISOString(),
    );
    const wrapper = mountRouteMap(dailyView);
    try {
      await wrapper.vm.$nextTick();
      await wrapper.find('button[aria-label="Phát"]').trigger("click");
      const map = lastMap();
      expect(map.easeTo.mock.calls.at(-1)?.[0].bearing).toBeGreaterThan(90);
      expect(map.easeTo.mock.calls.at(-1)?.[0].bearing).toBeLessThan(180);

      vi.advanceTimersByTime(10_240);
      expect(map.easeTo.mock.calls.at(-1)?.[0].bearing).toBeGreaterThan(170);
      vi.advanceTimersByTime(9_800);
      expect(map.easeTo.mock.calls.at(-1)?.[0]).toMatchObject({
        center: coordinates.at(-1), bearing: 180, pitch: 55,
      });
    } finally {
      wrapper.unmount();
    }
  });

  it("moves the follow camera to the look-ahead target when scrubbing", async () => {
    const wrapper = mountRouteMap(dailyViewFixture(TIMESTAMPS));
    await wrapper.vm.$nextTick();
    await wrapper.find('button[aria-label="Phát"]').trigger("click");
    const map = lastMap();
    map.easeTo.mockClear();
    await wrapper.find('input[type="range"]').setValue("500");
    expect(map.easeTo).toHaveBeenCalledWith(expect.objectContaining({
      bearing: expect.any(Number), pitch: 55, zoom: 17,
      center: expect.any(Array),
    }));
    const center = map.easeTo.mock.calls.at(-1)?.[0].center;
    const source = map.sources.get("daily-route-current");
    const puck = source?.setData.mock.calls.at(-1)?.[0] as FeatureCollection;
    expect(center[0]).toBeGreaterThan((puck.features[0].geometry as { coordinates: number[] }).coordinates[0]);
    wrapper.unmount();
  });

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
    expect(lastMap().dragRotate.disable).toHaveBeenCalled();
    expect(lastMap().touchPitch.disable).toHaveBeenCalled();
    expect(lastMap().setMinPitch).toHaveBeenCalledWith(55);

    await wrapper.find('button[aria-label="Tạm dừng"]').trigger("click");
    expect(wrapper.find('button[aria-label="Phát"]').exists()).toBe(true);
    expect(lastMap().dragRotate.enable).toHaveBeenCalled();
    expect(lastMap().touchPitch.enable).toHaveBeenCalled();
    expect(lastMap().setMinPitch).toHaveBeenLastCalledWith(0);

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

import { nextTick } from "vue";
import { useMapStore } from "../stores/map.store";
import { stopActivities } from "../features/activity/model";
import { PART_SOURCE } from "../map/route-parts";
import { STOP_SOURCE } from "../map/stops";
import { stationaryView, openStop } from "../test/fixtures/stationary";
import { tripView, tripPlayback, openTrip, rawPart, dailyPart } from "../test/fixtures/trips";

it("focuses and highlights a Stop disk and selects the same Timeline item from its map feature", async () => {
  const wrapper = mountRouteMap(stationaryView());
  const map = lastMap();
  expect(map.sources.has("activity-stops")).toBe(true);
  expect(map.addedLayers).toEqual(expect.arrayContaining([
    expect.objectContaining({ id: "stop-radius", type: "fill" }),
    expect.objectContaining({ id: "stop-markers", type: "circle" }),
  ]));
  const store = useMapStore();
  store.selectEvent(openStop.id);
  await nextTick();
  expect(map.easeTo).toHaveBeenLastCalledWith(expect.objectContaining({ center: [106.7, 10.77] }));
  expect(map.setPaintProperty).toHaveBeenCalledWith("stop-radius", "fill-opacity", expect.any(Array));
  store.clearSelection();
  const registration = map.on.mock.calls.find((args) => args[0] === "click" && args[1] === "stop-markers");
  expect(registration).toBeDefined();
  const callback = registration?.[2] as unknown as (event: unknown) => void;
  callback({ features: [{ properties: { eventId: openStop.id } }] });
  expect(store.selectedEventId).toBe(openStop.id);
  wrapper.unmount();
});

it("highlights and fits a Trip display Route Part and selects its Timeline item from the map", async () => {
  const wrapper = mountRouteMap(tripView(), tripPlayback());
  const map = lastMap();
  // Processed Route Parts render their display geometry; canonical playback is a
  // separate lazy resource, so the controller comes from playbackParts.
  expect(map.sources.has("activity-route-parts")).toBe(true);
  expect(map.addedLayers).toEqual(expect.arrayContaining([
    expect.objectContaining({ id: "trip-route-parts", type: "line" }),
  ]));
  expect(wrapper.find(".playback-bar").exists()).toBe(true);
  // One GeoJSON line feature per published Route Part, carrying its Trip id.
  const source = map.sources.get("activity-route-parts");
  const collection = source?.data as FeatureCollection<LineString>;
  expect(collection.features).toHaveLength(1);
  expect(collection.features[0].properties).toEqual({
    eventId: openTrip.id, partId: dailyPart.id,
  });
  expect(collection.features[0].geometry.coordinates).toEqual(
    dailyPart.display_geometry.coordinates,
  );

  const store = useMapStore();
  store.selectEvent(openTrip.id);
  await nextTick();
  expect(map.fitBounds).toHaveBeenCalled();
  expect(map.setPaintProperty).toHaveBeenCalledWith(
    "trip-route-parts", "line-width", expect.any(Array),
  );
  store.clearSelection();

  const registration = map.on.mock.calls.find(
    (args) => args[0] === "click" && args[1] === "trip-route-parts",
  );
  expect(registration).toBeDefined();
  const callback = registration?.[2] as unknown as (event: unknown) => void;
  callback({ features: [{ properties: { eventId: openTrip.id, partId: dailyPart.id } }] });
  expect(store.selectedEventId).toBe(openTrip.id);
  wrapper.unmount();
});

it("renders display geometry with no controller until playback is engaged", async () => {
  const wrapper = mountRouteMap(tripView());
  await flushPromises();
  const map = lastMap();
  // Display geometry is drawn; no canonical controller exists yet.
  expect(map.sources.get("activity-route-parts")?.data).toMatchObject({
    features: [{ geometry: { coordinates: dailyPart.display_geometry.coordinates } }],
  });
  expect(usePlaybackStore().status).toBe("idle");
  // Pressing play engages the lazy load instead of failing.
  await wrapper.get('button[aria-label="Phát"]').trigger("click");
  expect(wrapper.emitted("request-playback")).toHaveLength(1);
  // Canonical parts arrive: the controller is created and playback starts.
  await wrapper.setProps({ playbackParts: tripPlayback() });
  expect(usePlaybackStore().status).toBe("playing");
  wrapper.unmount();
});

function mountWithRecordCount(pointCount: number) {
  const view = tripView();
  view.summary.point_count = pointCount;
  const wrapper = mountRouteMap(view);
  const map = lastMap();
  return {
    wrapper,
    map,
    sources: map.sources.size,
    layers: (map.addedLayers as { id: string; type: string }[]).length,
  };
}

/**
 * GeoJSON features actually written into every source.
 *
 * Source and layer counts stay constant whether a day holds two records or
 * thirty thousand, so only the feature count inside each source can reveal a
 * marker loop over GPS Records.
 */
function featureCounts(map: InstanceType<typeof MockMap>): Record<string, number> {
  return Object.fromEntries(
    [...map.sources.entries()].map(([id, source]) => {
      const data = source.data as FeatureCollection | undefined;
      return [id, data?.type === "FeatureCollection" ? data.features.length : 1];
    }),
  );
}

it("draws disconnected geometry across a Gap without a marker per GPS Record", () => {
  const view = tripView();
  // One Part either side of a GPS Gap, from two Trips of the same day.
  view.route_parts = [
    dailyPart,
    { ...dailyPart, id: "rev:trip:1:segment:0:part:0", trip_id: "rev:trip:1" },
  ];
  view.timeline = [
    { ...openTrip },
    {
      id: "rev:gap:0",
      kind: "gap",
      activity_revision: "rev",
      observed_from_at: "2026-10-05T02:40:00Z",
      observed_until_at: "2026-10-05T02:50:00Z",
      observed_duration_s: 600,
      visible_from_at: "2026-10-05T02:40:00Z",
      visible_until_at: "2026-10-05T02:50:00Z",
      daily_observed_duration_s: 600,
      continues_before: false,
      continues_after: false,
    },
    { ...openTrip, id: "rev:trip:1" },
  ];
  const wrapper = mountRouteMap(view);
  const map = lastMap();

  // Each published Part is its own line, so the Gap reads as a break in the
  // geometry rather than a connector through unobserved space.
  const collection = map.sources.get("activity-route-parts")?.data as FeatureCollection<LineString>;
  expect(collection.features).toHaveLength(2);
  expect(collection.features.map((feature) => feature.properties?.eventId)).toEqual([
    openTrip.id,
    "rev:trip:1",
  ]);
  wrapper.unmount();
});

it("creates no map marker per GPS Record", () => {
  // A dense day and a sparse day differ only in `summary.point_count`. Every
  // source must therefore carry the same GeoJSON features, so the per-record
  // marker loop this guards against would show up as a growing feature count
  // rather than only as a new source or layer.
  const dense = mountWithRecordCount(30_000);
  const denseFeatures = featureCounts(dense.map);
  dense.wrapper.unmount();
  const sparse = mountWithRecordCount(2);
  const sparseFeatures = featureCounts(sparse.map);
  sparse.wrapper.unmount();

  expect(sparseFeatures).toEqual(denseFeatures);
  expect(dense.sources).toBe(sparse.sources);
  expect(dense.layers).toBe(sparse.layers);

  // The marker-bearing sources carry exactly the published activity: one line
  // per Route Part, and a disk plus a centre marker per Stop. Thirty thousand Raw
  // GPS Records would make a per-record loop publish thirty thousand features.
  const view = tripView();
  view.summary.point_count = 30_000;
  expect(denseFeatures[PART_SOURCE]).toBe(view.route_parts!.length);
  expect(denseFeatures[STOP_SOURCE]).toBe(2 * stopActivities(view).length);
  expect(denseFeatures[PART_SOURCE]).toBeLessThan(view.summary.point_count!);
  expect(denseFeatures[STOP_SOURCE]).toBeLessThan(view.summary.point_count!);
});

it("renders processed Route Parts without deriving length from their coordinates", () => {
  const view = tripView();
  // The fixture's coordinates describe a shorter span than the published length,
  // so a Web-side recomputation would contradict the server-owned metric.
  const part = view.route_parts![0];
  expect(part.distance_m).toBe(2204);
  expect(part.visible_distance_m).toBe(2204);
  const wrapper = mountRouteMap(view);
  const map = lastMap();
  expect(map.sources.get("activity-route-parts")).toBeDefined();
  // The published anchor clock, not a Web Haversine calculation, enables it.
  expect(wrapper.find('button[aria-label="Phát"]').exists()).toBe(true);
  wrapper.unmount();
});

import { usePlaybackStore } from "../stores/playback.store";

it.each(["raw", "processed"])("plays the %s Daily Route without selecting a Timeline item", async (source) => {
  vi.useFakeTimers();
  const view = source === "raw" ? dailyViewFixture(TIMESTAMPS) : tripView();
  const firstObservation = Date.parse(source === "raw" ? TIMESTAMPS[0] : view.route_parts![0].observed_from_at);
  const wrapper = mount(RouteMap, {
    props: {
      dailyView: view,
      dayClock: true,
      ...(source === "processed" ? { playbackParts: tripPlayback() } : {}),
    },
    global: { plugins: [createPinia()] },
  });
  try {
    await nextTick();
    const playback = usePlaybackStore();
    expect(useMapStore().selectedEventId).toBeNull();
    await wrapper.get('button[aria-label="Phát"]').trigger("click");
    expect(playback.status).toBe("playing");
    expect(playback.startTimeMs + playback.currentTimeMs).toBe(firstObservation);
    const current = lastMap().sources.get("daily-route-current")?.setData.mock.calls.at(-1)?.[0] as FeatureCollection;
    expect(current.features).toHaveLength(2);
    vi.advanceTimersByTime(1000);
    expect(playback.startTimeMs + playback.currentTimeMs).toBeGreaterThan(firstObservation);
    expect(useMapStore().selectedEventId).toBeNull();
    await wrapper.get('button[aria-label="Tạm dừng"]').trigger("click");
    const pausedTime = playback.currentTimeMs;
    await wrapper.get('button[aria-label="Phát"]').trigger("click");
    expect(playback.currentTimeMs).toBe(pausedTime);
    await wrapper.get('button[aria-label="Chạy lại từ đầu"]').trigger("click");
    expect(playback.currentTimeMs).toBe(0);
    await wrapper.get('button[aria-label="Phát"]').trigger("click");
    expect(playback.startTimeMs + playback.currentTimeMs).toBe(firstObservation);
  } finally {
    wrapper.unmount();
  }
});

it("reveals Daily Map dots from midnight, hides them on backward seek and replays from route coverage", async () => {
  const view = dailyViewFixture(TIMESTAMPS);
  const wrapper = mount(RouteMap, { props: { dailyView: view, dayClock: true }, global: { plugins: [createPinia()] } });
  const map = lastMap();
  const playback = usePlaybackStore();
  const midnight = Date.parse("2026-10-04T17:00:00Z");
  expect(playback.startTimeMs).toBe(midnight);
  expect(map.setFilter).toHaveBeenCalledWith("timeline-event-dots-circle", ["<=", ["get", "revealedAtMs"], midnight]);
  expect(map.sources.get("daily-route-current")?.setData).toHaveBeenLastCalledWith({ type: "FeatureCollection", features: [] });
  const events = map.sources.get("timeline-event-dots")?.data as FeatureCollection;
  expect(events.features.map(event => event.properties?.number)).toEqual([1, 2]);
  playback.requestSeek(Date.parse(TIMESTAMPS[1]));
  await nextTick();
  expect(map.setFilter).toHaveBeenCalledWith("timeline-event-dots-circle", ["<=", ["get", "revealedAtMs"], Date.parse(TIMESTAMPS[1])]);
  expect(playback.currentTimeMs).toBe(Date.parse(TIMESTAMPS[1]) - midnight);
  await wrapper.get('button[aria-label="Phát"]').trigger("click");
  expect(playback.currentTimeMs).toBe(Date.parse(TIMESTAMPS[1]) - midnight);
  playback.requestSeek(midnight);
  await nextTick();
  expect(map.setFilter).toHaveBeenLastCalledWith("timeline-event-dot-halo", ["<=", ["get", "revealedAtMs"], midnight]);
  await wrapper.get('input[type="range"]').setValue('1000');
  expect(playback.status).toBe('finished');
  expect(map.setFilter).toHaveBeenCalledWith("timeline-event-dots-circle", ["<=", ["get", "revealedAtMs"], playback.endTimeMs]);
  playback.requestRestart();
  await nextTick();
  expect(playback.status).toBe('playing');
  expect(playback.currentTimeMs).toBe(Date.parse(TIMESTAMPS[0]) - midnight);
  wrapper.unmount();
});

it("keeps all Daily Map dots visible and controls disabled when timestamps are missing", () => {
  const wrapper = mount(RouteMap, { props: { dailyView: dailyViewFixture(undefined), dayClock: true }, global: { plugins: [createPinia()] } });
  const map = lastMap();
  const layer = map.addedLayers.find(layer => (layer as { id: string }).id === 'timeline-event-dots-circle') as { filter: unknown };
  expect(layer.filter).toEqual(['<=', ['get', 'revealedAtMs'], Number.MAX_SAFE_INTEGER]);
  expect(wrapper.get('button[aria-label="Phát"]').attributes('disabled')).toBeDefined();
  wrapper.unmount();
});
