import { LngLat, LngLatBounds, type JumpToOptions } from "maplibre-gl";
import { normalizeBearing, type RouteBounds } from "./geometry";
import type { MapCoordinate } from "./types";

/**
 * Course-Up Follow Camera with Look-Ahead.
 *
 * During playback the camera tracks the route instead of the map staying
 * static while a marker moves across it:
 *
 * - `bearing` follows the GPS course, so the road ahead always points up
 *   on screen (course-up / heading-up);
 * - `center` aims at a look-ahead point ahead on the route (not the
 *   current GPS fix), so the viewer sees more of the road about to be
 *   travelled;
 * - the current-position (heading puck) marker sits below the camera
 *   center, at roughly 70% of the viewport height.
 *
 * When playback ends the camera exits follow mode: bearing and pitch
 * reset to 0 and the full route is framed again.
 *
 * MapLibre GL JS is the camera API; no second camera abstraction is
 * introduced here. No playback clock either: throttling uses a
 * caller-injectable clock only.
 */

/** Minimal surface of MapLibreMap used by the camera policy (mockable in tests). */
export interface FollowableMap {
  easeTo(options: CameraEaseOptions): unknown;
  stop(): unknown;
  cameraForBounds(
    bounds: LngLatBounds,
    options?: { padding?: PaddingOptions; maxZoom?: number },
  ): JumpToOptions | undefined | null;
}

export interface CameraEaseOptions {
  center: MapCoordinate;
  zoom?: number;
  bearing?: number;
  pitch?: number;
  offset?: [number, number];
  duration?: number;
}

export interface PaddingOptions {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export const FOLLOW_ZOOM = 17;
export const FOLLOW_PITCH = 55;
/** Default distance ahead on the route the camera aims at, in meters. */
export const LOOK_AHEAD_DISTANCE_M = 40;
/** Wall-clock smoothing: stable at different frame rates and playback speeds. */
export const BEARING_SMOOTHING_MS = 400;
/** How often the follow camera may start a new transition. */
export const FOLLOW_UPDATE_INTERVAL_MS = 200;
/** Follow transition length, slightly longer than the update interval for smoothness. */
export const FOLLOW_TRANSITION_MS = 500;
/** Transition into follow mode when playback starts. */
export const FOLLOW_ENTER_TRANSITION_MS = 700;
/** Hold the final frame before zooming out to the overview. */
export const FINISH_HOLD_MS = 700;
/** Overview transition back to the whole route. */
export const OVERVIEW_TRANSITION_MS = 1800;
export const OVERVIEW_MAX_ZOOM = 15;
export const OVERVIEW_PADDING: PaddingOptions = { top: 64, right: 48, bottom: 96, left: 48 };

export function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

/**
 * Vertical offset (px) applied to the follow camera center so the heading
 * look-ahead center sits below screen center; the puck lands around 70%
 * depending on the projected distance to the look-ahead point.
 * Combined with the look-ahead target this keeps the upcoming road in the
 * upper part of the screen. Bounded for very small/large viewports.
 */
export function followOffsetYPx(viewportHeightPx: number): number {
  if (!Number.isFinite(viewportHeightPx) || viewportHeightPx <= 0) return 120;
  return Math.min(160, Math.max(64, Math.round(viewportHeightPx * 0.15)));
}

/**
 * Follow target for one camera update.
 *
 * `position` is the current GPS fix (the heading puck stays here);
 * `lookAhead` is the point ahead on the route the camera aims at.
 */
export interface FollowTarget {
  position: MapCoordinate;
  lookAhead: MapCoordinate;
  bearing: number;
}

export interface FollowCameraOptions {
  zoom?: number;
  pitch?: number;
  /** Pixels the position sits below the viewport center. */
  offsetYPx?: number;
  updateIntervalMs?: number;
  transitionMs?: number;
  now?: () => number;
}

/** Close navigation-like follow camera with throttled transitions. */
export class FollowCamera {
  private readonly zoom: number;
  private readonly pitch: number;
  private readonly offsetYPx: number;
  private readonly updateIntervalMs: number;
  private readonly transitionMs: number;
  private readonly now: () => number;
  private lastUpdateMs = Number.NEGATIVE_INFINITY;
  private bearing = 0;

  constructor(
    private readonly map: FollowableMap,
    options: FollowCameraOptions = {},
  ) {
    this.zoom = options.zoom ?? FOLLOW_ZOOM;
    this.pitch = options.pitch ?? FOLLOW_PITCH;
    this.offsetYPx = options.offsetYPx ?? 120;
    this.updateIntervalMs = options.updateIntervalMs ?? FOLLOW_UPDATE_INTERVAL_MS;
    this.transitionMs = options.transitionMs ?? FOLLOW_TRANSITION_MS;
    this.now = options.now ?? (() => performance.now());
  }

  /** Transition from the overview into follow mode at the given target. */
  enter(target: FollowTarget): void {
    this.lastUpdateMs = this.now();
    this.bearing = normalizeBearing(target.bearing);
    this.map.easeTo({
      center: target.lookAhead,
      zoom: this.zoom,
      pitch: this.pitch,
      bearing: this.bearing,
      offset: [0, this.offsetYPx],
      duration: this.transitionDuration(FOLLOW_ENTER_TRANSITION_MS),
    });
  }

  /**
   * Nudge the follow camera toward the moving target. Safe to call on every
   * animation frame: at most one transition starts per update interval.
   */
  update(target: FollowTarget): void {
    const now = this.now();
    if (now - this.lastUpdateMs < this.updateIntervalMs) return;
    const elapsed = now - this.lastUpdateMs;
    const delta = ((target.bearing - this.bearing + 540) % 360 + 360) % 360 - 180;
    const weight = 1 - Math.exp(-elapsed / BEARING_SMOOTHING_MS);
    this.bearing = normalizeBearing(this.bearing + delta * weight);
    this.lastUpdateMs = now;
    this.map.easeTo({
      center: target.lookAhead,
      zoom: this.zoom,
      pitch: this.pitch,
      bearing: this.bearing,
      offset: [0, this.offsetYPx],
      duration: this.transitionDuration(this.transitionMs),
    });
  }

  /** Stop any in-flight camera animation (used on unmount). */
  dispose(): void {
    this.stop();
  }

  /** Freeze the camera when playback pauses. */
  stop(): void {
    this.map.stop();
  }

  private transitionDuration(preferredMs: number): number {
    return prefersReducedMotion() ? 0 : preferredMs;
  }
}

export interface OverviewCameraOptions {
  padding?: PaddingOptions;
  maxZoom?: number;
  transitionMs?: number;
}

/**
 * Animate to an overview of the complete route with a north-up, top-down
 * orientation (bearing 0, pitch 0). Returns false when MapLibre cannot
 * compute a camera for the bounds, in which case the caller keeps the final
 * playback frame.
 */
export function overviewCamera(
  map: FollowableMap,
  bounds: RouteBounds,
  options: OverviewCameraOptions = {},
): boolean {
  const camera = map.cameraForBounds(new LngLatBounds(bounds.min, bounds.max), {
    padding: options.padding ?? OVERVIEW_PADDING,
    maxZoom: options.maxZoom ?? OVERVIEW_MAX_ZOOM,
  });
  if (!camera?.center || camera.zoom === undefined) return false;
  const center = LngLat.convert(camera.center);
  map.easeTo({
    center: [center.lng, center.lat],
    zoom: camera.zoom,
    bearing: 0,
    pitch: 0,
    duration: prefersReducedMotion() ? 0 : (options.transitionMs ?? OVERVIEW_TRANSITION_MS),
  });
  return true;
}
