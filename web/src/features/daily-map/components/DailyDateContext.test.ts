import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import DailyDateContext from "./DailyDateContext.vue";

describe("DailyDateContext", () => {
  it("opens the date picker from the full date and closes after selection", async () => {
    const wrapper = mount(DailyDateContext, { props: { date: "2026-10-05" } });
    expect(wrapper.get("summary").text()).toBe("Thứ 2, 05 tháng 10, 2026");
    const picker = wrapper.get("details");
    expect(picker.element.open).toBe(false);
    picker.element.open = true;
    await wrapper.get('input[type="date"]').setValue("2026-11-12");
    expect(wrapper.emitted("date-change")).toEqual([["2026-11-12"]]);
    expect(picker.element.open).toBe(false);
    wrapper.unmount();
  });

  it("dismisses the picker using Escape without changing the selected day", async () => {
    const wrapper = mount(DailyDateContext, { props: { date: "2026-10-11" } });
    expect(wrapper.get("summary").text()).toContain("Chủ nhật");
    const picker = wrapper.get("details");
    picker.element.open = true;
    await picker.trigger("keydown", { key: "Escape" });
    expect(picker.element.open).toBe(false);
    expect(wrapper.emitted("date-change")).toBeUndefined();
    wrapper.unmount();
  });
});
