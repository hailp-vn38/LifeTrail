import { useQuery } from "@tanstack/vue-query";
import { computed, toValue, type MaybeRefOrGetter } from "vue";
import type { components } from "../generated/lifetrail-v1";
import { api } from "../client";
import { ApiRequestError } from "../errors/api-error";
import { queryKeys } from "../query-keys";

export type DailyView = components["schemas"]["DailyView"];

export async function getDailyView(deviceId: string, date: string, raw = false): Promise<DailyView> {
  const { data, error, response } = await api.GET("/api/v1/devices/{deviceId}/days/{date}", {
    params: { path: { deviceId, date }, ...(raw ? { query: { view: "raw" as const } } : {}) },
  });
  if (!data) {
    throw new ApiRequestError(
      response.status,
      error?.error.message ?? "Không thể tải Daily View.",
    );
  }
  return data;
}

export function useDailyView(
  deviceId: MaybeRefOrGetter<string>,
  date: MaybeRefOrGetter<string>,
  raw: MaybeRefOrGetter<boolean> = false,
) {
  return useQuery({
    queryKey: computed(() => queryKeys.dailyView(toValue(deviceId), toValue(date), toValue(raw))),
    queryFn: () => getDailyView(toValue(deviceId), toValue(date), toValue(raw)),
  });
}
