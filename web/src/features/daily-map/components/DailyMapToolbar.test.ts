import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";
import DailyMapToolbar from "./DailyMapToolbar.vue";

afterEach(() => vi.useRealTimers());

describe("DailyMapToolbar week calendar", () => {
  it("shows Monday through Sunday across a year boundary and selects a day", async () => {
    const wrapper = mount(DailyMapToolbar, { props: { date: "2027-01-01" } });
    const days = wrapper.findAll(".daily-map-toolbar__day");
    expect(days).toHaveLength(7);
    expect(days.map(day => day.get("strong").text())).toEqual(["28", "29", "30", "31", "1", "2", "3"]);
    expect(days[4].attributes("aria-current")).toBe("date");
    await days[6].trigger("click");
    expect(wrapper.emitted("date-change")).toEqual([["2027-01-03"]]);
    await wrapper.setProps({ date: "2027-01-03" });
    expect(wrapper.get('[aria-current="date"]').text()).toContain("CN");
    wrapper.unmount();
  });

  it("marks only days with GPS data, including the selected day", async () => {
    const wrapper = mount(DailyMapToolbar, { props: { date: "2026-10-05", daysWithData: ["2026-10-05", "2026-10-07"] } });
    const days = wrapper.findAll(".daily-map-toolbar__day");
    expect(days.map(day => day.find(".daily-map-toolbar__data-dot").exists())).toEqual([true, false, true, false, false, false, false]);
    expect(days[0].attributes("aria-label")).toContain("Có dữ liệu GPS");
    await wrapper.setProps({ daysWithData: ["2026-10-06"] });
    expect(wrapper.findAll(".daily-map-toolbar__data-dot")).toHaveLength(1);
    expect(days[1].find(".daily-map-toolbar__data-dot").exists()).toBe(true);
    wrapper.unmount();
  });

  it("moves by weeks", async () => {
    const wrapper = mount(DailyMapToolbar, { props: { date: "2026-10-06" } });
    await wrapper.get('[aria-label="Tuần trước"]').trigger("click");
    await wrapper.get('[aria-label="Tuần sau"]').trigger("click");
    expect(wrapper.emitted("date-change")).toEqual([["2026-09-29"], ["2026-10-13"]]);
    wrapper.unmount();
  });

  it("marks today in the Owner timezone separately from the selected date", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-10-06T18:00:00Z"));
    const wrapper = mount(DailyMapToolbar, { props: { date: "2026-10-05", timezone: "Asia/Ho_Chi_Minh" } });
    expect(wrapper.find(".daily-map-toolbar__today").exists()).toBe(false);
    expect(wrapper.get(".is-today strong").text()).toBe("7");
    expect(wrapper.get(".is-today").attributes("aria-label")).toContain("Hôm nay");
    expect(wrapper.get(".is-today").classes()).not.toContain("is-selected");
    await wrapper.setProps({ date: "2026-10-07" });
    expect(wrapper.get(".is-today").classes()).toContain("is-selected");
    await wrapper.setProps({ timezone: "America/Los_Angeles" });
    expect(wrapper.get(".is-today strong").text()).toBe("6");
    wrapper.unmount();
  });

  it("uses the Owner timezone when returning to today", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-10-06T18:00:00Z"));
    const wrapper = mount(DailyMapToolbar, { props: { date: "2026-09-28", timezone: "Asia/Ho_Chi_Minh" } });
    await wrapper.get(".app-button").trigger("click");
    expect(wrapper.emitted("date-change")).toEqual([["2026-10-07"]]);
    wrapper.unmount();
  });
});
