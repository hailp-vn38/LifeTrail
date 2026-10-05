import { describe, expect, it } from "vitest";
import { createMemoryHistory } from "vue-router";
import { createLifeTrailRouter } from "./index";

describe("LifeTrail router", () => {
  it("keeps the Device and Owner-local date from a Daily View URL", async () => {
    const router = createLifeTrailRouter(createMemoryHistory());

    await router.push("/devices/8f467f83-f8e0-4d91-8f73-26e77e8fb0b5/day/2026-10-04");
    await router.isReady();

    expect(router.currentRoute.value.name).toBe("daily-view");
    expect(router.currentRoute.value.params).toEqual({
      deviceId: "8f467f83-f8e0-4d91-8f73-26e77e8fb0b5",
      date: "2026-10-04",
    });
  });
});
