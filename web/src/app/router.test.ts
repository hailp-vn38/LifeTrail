import { describe, expect, it } from "vitest";
import { createMemoryHistory } from "vue-router";
import { createLifeTrailRouter } from "./router";

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
    expect(router.currentRoute.value.meta.nav).toBe("daily-map");
  });

  it("renders every main page inside the shared AppLayout", async () => {
    const router = createLifeTrailRouter(createMemoryHistory());

    for (const path of ["/", "/devices", "/timeline", "/reports", "/settings"]) {
      await router.push(path);
      await router.isReady();
      const matched = router.currentRoute.value.matched;
      expect(matched.length).toBeGreaterThan(0);
      // The root route renders AppLayout; the page is its child.
      expect(matched[0]?.path).toBe("/");
    }
  });

  it("keeps the device detail route under the devices nav section", async () => {
    const router = createLifeTrailRouter(createMemoryHistory());

    await router.push("/devices/8f467f83-f8e0-4d91-8f73-26e77e8fb0b5");
    await router.isReady();

    expect(router.currentRoute.value.name).toBe("device-detail");
    expect(router.currentRoute.value.meta.nav).toBe("devices");
  });

  it("redirects unknown paths to the overview", async () => {
    const router = createLifeTrailRouter(createMemoryHistory());

    await router.push("/nope/not-here");
    await router.isReady();

    expect(router.currentRoute.value.path).toBe("/");
  });
});
