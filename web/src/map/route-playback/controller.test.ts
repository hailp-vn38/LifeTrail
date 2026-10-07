import { describe, expect, it, vi } from "vitest";
import { PlaybackController, type PlaybackScheduler, type PlaybackControllerOptions } from "./controller";
import type { PlaybackFrame, PlaybackPoint } from "./types";

/** Manual clock + timer queue: fully deterministic, no wall-clock sleeps. */
class FakeScheduler implements PlaybackScheduler {
  nowMs = 0;
  private frames = new Map<number, (time: number) => void>();
  private delays = new Map<number, { callback: () => void; at: number }>();
  private nextId = 1;

  now(): number {
    return this.nowMs;
  }

  requestFrame(callback: (time: number) => void): number {
    const id = this.nextId++;
    this.frames.set(id, callback);
    return id;
  }

  cancelFrame(handle: number): void {
    this.frames.delete(handle);
  }

  delay(callback: () => void, ms: number): number {
    const id = this.nextId++;
    this.delays.set(id, { callback, at: this.nowMs + ms });
    return id;
  }

  clearDelay(handle: number): void {
    this.delays.delete(handle);
  }

  pendingFrames(): number {
    return this.frames.size;
  }

  pendingDelays(): number {
    return this.delays.size;
  }

  advance(ms: number): void {
    const target = this.nowMs + ms;
    for (;;) {
      let earliest: [number, { callback: () => void; at: number }] | undefined;
      for (const entry of this.delays) {
        if (entry[1].at <= target && (!earliest || entry[1].at < earliest[1].at)) {
          earliest = entry;
        }
      }
      if (!earliest) break;
      this.delays.delete(earliest[0]);
      this.nowMs = earliest[1].at;
      earliest[1].callback();
    }
    this.nowMs = target;
  }

  runFrames(): void {
    const callbacks = [...this.frames.values()];
    this.frames.clear();
    for (const callback of callbacks) callback(this.nowMs);
  }
}

/** Irregular GPS sampling: 3s, 22s, 5s segments. */
function fixturePoints(): PlaybackPoint[] {
  return [
    { coordinate: [106.7, 10.776], recordedAtMs: 0 },
    { coordinate: [106.7005, 10.7764], recordedAtMs: 3_000 },
    { coordinate: [106.701, 10.777], recordedAtMs: 25_000 },
    { coordinate: [106.7015, 10.7774], recordedAtMs: 30_000 },
  ];
}

interface Harness {
  controller: PlaybackController;
  scheduler: FakeScheduler;
  frames: PlaybackFrame[];
  onOverviewReady: () => void;
}

function harness(points: PlaybackPoint[] = fixturePoints(), speed = 1,
  options: Pick<PlaybackControllerOptions, "stops" | "skipStops" | "startTimeMs" | "endTimeMs"> = {},
): Harness {
  const scheduler = new FakeScheduler();
  const frames: PlaybackFrame[] = [];
  const onOverviewReady = vi.fn();
  const controller = new PlaybackController({
    points,
    speed,
    ...options,
    scheduler,
    events: { onFrame: (frame) => frames.push(frame), onOverviewReady },
  });
  return { controller, scheduler, frames, onOverviewReady };
}

describe("PlaybackController", () => {
  it("keeps Stop durations by default and allows toggling during playback", () => {
    const { controller, scheduler } = harness(fixturePoints(), 1, {
      stops: [{ startTimeMs: 3_000, endTimeMs: 25_000 }],
    });
    controller.play();
    scheduler.advance(5_000);
    scheduler.runFrames();
    expect(controller.currentRouteTimeMs).toBe(5_000);
    controller.setSkipStops(true);
    scheduler.advance(1_000);
    scheduler.runFrames();
    expect(controller.currentRouteTimeMs).toBe(26_000);
    controller.setSkipStops(false);
    controller.seek(5_000);
    controller.play();
    scheduler.advance(1_000);
    scheduler.runFrames();
    expect(controller.currentRouteTimeMs).toBe(6_000);
  });

  it("skips Stops on the real day clock while preserving manual seeks and speed", () => {
    const { controller, scheduler, frames } = harness(fixturePoints(), 2, {
      startTimeMs: -10_000, endTimeMs: 40_000, skipStops: true,
      stops: [{ startTimeMs: 3_000, endTimeMs: 25_000 }],
    });
    controller.seek(15_000);
    expect(controller.currentRouteTimeMs).toBe(15_000);
    controller.play();
    expect(controller.currentRouteTimeMs).toBe(35_000);
    scheduler.advance(1_000);
    scheduler.runFrames();
    expect(controller.currentRouteTimeMs).toBe(37_000);
    expect(frames.at(-1)?.vertexIndex).toBe(2);
  });

  it("finishes normally when skipped Stops reach the end and skips again on restart", () => {
    const { controller, scheduler, onOverviewReady } = harness(fixturePoints(), 1, {
      skipStops: true, stops: [{ startTimeMs: 3_000, endTimeMs: 30_000 }],
    });
    controller.play();
    scheduler.advance(3_000);
    scheduler.runFrames();
    expect(controller.currentState).toBe("finished");
    expect(controller.currentRouteTimeMs).toBe(30_000);
    scheduler.advance(700);
    expect(onOverviewReady).toHaveBeenCalledOnce();
    controller.restart();
    controller.play();
    scheduler.advance(3_000);
    scheduler.runFrames();
    expect(controller.currentState).toBe("finished");
  });

  it("follows east then south even when GPS records are less than 5m apart", () => {
    const points: PlaybackPoint[] = [
      ...Array.from({ length: 11 }, (_, i) => ({
        coordinate: [106.7 + i * 0.00002, 10.776] as [number, number],
        recordedAtMs: i * 1000,
      })),
      ...Array.from({ length: 10 }, (_, i) => ({
        coordinate: [106.7002, 10.776 - (i + 1) * 0.00002] as [number, number],
        recordedAtMs: (i + 11) * 1000,
      })),
    ];
    const { controller, frames } = harness(points);
    controller.play();
    expect(frames.at(-1)?.bearing).toBeCloseTo(90, 1);
    controller.seek(10_000);
    expect(frames.at(-1)?.bearing).toBeCloseTo(180, 1);
    controller.seek(20_000);
    expect(frames.at(-1)?.bearing).toBeCloseTo(180, 1);
    controller.restart();
    expect(frames.at(-1)?.bearing).toBeCloseTo(90, 1);
  });

  it("transitions idle -> playing and emits the start frame", () => {
    const { controller, frames } = harness();
    expect(controller.currentState).toBe("idle");

    controller.play();

    expect(controller.currentState).toBe("playing");
    expect(frames).toHaveLength(1);
    expect(frames[0]).toMatchObject({
      state: "playing",
      routeTimeMs: 0,
      progress: 0,
      position: [106.7, 10.776],
    });
  });

  it("advances playback by GPS time, not by vertex count", () => {
    const { controller, scheduler, frames } = harness();
    controller.play();

    scheduler.advance(4_000);
    scheduler.runFrames();

    const last = frames[frames.length - 1];
    expect(last.routeTimeMs).toBe(4_000);
    // 4s into the track is inside the long 3s..25s segment.
    expect(last.vertexIndex).toBe(1);
    expect(last.segmentRatio).toBeCloseTo(1 / 22, 6);
  });

  it("keeps real segment durations for irregular sampling", () => {
    const { controller, scheduler, frames } = harness();
    controller.play();

    scheduler.advance(24_000);
    scheduler.runFrames();
    expect(frames[frames.length - 1].vertexIndex).toBe(1);

    scheduler.advance(1_000);
    scheduler.runFrames();
    const last = frames[frames.length - 1];
    expect(last.routeTimeMs).toBe(25_000);
    expect(last.vertexIndex).toBe(2);
    expect(last.position).toEqual([106.701, 10.777]);
  });

  it("pauses and resumes from the exact logical route time", () => {
    const { controller, scheduler, frames } = harness();
    controller.play();
    scheduler.advance(5_000);
    scheduler.runFrames();

    controller.pause();
    expect(controller.currentState).toBe("paused");
    const frameCount = frames.length;

    scheduler.advance(10_000);
    scheduler.runFrames();
    expect(frames).toHaveLength(frameCount);
    expect(controller.currentRouteTimeMs).toBe(5_000);

    controller.play();
    expect(controller.currentState).toBe("playing");
    scheduler.advance(1_000);
    scheduler.runFrames();
    expect(controller.currentRouteTimeMs).toBe(6_000);
  });

  it("finishes at the final timestamp, holds, then signals the overview", () => {
    const { controller, scheduler, frames, onOverviewReady } = harness();
    controller.play();

    scheduler.advance(60_000);
    scheduler.runFrames();

    const last = frames[frames.length - 1];
    expect(controller.currentState).toBe("finished");
    expect(last.routeTimeMs).toBe(30_000);
    expect(last.position).toEqual([106.7015, 10.7774]);
    expect(last.progress).toBe(1);
    expect(onOverviewReady).not.toHaveBeenCalled();

    scheduler.advance(699);
    expect(onOverviewReady).not.toHaveBeenCalled();
    scheduler.advance(1);
    expect(onOverviewReady).toHaveBeenCalledTimes(1);
  });

  it("restarts from the beginning when Play is pressed after finished", () => {
    const { controller, scheduler, frames } = harness();
    controller.play();
    scheduler.advance(60_000);
    scheduler.runFrames();
    expect(controller.currentState).toBe("finished");

    controller.play();
    expect(controller.currentState).toBe("playing");
    const last = frames[frames.length - 1];
    expect(last.routeTimeMs).toBe(0);
    expect(last.position).toEqual([106.7, 10.776]);
  });

  it("restart returns to idle at the first GPS timestamp", () => {
    const { controller, scheduler, frames } = harness();
    controller.play();
    scheduler.advance(5_000);
    scheduler.runFrames();

    controller.restart();

    expect(controller.currentState).toBe("idle");
    expect(controller.currentRouteTimeMs).toBe(0);
    const last = frames[frames.length - 1];
    expect(last.state).toBe("idle");
    expect(last.position).toEqual([106.7, 10.776]);
    expect(scheduler.pendingFrames()).toBe(0);
  });

  it("seeks with binary search and pauses during scrub", () => {
    const { controller, frames } = harness();
    controller.play();

    controller.seek(15_000);

    expect(controller.currentState).toBe("paused");
    expect(controller.currentRouteTimeMs).toBe(15_000);
    const last = frames[frames.length - 1];
    expect(last.vertexIndex).toBe(1);
    expect(last.segmentRatio).toBeCloseTo(12 / 22, 6);
  });

  it("clamps seeks outside the route", () => {
    const { controller } = harness();
    controller.seek(-100);
    expect(controller.currentRouteTimeMs).toBe(0);
    expect(controller.currentState).toBe("paused");

    controller.seek(9_999_999);
    expect(controller.currentRouteTimeMs).toBe(30_000);
    expect(controller.currentState).toBe("finished");
  });

  it("seeking away from the end cancels the pending overview", () => {
    const { controller, scheduler, onOverviewReady } = harness();
    controller.play();
    scheduler.advance(60_000);
    scheduler.runFrames();
    expect(controller.currentState).toBe("finished");

    controller.seek(1_000);
    scheduler.advance(5_000);
    expect(onOverviewReady).not.toHaveBeenCalled();
    expect(scheduler.pendingDelays()).toBe(0);
  });

  it("scales wall-clock speed without changing logical GPS timing", () => {
    const { controller, scheduler } = harness(fixturePoints(), 5);
    controller.play();

    scheduler.advance(1_000);
    scheduler.runFrames();
    expect(controller.currentRouteTimeMs).toBe(5_000);

    controller.setSpeed(2);
    scheduler.advance(1_000);
    scheduler.runFrames();
    expect(controller.currentRouteTimeMs).toBe(7_000);
  });

  it("uses the incoming route bearing when jumping straight to the final frame", () => {
    const { controller, scheduler, frames } = harness();
    controller.play();

    scheduler.advance(60_000);
    scheduler.runFrames();
    const last = frames[frames.length - 1];
    expect(last.bearing).toBeCloseTo(50.842, 1);
    expect(Number.isFinite(last.bearing)).toBe(true);
  });

  it("finishes immediately for a zero-duration route", () => {
    const { controller, scheduler, onOverviewReady } = harness([
      { coordinate: [106.7, 10.776], recordedAtMs: 1_000 },
      { coordinate: [106.7005, 10.7764], recordedAtMs: 1_000 },
    ]);
    controller.play();

    expect(controller.currentState).toBe("finished");
    scheduler.advance(700);
    expect(onOverviewReady).toHaveBeenCalledTimes(1);
  });

  it("dispose cancels the loop and the finish hold", () => {
    const { controller, scheduler, frames, onOverviewReady } = harness();
    controller.play();
    scheduler.advance(60_000);
    scheduler.runFrames();
    const frameCount = frames.length;

    controller.dispose();
    scheduler.advance(10_000);
    scheduler.runFrames();

    expect(frames).toHaveLength(frameCount);
    expect(onOverviewReady).not.toHaveBeenCalled();
    expect(scheduler.pendingFrames()).toBe(0);
    expect(scheduler.pendingDelays()).toBe(0);

    controller.play();
    expect(controller.currentState).toBe("finished");
  });

  it("rejects fewer than 2 points", () => {
    expect(
      () =>
        new PlaybackController({
          points: [{ coordinate: [106.7, 10.776], recordedAtMs: 0 }],
          scheduler: new FakeScheduler(),
          events: { onFrame: () => {} },
        }),
    ).toThrow();
  });
});
