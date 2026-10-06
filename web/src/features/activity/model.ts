/**
 * Published activity model shared by the Timeline and the Map.
 *
 * `DailyView.timeline` mixes Trips and Stops, so both consumers narrow through
 * these helpers instead of asserting the presence of Stop-only fields.
 */
import type { components } from "../../api/generated/lifetrail-v1";
import type { DailyView } from "../../api/queries/daily-view.query";

export type DailyActivity = components["schemas"]["DailyActivity"];
export type DailyStop = components["schemas"]["DailyStop"];
export type DailyTrip = components["schemas"]["DailyTrip"];
export type MovementSegment = components["schemas"]["MovementSegment"];
export type RoutePart = components["schemas"]["RoutePart"];

export function stopActivities(dailyView: DailyView): DailyStop[] {
  return (dailyView.timeline ?? []).filter((item): item is DailyStop => item.kind === "stop");
}

export function tripActivities(dailyView: DailyView): DailyTrip[] {
  return (dailyView.timeline ?? []).filter((item): item is DailyTrip => item.kind === "trip");
}

/** Route Parts belonging to one Trip, in published movement order. */
export function tripParts(dailyView: DailyView, tripId: string): RoutePart[] {
  return (dailyView.route_parts ?? []).filter((part) => part.trip_id === tripId);
}

/**
 * Total length of a Trip's published parts.
 *
 * The server owns the progress metric: this sums the published part lengths
 * instead of recomputing distance from the coordinates.
 */
export function tripDistanceM(dailyView: DailyView, tripId: string): number {
  return tripParts(dailyView, tripId).reduce((total, part) => total + part.distance_m, 0);
}
