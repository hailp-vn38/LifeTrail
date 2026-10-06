import { useQuery } from "@tanstack/vue-query";
import { computed, toValue, type MaybeRefOrGetter } from "vue";
import type { components } from "../generated/lifetrail-v1";
import { api } from "../client";
import { ApiRequestError } from "../errors/api-error";
import { queryKeys } from "../query-keys";

export type DailyView = components["schemas"]["DailyView"];
export type ProcessingStatus = components["schemas"]["ProcessingStatus"];

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

export async function getDailyStatus(deviceId: string, date: string): Promise<ProcessingStatus> {
  const { data, error, response } = await api.GET("/api/v1/devices/{deviceId}/days/{date}/status", {
    params: { path: { deviceId, date } },
  });
  if (!data) throw new ApiRequestError(response.status, error?.error.message ?? "Không thể tải trạng thái xử lý.");
  return data;
}

/** Poll only the small status response while a visible page has active work. */
export function useDailyStatus(deviceId: MaybeRefOrGetter<string>, date: MaybeRefOrGetter<string>) {
  return useQuery({
    queryKey: computed(() => queryKeys.dailyStatus(toValue(deviceId), toValue(date))),
    queryFn: () => getDailyStatus(toValue(deviceId), toValue(date)),
    refetchInterval: (query) => {
      const state = query.state.data?.state;
      return document.visibilityState === "visible" && (state === "queued" || state === "running") ? 5_000 : false;
    },
  });
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
