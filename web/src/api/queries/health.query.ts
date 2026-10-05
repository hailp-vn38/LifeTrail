import { useQuery } from "@tanstack/vue-query";
import { api } from "../client";
import { queryKeys } from "../query-keys";

export interface SystemStatus {
  ok: boolean;
  deviceCount: number | null;
  checkedAt: number;
}

/**
 * Lightweight API reachability probe.
 *
 * There is no dedicated browser-facing health endpoint behind the web
 * reverse proxy (`/health/*` is not proxied), so the devices listing — a
 * stable, cheap Phase 1 endpoint — doubles as the status signal. It answers
 * "can the browser reach the API through Nginx", which is what the UI
 * status indicator needs.
 */
export async function getSystemStatus(): Promise<SystemStatus> {
  try {
    const { data, error, response } = await api.GET("/api/v1/devices");
    // openapi-fetch does not throw on HTTP errors — it populates `error`.
    // Only report ok when the response actually succeeded with a body.
    const ok = !error && response.ok && data != null;
    return { ok, deviceCount: ok ? data.devices.length : null, checkedAt: Date.now() };
  } catch {
    return { ok: false, deviceCount: null, checkedAt: Date.now() };
  }
}

export function useSystemStatus() {
  return useQuery({
    queryKey: queryKeys.systemStatus,
    queryFn: getSystemStatus,
    refetchInterval: 60_000,
  });
}
