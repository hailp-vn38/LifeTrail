import { mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { beforeEach, describe, expect, it } from "vitest";
import { MAP_STYLE_STORAGE_KEY } from "../../../stores/map-preferences.store";
import SettingsPage from "./SettingsPage.vue";

beforeEach(() => localStorage.clear());

describe("SettingsPage", () => {
  it("offers VersaTiles themes and persists a dark palette", async () => {
    const wrapper = mount(SettingsPage, { global: { plugins: [createPinia()] } });
    const group = wrapper.get('optgroup[label="VersaTiles"]');
    expect(group.findAll("option")).toHaveLength(10);
    await wrapper.get("select").setValue("versatiles-colorful-dark");
    expect(localStorage.getItem(MAP_STYLE_STORAGE_KEY)).toBe("versatiles-colorful-dark");
    expect(wrapper.get("#map-style-description").text()).toContain("nền tối");
    wrapper.unmount();
  });
  it("lets the user choose 3D and restores the choice when returning to Settings", async () => {
    const mountSettings = () => mount(SettingsPage, { global: { plugins: [createPinia()] } });
    const wrapper = mountSettings();
    expect(wrapper.findAll("option").map((option) => option.text())).toContain("Đường phố 3D");
    await wrapper.get("select").setValue("streets-3d");
    expect(localStorage.getItem(MAP_STYLE_STORAGE_KEY)).toBe("streets-3d");
    expect(wrapper.get("#map-style-description").text()).toContain("tòa nhà 3D");
    wrapper.unmount();
    const reopened = mountSettings();
    expect((reopened.get("select").element as HTMLSelectElement).value).toBe("streets-3d");
    reopened.unmount();
  });
});
