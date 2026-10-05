import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import DailySummary from "./DailySummary.vue";

describe("DailySummary", () => {
  it("renders the raw Daily View summary", () => {
    const wrapper = mount(DailySummary, {
      props: {
        dailyView: {
          device_id: "8f467f83-f8e0-4d91-8f73-26e77e8fb0b5",
          date: "2026-10-04",
          timezone: "Asia/Ho_Chi_Minh",
          processing_state: "raw",
          summary: {
            point_count: 2,
            distance_m: 1284,
            duration_s: 3660,
            first_fix_at: "2026-10-04T00:00:00Z",
            last_fix_at: "2026-10-04T01:01:00Z",
          },
          route: null,
          start: null,
          end: null,
        },
      },
    });

    expect(wrapper.text()).toContain("2");
    expect(wrapper.text()).toContain("1.28 km");
    expect(wrapper.text()).toContain("1 giờ 1 phút");
  });
});
