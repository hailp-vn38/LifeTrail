import { LngLat, LngLatBounds, type JumpToOptions } from "maplibre-gl";
import { normalizeBearing, type RouteBounds } from "./geometry";
import type { MapCoordinate } from "./types";

/**
 * Camera policy for route playback. MapLibre GL JS is the camera API;
 * no second camera abstraction is introduced here. No playback clock either:
 * throttling uses a caller-injectable clock only.
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

export const FOLLOW_ZOOM = 16.5;
export const FOLLOW_PITCH = 45;
/** How often the follow camera may start a new transition. */
export const FOLLOW_UPDATE_INTERVAL_MS = 200;
/** Follow transition length, slightly longer than the update interval for smoothness. */
export const FOLLOW_TRANSITION_MS = 320;
/** Transition into follow mode when playback starts. */
export const FOLLOW_ENTER_TRANSITION_MS = 700;
/** Hold the final frame before zooming out to the overview. */
export const FINISH_HOLD_MS = 700;
/** Overview transition back to the whole route. */
export const OVERVIEW_TRANSITION_MS = 1600;
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
 * Vertical offset (px) that keeps the moving point slightly below the viewport
 * center so more of the route ahead stays visible. Bounded by viewport height
 * for narrow/mobile layouts; the exact value should be verified visually.
 */
export function followOffsetYPx(viewportHeightPx: number): number {
  if (!Number.isFinite(viewportHeightPx) || viewportHeightPx <= 0) return 80;
  return Math.min(80, Math.max(32, Math.round(viewportHeightPx * 0.12)));
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

  constructor(
    private readonly map: FollowableMap,
    options: FollowCameraOptions = {},
  ) {
    this.zoom = options.zoom ?? FOLLOW_ZOOM;
    this.pitch = options.pitch ?? FOLLOW_PITCH;
    this.offsetYPx = options.offsetYPx ?? 80;
    this.updateIntervalMs = options.updateIntervalMs ?? FOLLOW_UPDATE_INTERVAL_MS;
    this.transitionMs = options.transitionMs ?? FOLLOW_TRANSITION_MS;
    this.now = options.now ?? (() => performance.now());
  }

  /** Transition from the overview into follow mode at the given position. */
  enter(position: MapCoordinate, bearing: number): void {
    this.lastUpdateMs = this.now();
    this.map.easeTo({
      center: position,
      zoom: this.zoom,
      pitch: this.pitch,
      bearing: normalizeBearing(bearing),
      offset: [0, this.offsetYPx],
      duration: this.transitionDuration(FOLLOW_ENTER_TRANSITION_MS),
    });
  }

  /**
   * Nudge the follow camera toward the moving position. Safe to call on every
   * animation frame: at most one transition starts per update interval.
   */
  update(position: MapCoordinate, bearing: number): void {
    const now = this.now();
    if (now - this.lastUpdateMs < this.updateIntervalMs) return;
    this.lastUpdateMs = now;
    this.map.easeTo({
      center: position,
      zoom: this.zoom,
      pitch: this.pitch,
      bearing: normalizeBearing(bearing),
      offset: [0, this.offsetYPx],
      duration: this.transitionDuration(this.transitionMs),
    });
  }

  /** Stop any in-flight camera animation (used on unmount). */
  dispose(): void {
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
