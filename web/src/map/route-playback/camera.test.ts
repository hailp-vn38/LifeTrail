import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("maplibre-gl", () => ({
  LngLatBounds: class {
    constructor(
      public readonly min: [number, number],
      public readonly max: [number, number],
    ) {}
  },
  LngLat: class {
    static convert(input: { lng: number; lat: number } | [number, number]) {
      if (Array.isArray(input)) {
        return { lng: input[0], lat: input[1] };
      }
      return { lng: input.lng, lat: input.lat };
    }
  },
}));

import {
  FOLLOW_PITCH,
  FOLLOW_ZOOM,
  FollowCamera,
  followOffsetYPx,
  OVERVIEW_MAX_ZOOM,
  overviewCamera,
  prefersReducedMotion,
  type FollowableMap,
} from "./camera";

function mockMap() {
  return {
    easeTo: vi.fn(),
    stop: vi.fn(),
    cameraForBounds: vi.fn(),
  };
}

function asFollowableMap(map: ReturnType<typeof mockMap>): FollowableMap {
  return map as unknown as FollowableMap;
}

describe("FollowCamera", () => {
  let nowMs: number;

  beforeEach(() => {
    nowMs = 0;
  });

  function cameraFor(map: FollowableMap) {
    return new FollowCamera(map, { now: () => nowMs });
  }

  it("enters follow mode with a close navigation-like view", () => {
    const map = mockMap();
    cameraFor(asFollowableMap(map)).enter([106.7, 10.776], 90);

    expect(map.easeTo).toHaveBeenCalledTimes(1);
    expect(map.easeTo).toHaveBeenCalledWith({
      center: [106.7, 10.776],
      zoom: FOLLOW_ZOOM,
      pitch: FOLLOW_PITCH,
      bearing: 90,
      offset: [0, 80],
      duration: 700,
    });
    expect(FOLLOW_ZOOM).toBeCloseTo(16.5, 1);
    expect(FOLLOW_PITCH).toBe(45);
  });

  it("throttles follow updates instead of easing on every frame", () => {
    const map = mockMap();
    const camera = cameraFor(asFollowableMap(map));
    camera.enter([106.7, 10.776], 0);
    map.easeTo.mockClear();

    nowMs = 100;
    camera.update([106.7001, 10.7761], 10);
    expect(map.easeTo).not.toHaveBeenCalled();

    nowMs = 200;
    camera.update([106.7002, 10.7762], 20);
    expect(map.easeTo).toHaveBeenCalledTimes(1);
    expect(map.easeTo).toHaveBeenCalledWith(
      expect.objectContaining({
        center: [106.7002, 10.7762],
        zoom: FOLLOW_ZOOM,
        pitch: FOLLOW_PITCH,
        bearing: 20,
        offset: [0, 80],
        duration: 320,
      }),
    );
  });

  it("normalizes the bearing", () => {
    const map = mockMap();
    cameraFor(asFollowableMap(map)).enter([106.7, 10.776], 405);
    expect(map.easeTo).toHaveBeenCalledWith(expect.objectContaining({ bearing: 45 }));
  });

  it("stops camera animation on dispose", () => {
    const map = mockMap();
    cameraFor(asFollowableMap(map)).dispose();
    expect(map.stop).toHaveBeenCalledTimes(1);
  });

  it("uses instant transitions when reduced motion is preferred", () => {
    const w = window as unknown as { matchMedia?: unknown };
    const original = w.matchMedia;
    w.matchMedia = vi.fn().mockReturnValue({ matches: true });
    try {
      expect(prefersReducedMotion()).toBe(true);

      const map = mockMap();
      cameraFor(asFollowableMap(map)).enter([106.7, 10.776], 0);
      expect(map.easeTo).toHaveBeenCalledWith(expect.objectContaining({ duration: 0 }));
    } finally {
      w.matchMedia = original;
    }
  });
});

describe("followOffsetYPx", () => {
  it("keeps the point below center with a responsive bound", () => {
    expect(followOffsetYPx(800)).toBe(80);
    expect(followOffsetYPx(400)).toBe(48);
    // Narrow layouts still keep a usable minimum.
    expect(followOffsetYPx(200)).toBe(32);
    expect(followOffsetYPx(Number.NaN)).toBe(80);
  });
});

describe("overviewCamera", () => {
  it("animates to the whole route north-up and top-down", () => {
    const map = mockMap();
    map.cameraForBounds.mockReturnValue({ center: { lng: 106.7007, lat: 10.7767 }, zoom: 14 });

    const applied = overviewCamera(asFollowableMap(map), { min: [106.7, 10.776], max: [106.7015, 10.7774] });

    expect(applied).toBe(true);
    expect(map.cameraForBounds).toHaveBeenCalledWith(
      expect.objectContaining({ min: [106.7, 10.776], max: [106.7015, 10.7774] }),
      expect.objectContaining({ maxZoom: OVERVIEW_MAX_ZOOM }),
    );
    expect(map.easeTo).toHaveBeenCalledWith({
      center: [106.7007, 10.7767],
      zoom: 14,
      bearing: 0,
      pitch: 0,
      duration: 1600,
    });
  });

  it("fails safely when no camera can be computed", () => {
    const map = mockMap();
    map.cameraForBounds.mockReturnValue(undefined);

    expect(overviewCamera(asFollowableMap(map), { min: [106.7, 10.776], max: [106.7, 10.776] })).toBe(false);
    expect(map.easeTo).not.toHaveBeenCalled();
  });
});
