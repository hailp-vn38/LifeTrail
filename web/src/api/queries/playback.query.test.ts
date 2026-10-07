import { describe, expect, it, vi } from "vitest";
const { get } = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock("../client", () => ({ api: { GET: get } }));
import { getPlayback } from "./playback.query";

const deviceId = "8f467f83-f8e0-4d91-8f73-26e77e8fb0b5";

describe("getPlayback", () => {
  it("requests the canonical playback resource for the day", async () => {
    get.mockResolvedValue({
      data: {
        device_id: deviceId,
        date: "2026-10-05",
        timezone: "Asia/Ho_Chi_Minh",
        manifest_version: "01900000-0000-7000-8000-000000000020",
        projection_schema_version: 2,
        route_parts: [],
        provenance: {},
      },
      response: new Response(),
    });

    await getPlayback(deviceId, "2026-10-05");

    expect(get).toHaveBeenCalledWith(
      "/api/v1/devices/{deviceId}/days/{date}/playback",
      { params: { path: { deviceId, date: "2026-10-05" } } },
    );
  });

  it("pins the manifest version when the client holds one", async () => {
    get.mockResolvedValue({ data: { route_parts: [] }, response: new Response() });
    const manifestVersion = "01900000-0000-7000-8000-000000000020";

    await getPlayback(deviceId, "2026-10-05", manifestVersion);

    expect(get).toHaveBeenCalledWith(
      "/api/v1/devices/{deviceId}/days/{date}/playback",
      { params: { path: { deviceId, date: "2026-10-05" }, query: { manifest_version: manifestVersion } } },
    );
  });

  it("surfaces a 410 so the caller can drop its cache", async () => {
    get.mockResolvedValue({
      error: { error: { code: "manifest_expired", message: "gone", request_id: "req_1" } },
      response: new Response(null, { status: 410 }),
    });

    await expect(getPlayback(deviceId, "2026-10-05")).rejects.toMatchObject({ status: 410 });
  });
});
