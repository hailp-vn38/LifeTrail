import { bearingBetween, pointAheadOnRoute } from "./geometry";
import { LOOK_AHEAD_DISTANCE_M, type FollowTarget } from "./camera";
import type { PlaybackFrame, PlaybackPoint } from "./types";

/** Use the upcoming route to anticipate turns; retain the incoming heading at the end. */
export function followTarget(points: PlaybackPoint[], frame: PlaybackFrame): FollowTarget {
  const lookAhead = pointAheadOnRoute(
    points, frame.vertexIndex, frame.position, LOOK_AHEAD_DISTANCE_M,
  );
  return {
    position: frame.position,
    lookAhead,
    bearing: bearingBetween(frame.position, lookAhead) ?? frame.bearing,
  };
}
