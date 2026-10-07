/**
 * Display-projection telemetry (Ticket 12). Log-only, never sent to the server.
 *
 * These numbers exist to compare the Daily Map payload against the canonical
 * Playback resource during rollout; they are not a contract.
 */
export function logMapMetrics(event: string, fields: Record<string, number | string | boolean>): void {
  console.debug(`[map-metrics] ${event}`, fields);
}

/** Simplified display vertices across every Route Part of a day. */
export function displayVertexCount(dailyView: { route_parts?: { display_geometry: { coordinates: unknown[] } }[] }): number {
  return (dailyView.route_parts ?? []).reduce(
    (total, part) => total + part.display_geometry.coordinates.length,
    0,
  );
}
