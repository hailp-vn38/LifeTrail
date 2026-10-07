import { createPinia, setActivePinia } from "pinia";
import { describe, expect, it, beforeEach } from "vitest";
import { usePlaybackStore } from "./playback.store";

describe("usePlaybackStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("starts from the idle Phase 2 model", () => {
    const store = usePlaybackStore();

    expect(store.status).toBe("idle");
    expect(store.currentTimeMs).toBe(0);
    expect(store.cameraFollow).toBe(false);
    expect(store.durationMs).toBe(0);
  });

  it("tracks route bounds, frames, speed and follow mode", () => {
    const store = usePlaybackStore();

    store.initialize({ startTimeMs: 1_000, endTimeMs: 61_000 });
    expect(store.durationMs).toBe(60_000);

    store.setFrame("playing", 15_000);
    expect(store.status).toBe("playing");
    expect(store.currentTimeMs).toBe(15_000);

    store.setSpeed(100);
    expect(store.speed).toBe(100);

    store.setCameraFollow(true);
    expect(store.cameraFollow).toBe(true);

    store.setFrame("finished", 60_000);
    store.setCameraFollow(false);
    expect(store.cameraFollow).toBe(false);

    store.reset();
    expect(store.status).toBe("idle");
    expect(store.currentTimeMs).toBe(0);
  });
});
