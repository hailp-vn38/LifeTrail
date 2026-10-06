/** Playback lifecycle states. See the state diagram in the implementation guide. */
export type PlaybackState = "idle" | "playing" | "paused" | "finished";

/** A published interval the cursor must not interpolate through. */
export type PlaybackBreak = "disconnected" | "gps-gap" | "evidence-hole";

/** A [longitude, latitude] pair. */
export type MapCoordinate = [number, number];

/** One GPS fix normalized for playback: position plus GPS time. */
export interface PlaybackPoint {
  coordinate: MapCoordinate;
  /** Milliseconds since the Unix epoch, parsed from the RFC 3339 timestamp. */
  recordedAtMs: number;
  /** The interval to the next observed point has no drawable continuity. */
  breakToNext?: PlaybackBreak;
}

/** Where the playback cursor sits inside the point list. */
export interface SegmentLocation {
  /** Index of the segment's start vertex, always in [0, points.length - 2]. */
  vertexIndex: number;
  /** Interpolation ratio inside the segment, clamped to [0, 1]. */
  segmentRatio: number;
  /** Interpolated [longitude, latitude] at the cursor. */
  position: MapCoordinate;
  /** Coverage disclosure for a held, non-interpolated interval. */
  interruption?: PlaybackBreak;
}

/** One rendered tick of playback. Emitted by the controller, consumed by the map. */
export interface PlaybackFrame extends SegmentLocation {
  state: PlaybackState;
  /** Logical cursor time, milliseconds since the configured clock start. */
  routeTimeMs: number;
  /** Total logical duration of the configured playback clock. */
  durationMs: number;
  /** routeTimeMs / durationMs clamped to [0, 1]. */
  progress: number;
  /** Compass bearing of travel in degrees, normalized to [0, 360). */
  bearing: number;
}
