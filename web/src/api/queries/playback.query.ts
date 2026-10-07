import { useQuery } from "@tanstack/vue-query";
import { computed, toValue, type MaybeRefOrGetter } from "vue";
import type { components } from "../generated/lifetrail-v1";
import { api } from "../client";
import { ApiRequestError } from "../errors/api-error";
import { queryKeys } from "../query-keys";

export type PlaybackView = components["schemas"]["PlaybackView"];
export type RoutePart = components["schemas"]["RoutePart"];

/**
 * Canonical full-resolution playback geometry for one Owner-local day.
 *
 * The Daily Snapshot only carries simplified display geometry, so playback is a
 * second, lazily-loaded resource. `manifestVersion` pins the response to the
 * manifest the client already holds; a 410 means the pin is gone and the caller
 * must drop its cache and reload without the pin.
 */
export async function getPlayback(
  deviceId: string,
  date: string,
  manifestVersion?: string,
): Promise<PlaybackView> {
  const { data, error, response } = await api.GET(
    "/api/v1/devices/{deviceId}/days/{date}/playback",
    {
      params: {
        path: { deviceId, date },
        ...(manifestVersion ? { query: { manifest_version: manifestVersion } } : {}),
      },
    },
  );
  if (!data) {
    throw new ApiRequestError(
      response.status,
      error?.error.message ?? "Không thể tải dữ liệu playback.",
    );
  }
  return data;
}

/**
 * Lazy playback query.
 *
 * `enabled` gates the request so nothing is fetched until playback is engaged.
 * The query key includes the manifest version, so a snapshot refresh loads a new
 * resource instead of reusing stale canonical geometry.
 */
export function usePlaybackQuery(
  deviceId: MaybeRefOrGetter<string>,
  date: MaybeRefOrGetter<string>,
  manifestVersion: MaybeRefOrGetter<string | undefined>,
  enabled: MaybeRefOrGetter<boolean>,
) {
  return useQuery({
    queryKey: computed(() =>
      queryKeys.playback(toValue(deviceId), toValue(date), toValue(manifestVersion)),
    ),
    queryFn: () =>
      getPlayback(toValue(deviceId), toValue(date), toValue(manifestVersion)),
    enabled: computed(() => toValue(enabled)),
  });
}
