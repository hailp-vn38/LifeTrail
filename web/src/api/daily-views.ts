import type { components } from "./generated/lifetrail-v1";
import { api } from "./client";
import { ApiRequestError } from "./request-error";

export type DailyView = components["schemas"]["DailyView"];

export async function getDailyView(deviceId: string, date: string): Promise<DailyView> {
  const { data, error, response } = await api.GET(
    "/api/v1/devices/{deviceId}/days/{date}",
    { params: { path: { deviceId, date } } },
  );
  if (!data) {
    throw new ApiRequestError(response.status, error?.error.message ?? "Không thể tải Daily View.");
  }
  return data;
}
