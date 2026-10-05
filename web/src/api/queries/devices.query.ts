import { useQuery } from "@tanstack/vue-query";
import { computed, toValue, type MaybeRefOrGetter } from "vue";
import type { components } from "../generated/lifetrail-v1";
import { api } from "../client";
import { ApiRequestError } from "../errors/api-error";
import { queryKeys } from "../query-keys";

export type Device = components["schemas"]["Device"];

export async function listDevices(): Promise<Device[]> {
  const { data, error, response } = await api.GET("/api/v1/devices");
  if (!data) {
    throw new ApiRequestError(response.status, error?.error.message ?? "Không thể tải Devices.");
  }
  return data.devices;
}

export async function getDevice(deviceId: string): Promise<Device> {
  const { data, error, response } = await api.GET("/api/v1/devices/{deviceId}", {
    params: { path: { deviceId } },
  });
  if (!data) {
    throw new ApiRequestError(response.status, error?.error.message ?? "Không thể tải Device.");
  }
  return data;
}

export function useDevices() {
  return useQuery({ queryKey: queryKeys.devices, queryFn: listDevices });
}

export function useDevice(deviceId: MaybeRefOrGetter<string>) {
  return useQuery({
    queryKey: computed(() => queryKeys.device(toValue(deviceId))),
    queryFn: () => getDevice(toValue(deviceId)),
  });
}
