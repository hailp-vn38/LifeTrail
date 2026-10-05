import type { components } from "./generated/lifetrail-v1";
import { api } from "./client";
import { ApiRequestError } from "./request-error";

export type Device = components["schemas"]["Device"];

export async function listDevices(): Promise<Device[]> {
  const { data, error, response } = await api.GET("/api/v1/devices");
  if (!data) {
    throw new ApiRequestError(response.status, error?.error.message ?? "Không thể tải Devices.");
  }
  return data.devices;
}
