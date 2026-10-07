/**
 * Published activity model shared by the Timeline and the Map.
 *
 * `DailyView.timeline` mixes Trips and Stops, so both consumers narrow through
 * these helpers instead of asserting the presence of Stop-only fields.
 */
import type { components } from "../../api/generated/lifetrail-v1";
import type { DailyView } from "../../api/queries/daily-view.query";

export type DailyActivity = components["schemas"]["DailyActivity"];
export type DailyGap = components["schemas"]["DailyGap"];
export type DailyStop = components["schemas"]["DailyStop"];
export type DailyTrip = components["schemas"]["DailyTrip"];
export type EvidenceHole = components["schemas"]["EvidenceHole"];
export type MovementSegment = components["schemas"]["MovementSegment"];
export type RoutePart = components["schemas"]["RoutePart"];
export type DailyRoutePart = components["schemas"]["DailyRoutePart"];
export type PlaybackView = components["schemas"]["PlaybackView"];

export function stopActivities(dailyView: DailyView): DailyStop[] {
  return (dailyView.timeline ?? []).filter((item): item is DailyStop => item.kind === "stop");
}

/**
 * GPS Gaps in the published Timeline.
 *
 * A Gap is the absence of Raw observations, so it has no geometry and no
 * location to focus. It is deliberately distinct from `evidence_holes`, which
 * describe intervals that contain unreliable observations.
 */
export function gapActivities(dailyView: DailyView): DailyGap[] {
  return (dailyView.timeline ?? []).filter((item): item is DailyGap => item.kind === "gap");
}

/** Intervals with Raw observations that cannot support reliable activity. */
export function evidenceHoles(dailyView: DailyView): EvidenceHole[] {
  return dailyView.evidence_holes ?? [];
}

export function tripActivities(dailyView: DailyView): DailyTrip[] {
  return (dailyView.timeline ?? []).filter((item): item is DailyTrip => item.kind === "trip");
}

/** Display Route Parts belonging to one Trip, in published movement order. */
export function tripDisplayParts(dailyView: DailyView, tripId: string): DailyRoutePart[] {
  return (dailyView.route_parts ?? []).filter((part) => part.trip_id === tripId);
}

/**
 * Total length of a Trip's published parts.
 *
 * The server owns the progress metric: this sums the published part lengths
 * instead of recomputing distance from the coordinates. Display geometry is
 * visualization-only, so `visible_distance_m` — not the coordinates — is the
 * source of truth.
 */
export function tripDistanceM(dailyView: DailyView, tripId: string): number {
  // A Route Part may cross a local-day boundary. `distance_m` is its complete
  // UTC-history length, while `visible_distance_m` is this Daily View's
  // anchored-progress slice. The timeline must not attribute the other day's
  // distance to the Owner's selected day.
  return tripDisplayParts(dailyView, tripId).reduce(
    (total, part) => total + part.visible_distance_m,
    0,
  );
}
