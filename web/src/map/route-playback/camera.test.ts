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

  function target(position: [number, number], lookAhead: [number, number], bearing: number) {
    return { position, lookAhead, bearing };
  }

  it("enters follow mode aiming at the look-ahead point", () => {
    const map = mockMap();
    cameraFor(asFollowableMap(map)).enter(target([106.7, 10.776], [106.7002, 10.7762], 90));

    expect(map.easeTo).toHaveBeenCalledTimes(1);
    expect(map.easeTo).toHaveBeenCalledWith({
      center: [106.7002, 10.7762],
      zoom: FOLLOW_ZOOM,
      pitch: FOLLOW_PITCH,
      bearing: 90,
      offset: [0, 120],
      duration: 700,
    });
    expect(FOLLOW_ZOOM).toBeCloseTo(17, 1);
    expect(FOLLOW_PITCH).toBe(55);
  });

  it("throttles follow updates instead of easing on every frame", () => {
    const map = mockMap();
    const camera = cameraFor(asFollowableMap(map));
    camera.enter(target([106.7, 10.776], [106.7002, 10.7762], 0));
    map.easeTo.mockClear();

    nowMs = 100;
    camera.update(target([106.7001, 10.7761], [106.7003, 10.7763], 10));
    expect(map.easeTo).not.toHaveBeenCalled();

    nowMs = 200;
    camera.update(target([106.7002, 10.7762], [106.7004, 10.7764], 20));
    expect(map.easeTo).toHaveBeenCalledTimes(1);
    expect(map.easeTo).toHaveBeenCalledWith(
      expect.objectContaining({
        center: [106.7004, 10.7764],
        zoom: FOLLOW_ZOOM,
        pitch: FOLLOW_PITCH,
        bearing: expect.closeTo(20 * (1 - Math.exp(-0.5)), 5),
        offset: [0, 120],
        duration: 500,
      }),
    );
  });

  it("normalizes the bearing", () => {
    const map = mockMap();
    cameraFor(asFollowableMap(map)).enter(target([106.7, 10.776], [106.7002, 10.7762], 405));
    expect(map.easeTo).toHaveBeenCalledWith(expect.objectContaining({ bearing: 45 }));
  });

  it("smooths across north along the short arc and resets on seek", () => {
    const map = mockMap();
    const camera = cameraFor(asFollowableMap(map));
    camera.enter(target([0, 0], [0, 1], 358));
    nowMs = 200;
    camera.update(target([0, 0], [0, 1], 2));
    const bearing = map.easeTo.mock.calls.at(-1)?.[0].bearing;
    expect(bearing).toBeGreaterThan(358);
    expect(bearing).toBeLessThan(360);
    nowMs = 400;
    camera.update(target([0, 0], [0, 1], 2));
    expect(map.easeTo.mock.calls.at(-1)?.[0].bearing).toBeLessThan(2);

    camera.enter(target([0, 0], [1, 0], 90));
    expect(map.easeTo.mock.calls.at(-1)?.[0].bearing).toBe(90);
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
      cameraFor(asFollowableMap(map)).enter(target([106.7, 10.776], [106.7002, 10.7762], 0));
      expect(map.easeTo).toHaveBeenCalledWith(expect.objectContaining({ duration: 0 }));
    } finally {
      w.matchMedia = original;
    }
  });
});

describe("followOffsetYPx", () => {
  it("offsets the look-ahead center below mid-screen within viewport limits", () => {
    expect(followOffsetYPx(800)).toBe(120);
    expect(followOffsetYPx(900)).toBe(135);
    expect(followOffsetYPx(400)).toBe(64);
    // Very tall layouts are capped.
    expect(followOffsetYPx(1600)).toBe(160);
    expect(followOffsetYPx(Number.NaN)).toBe(120);
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
      duration: 1800,
    });
  });

  it("fails safely when no camera can be computed", () => {
    const map = mockMap();
    map.cameraForBounds.mockReturnValue(undefined);

    expect(overviewCamera(asFollowableMap(map), { min: [106.7, 10.776], max: [106.7, 10.776] })).toBe(false);
    expect(map.easeTo).not.toHaveBeenCalled();
  });
});
