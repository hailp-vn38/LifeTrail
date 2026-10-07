import { mount, flushPromises } from "@vue/test-utils";
import { QueryClient, VueQueryPlugin } from "@tanstack/vue-query";
import { defineComponent, ref } from "vue";
import { describe, expect, it, vi } from "vitest";
import { getDailyView, type DailyView } from "../../../api/queries/daily-view.query";
import { useWeekData } from "./useWeekData";

vi.mock("../../../api/queries/daily-view.query", () => ({ getDailyView: vi.fn() }));

function view(device: string, date: string, count: number): DailyView {
  return { device_id: device, date, timezone: "Asia/Ho_Chi_Minh", processing_state: "raw",
    summary: { point_count: count, distance_m: 0, duration_s: 0, first_fix_at: null, last_fix_at: null },
    route: null, start: null, end: null };
}

describe("useWeekData", () => {
  it("reads Raw GPS availability, caches the week and isolates Device changes", async () => {
    vi.mocked(getDailyView).mockImplementation(async (device, date) => view(device, date, device === "device-1" && date === "2026-10-05" ? 1 : 0));
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const device = ref("device-1");
    const date = ref("2026-10-05");
    let week!: ReturnType<typeof useWeekData>;
    const wrapper = mount(defineComponent({ setup() { week = useWeekData(device, date, "Asia/Ho_Chi_Minh"); return () => null; } }), {
      global: { plugins: [[VueQueryPlugin, { queryClient: client }]] },
    });
    await flushPromises();
    await vi.waitFor(() => expect(week.daysWithData.value).toEqual(["2026-10-05"]));
    expect(getDailyView).toHaveBeenCalledTimes(7);
    expect(getDailyView).toHaveBeenCalledWith("device-1", "2026-10-05", true);
    date.value = "2026-10-07";
    await flushPromises();
    expect(getDailyView).toHaveBeenCalledTimes(7);
    device.value = "device-2";
    await flushPromises();
    await vi.waitFor(() => expect(getDailyView).toHaveBeenCalledTimes(14));
    expect(week.daysWithData.value).toEqual([]);
    vi.mocked(getDailyView).mockImplementation(async (device, date) => view(device, date, date === "2026-10-06" ? 5 : 0));
    await week.refresh();
    await flushPromises();
    await vi.waitFor(() => expect(week.daysWithData.value).toEqual(["2026-10-06"]));
    wrapper.unmount();
    client.clear();
  });
});
