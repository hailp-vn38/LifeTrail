import type { PlaybackState as PlaybackStatus } from "../../map/route-playback/types";
import { PLAYBACK_SPEEDS } from "../../map/route-playback/controller";

export type PlaybackSpeed = (typeof PLAYBACK_SPEEDS)[number];

/**
 * Phase 2 playback state model (Option 2 spec §21).
 *
 * Named `PlaybackModel` to avoid clashing with the established
 * `PlaybackState` status union in `map/route-playback/types.ts`.
 *
 * The store holds only state shared between playback, timeline and map.
 * Animation-frame rendering stays in the playback/map composables and the
 * `PlaybackController`; the store is patched from controller frames.
 */
export interface PlaybackModel {
  status: PlaybackStatus;
  currentTimeMs: number;
  startTimeMs: number;
  endTimeMs: number;
  speed: PlaybackSpeed;
  cameraFollow: boolean;
  selectedEventId: string | null;
}

export function initialPlaybackModel(): PlaybackModel {
  return {
    status: "idle",
    currentTimeMs: 0,
    startTimeMs: 0,
    endTimeMs: 0,
    speed: 1,
    cameraFollow: false,
    selectedEventId: null,
  };
}
