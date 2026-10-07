import { routeBearing } from "./heading";
import { clampRouteTime, locateSegment, progressRatio, routeDurationMs } from "./timeline";
import type { PlaybackFrame, PlaybackPoint, PlaybackState } from "./types";
import { advanceWithoutStops, playbackStops, type PlaybackStop } from "./stop-time";

/**
 * Playback runtime: requestAnimationFrame loop, play/pause/restart, speed,
 * seek, state transitions and cleanup. It owns no Vue rendering and never
 * creates the MapLibre map instance; it only emits frames to callbacks.
 */

export const PLAYBACK_SPEEDS = [1, 10, 50, 100] as const;
export const DEFAULT_FINISH_HOLD_MS = 700;

/** Clock + timer surface. The default uses rAF/setTimeout; tests inject a fake. */
export interface PlaybackScheduler {
  now(): number;
  requestFrame(callback: (time: number) => void): number;
  cancelFrame(handle: number): void;
  delay(callback: () => void, ms: number): number;
  clearDelay(handle: number): void;
}

export const defaultPlaybackScheduler: PlaybackScheduler = {
  now: () => performance.now(),
  requestFrame: (callback) => requestAnimationFrame(callback),
  cancelFrame: (handle) => cancelAnimationFrame(handle),
  delay: (callback, ms) => window.setTimeout(callback, ms),
  clearDelay: (handle) => clearTimeout(handle),
};

export interface PlaybackControllerEvents {
  onFrame(frame: PlaybackFrame): void;
  /** Called finishHoldMs after the final timestamp is reached. */
  onOverviewReady?(): void;
}

export interface PlaybackControllerOptions {
  /** Validated playback points (at least 2, monotonic timestamps). */
  points: PlaybackPoint[];
  speed?: number;
  stops?: PlaybackStop[];
  skipStops?: boolean;
  /** Optional calendar-day clock; observations outside coverage are never invented. */
  startTimeMs?: number;
  endTimeMs?: number;
  finishHoldMs?: number;
  scheduler?: PlaybackScheduler;
  events: PlaybackControllerEvents;
}

export class PlaybackController {
  private readonly points: PlaybackPoint[];
  private readonly durationMs: number;
  private readonly startTimeMs: number;
  private readonly finishHoldMs: number;
  private readonly scheduler: PlaybackScheduler;
  private readonly events: PlaybackControllerEvents;

  private state: PlaybackState = "idle";
  private speed: number;
  private readonly stops: PlaybackStop[];
  private skipStops: boolean;
  private routeTimeMs = 0;
  private frameHandle: number | null = null;
  private lastTickMs: number | null = null;
  private holdHandle: number | null = null;
  private lastStableBearing = 0;
  private disposed = false;

  constructor(options: PlaybackControllerOptions) {
    if (options.points.length < 2) {
      throw new Error("PlaybackController needs at least 2 points.");
    }
    this.points = options.points;
    this.startTimeMs = options.startTimeMs ?? options.points[0].recordedAtMs;
    this.durationMs = options.endTimeMs === undefined ? routeDurationMs(options.points) : options.endTimeMs - this.startTimeMs;
    this.speed = options.speed && options.speed > 0 ? options.speed : 1;
    this.stops = playbackStops(options.stops ?? [], this.startTimeMs, this.startTimeMs + this.durationMs);
    this.skipStops = options.skipStops ?? false;
    this.finishHoldMs = options.finishHoldMs ?? DEFAULT_FINISH_HOLD_MS;
    this.scheduler = options.scheduler ?? defaultPlaybackScheduler;
    this.events = options.events;
  }

  get currentState(): PlaybackState {
    return this.state;
  }

  get currentSpeed(): number {
    return this.speed;
  }

  get currentRouteTimeMs(): number {
    return this.routeTimeMs;
  }

  get totalDurationMs(): number {
    return this.durationMs;
  }

  /** idle -> playing, paused -> playing. After finished, restarts from the start. */
  play(): void {
    if (this.disposed || this.state === "playing") return;
    if (this.state === "finished") {
      this.routeTimeMs = 0;
    }
    this.clearHold();
    this.state = "playing";
    this.lastTickMs = this.scheduler.now();
    if (this.skipStops) this.routeTimeMs = Math.min(this.durationMs, advanceWithoutStops(this.routeTimeMs, 0, this.stops));
    if (this.routeTimeMs >= this.durationMs) {
      this.finishPlayback();
      return;
    }
    this.emitFrame();
    this.scheduleTick();
  }

  /** playing -> paused. Keeps the route, marker and camera untouched. */
  pause(): void {
    if (this.disposed || this.state !== "playing") return;
    this.cancelTick();
    this.state = "paused";
    this.emitFrame();
  }

  /** Reset to the first GPS timestamp. Preferred UX: restart lands in idle. */
  restart(): void {
    if (this.disposed) return;
    this.cancelTick();
    this.clearHold();
    this.routeTimeMs = 0;
    this.state = "idle";
    this.emitFrame();
  }

  /**
   * Jump to a logical route time. Deterministic: the segment is found with
   * binary search (no replay from zero) and playback pauses during scrub.
   */
  seek(routeTimeMs: number): void {
    if (this.disposed) return;
    this.cancelTick();
    this.clearHold();
    this.routeTimeMs = clampRouteTime(routeTimeMs, this.durationMs);
    this.state = this.routeTimeMs >= this.durationMs ? "finished" : "paused";
    this.emitFrame();
  }

  setSpeed(speed: number): void {
    if (this.disposed || !(speed > 0)) return;
    this.speed = speed;
    if (this.state === "playing") {
      // Avoid a time jump from the interval spent at the old speed.
      this.lastTickMs = this.scheduler.now();
    }
  }

  setSkipStops(enabled: boolean): void {
    if (this.disposed) return;
    this.skipStops = enabled;
  }

  /** Cancel the rAF loop and the finish hold; no callbacks fire afterwards. */
  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.cancelTick();
    this.clearHold();
  }

  private scheduleTick(): void {
    this.cancelTick();
    this.frameHandle = this.scheduler.requestFrame((time) => this.tick(time));
  }

  private cancelTick(): void {
    if (this.frameHandle !== null) {
      this.scheduler.cancelFrame(this.frameHandle);
      this.frameHandle = null;
    }
    // NOTE: lastTickMs is intentionally preserved here. play() and tick()
    // always refresh it, and setSpeed() resets it; clearing it would make the
    // next tick compute a zero wall-clock delta.
  }

  private clearHold(): void {
    if (this.holdHandle !== null) {
      this.scheduler.clearDelay(this.holdHandle);
      this.holdHandle = null;
    }
  }

  private tick(nowMs: number): void {
    if (this.disposed || this.state !== "playing") return;
    const last = this.lastTickMs ?? nowMs;
    const wallDeltaMs = Math.max(0, nowMs - last);
    this.lastTickMs = nowMs;
    const delta = wallDeltaMs * this.speed;
    this.routeTimeMs = Math.min(this.durationMs, this.skipStops
      ? advanceWithoutStops(this.routeTimeMs, delta, this.stops)
      : this.routeTimeMs + delta);
    if (this.routeTimeMs >= this.durationMs) {
      this.finishPlayback();
      return;
    }
    this.emitFrame();
    this.scheduleTick();
  }

  private finishPlayback(): void {
    this.cancelTick();
    this.routeTimeMs = this.durationMs;
    this.state = "finished";
    this.emitFrame();
    this.holdHandle = this.scheduler.delay(() => {
      this.holdHandle = null;
      if (!this.disposed) {
        this.events.onOverviewReady?.();
      }
    }, this.finishHoldMs);
  }

  private emitFrame(): void {
    const location = locateSegment(this.points, this.routeTimeMs + this.startTimeMs - this.points[0].recordedAtMs);
    const segmentBearing = routeBearing(this.points, location.vertexIndex, location.position);
    if (segmentBearing !== null) {
      this.lastStableBearing = segmentBearing;
    }
    this.events.onFrame({
      state: this.state,
      routeTimeMs: this.routeTimeMs,
      durationMs: this.durationMs,
      progress: progressRatio(this.routeTimeMs, this.durationMs),
      position: location.position,
      bearing: this.lastStableBearing,
      vertexIndex: location.vertexIndex,
      segmentRatio: location.segmentRatio,
      interruption: location.interruption,
    });
  }
}
