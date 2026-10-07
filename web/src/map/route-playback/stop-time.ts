export interface PlaybackStop {
  startTimeMs: number;
  endTimeMs: number;
}

/** Clip published Stops to the playback clock and merge overlapping intervals. */
export function playbackStops(stops: PlaybackStop[], start: number, end: number): PlaybackStop[] {
  const intervals = stops
    .filter(stop => Number.isFinite(stop.startTimeMs) && Number.isFinite(stop.endTimeMs))
    .map(stop => ({ startTimeMs: Math.max(start, stop.startTimeMs) - start, endTimeMs: Math.min(end, stop.endTimeMs) - start }))
    .filter(stop => stop.endTimeMs > stop.startTimeMs)
    .sort((a, b) => a.startTimeMs - b.startTimeMs);
  const merged: PlaybackStop[] = [];
  for (const interval of intervals) {
    const previous = merged.at(-1);
    if (previous && interval.startTimeMs <= previous.endTimeMs) {
      previous.endTimeMs = Math.max(previous.endTimeMs, interval.endTimeMs);
    } else {
      merged.push(interval);
    }
  }
  return merged;
}

/** Spend the elapsed playback time on movement, retaining the real GPS clock. */
export function advanceWithoutStops(time: number, delta: number, stops: PlaybackStop[]): number {
  let next = time + delta;
  for (const stop of stops) {
    if (stop.endTimeMs <= time) continue;
    if (next < stop.startTimeMs) break;
    next += stop.endTimeMs - Math.max(time, stop.startTimeMs);
    time = stop.endTimeMs;
  }
  return next;
}
