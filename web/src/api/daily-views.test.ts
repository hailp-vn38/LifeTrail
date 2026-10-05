import { describe, expect, it, vi } from "vitest";
const { get } = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock("./client", () => ({ api: { GET: get } }));
import { getDailyView } from "./daily-views";
import { ApiRequestError } from "./request-error";

const deviceId = "8f467f83-f8e0-4d91-8f73-26e77e8fb0b5";

describe("getDailyView", () => {
  it("returns the valid zero-data Daily View instead of treating it as not found", async () => {
    get.mockResolvedValue({
      data: {
        device_id: deviceId,
        date: "2026-10-04",
        timezone: "Asia/Ho_Chi_Minh",
        processing_state: "raw",
        summary: { point_count: 0, distance_m: 0, duration_s: 0, first_fix_at: null, last_fix_at: null },
        route: null,
        start: null,
        end: null,
      },
      response: new Response(),
    });

    const dailyView = await getDailyView(deviceId, "2026-10-04");

    expect(dailyView.route).toBeNull();
    expect(get).toHaveBeenCalledWith("/api/v1/devices/{deviceId}/days/{date}", {
      params: { path: { deviceId, date: "2026-10-04" } },
    });
  });

  it("preserves a nonexistent Device as a distinct 404 error", async () => {
    get.mockResolvedValue({
      error: { error: { code: "not_found", message: "Not found", request_id: "req_1" } },
      response: new Response(null, { status: 404 }),
    });

    await expect(getDailyView(deviceId, "2026-10-04")).rejects.toMatchObject({ status: 404 });
  });
});
