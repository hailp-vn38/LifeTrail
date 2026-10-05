import { mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { describe, expect, it } from "vitest";
import { createMemoryHistory } from "vue-router";
import { createLifeTrailRouter } from "../../app/router";
import SidebarNav from "./SidebarNav.vue";

async function mountNav(path: string) {
  const router = createLifeTrailRouter(createMemoryHistory());
  await router.push(path);
  await router.isReady();
  return mount(SidebarNav, {
    global: { plugins: [router, createPinia()] },
  });
}

describe("SidebarNav", () => {
  it("highlights the active section", async () => {
    const wrapper = await mountNav("/devices");

    const active = wrapper.find('[aria-current="page"]');
    expect(active.exists()).toBe(true);
    expect(active.text()).toContain("Devices");
    // Exactly one item is active.
    expect(wrapper.findAll('[aria-current="page"]')).toHaveLength(1);

    wrapper.unmount();
  });

  it("highlights Daily Map for the canonical daily route", async () => {
    const wrapper = await mountNav(
      "/devices/8f467f83-f8e0-4d91-8f73-26e77e8fb0b5/day/2026-10-04",
    );

    const active = wrapper.find('[aria-current="page"]');
    expect(active.text()).toContain("Daily Map");

    wrapper.unmount();
  });

  it("never hides entries for unfinished features", async () => {
    const wrapper = await mountNav("/");

    const labels = wrapper
      .findAll(".sidebar-nav-item__label")
      .map((node) => node.text());
    expect(labels).toEqual(
      expect.arrayContaining(["Timeline", "Reports", "Settings"]),
    );

    wrapper.unmount();
  });
});
