import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { PlaybackState as PlaybackStatus } from "../map/route-playback/types";
import {
  initialPlaybackModel,
  type PlaybackModel,
  type PlaybackSpeed,
} from "../features/playback/playback.types";

/**
 * Shared playback state (Option 2 spec §21/§22).
 *
 * The `PlaybackController` drives animation frames; it patches this store
 * so the timeline, map and playback bar stay in sync without direct
 * component-to-component calls.
 */
export const usePlaybackStore = defineStore("playback", () => {
  const model = ref<PlaybackModel>(initialPlaybackModel());

  const status = computed(() => model.value.status);
  const currentTimeMs = computed(() => model.value.currentTimeMs);
  const startTimeMs = computed(() => model.value.startTimeMs);
  const endTimeMs = computed(() => model.value.endTimeMs);
  const speed = computed(() => model.value.speed);
  const cameraFollow = computed(() => model.value.cameraFollow);

  /** Duration of the loaded route in ms (GPS time, last - first). */
  const durationMs = computed(() =>
    Math.max(0, model.value.endTimeMs - model.value.startTimeMs),
  );

  function initialize(input: { startTimeMs: number; endTimeMs: number }): void {
    model.value = {
      ...initialPlaybackModel(),
      startTimeMs: input.startTimeMs,
      endTimeMs: input.endTimeMs,
    };
  }

  function setFrame(nextStatus: PlaybackStatus, nextTimeMs: number): void {
    model.value.status = nextStatus;
    model.value.currentTimeMs = nextTimeMs;
  }

  function setSpeed(nextSpeed: PlaybackSpeed): void {
    model.value.speed = nextSpeed;
  }

  function setCameraFollow(follow: boolean): void {
    model.value.cameraFollow = follow;
  }

  function setSelectedEventId(id: string | null): void {
    model.value.selectedEventId = id;
  }

  function reset(): void {
    model.value = initialPlaybackModel();
  }

  return {
    model,
    status,
    currentTimeMs,
    startTimeMs,
    endTimeMs,
    durationMs,
    speed,
    cameraFollow,
    initialize,
    setFrame,
    setSpeed,
    setCameraFollow,
    setSelectedEventId,
    reset,
  };
});
