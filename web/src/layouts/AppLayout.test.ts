import { VueQueryPlugin, QueryClient } from "@tanstack/vue-query";
import { mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { createMemoryHistory } from "vue-router";
import { createLifeTrailRouter } from "../app/router";

vi.mock("../api/queries/health.query", () => ({
  useSystemStatus: () => ({
    isPending: { value: false },
    data: { value: { ok: true, deviceCount: 2, checkedAt: 0 } },
  }),
}));

describe("AppLayout", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // jsdom has no matchMedia; simulate a desktop viewport.
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })) as unknown as typeof window.matchMedia;
  });

  async function mountLayout(path: string) {
    const router = createLifeTrailRouter(createMemoryHistory());
    await router.push(path);
    await router.isReady();
    // Mount through a plain RouterView like the real App.vue does, so the
    // router itself renders the single AppLayout instance.
    const wrapper = mount(
      { template: "<RouterView />" },
      {
        global: {
          plugins: [
            router,
            createPinia(),
            [VueQueryPlugin, { queryClient: new QueryClient() }],
          ],
        },
      },
    );
    await router.isReady();
    return wrapper;
  }

  it("renders the persistent sidebar with all main navigation entries", async () => {
    const wrapper = await mountLayout("/");

    const sidebar = wrapper.find(".app-sidebar");
    expect(sidebar.exists()).toBe(true);
    const labels = wrapper
      .findAll(".sidebar-nav-item__label")
      .map((node) => node.text());
    expect(labels).toEqual([
      "Overview",
      "Daily Map",
      "Timeline",
      "Devices",
      "Reports",
      "Settings",
    ]);

    wrapper.unmount();
  });

  it("shows the page title in the topbar", async () => {
    const wrapper = await mountLayout("/devices");

    expect(wrapper.find(".app-topbar__title").text()).toBe("Devices");

    wrapper.unmount();
  });

  it("keeps the sidebar mounted when navigating between pages", async () => {
    const wrapper = await mountLayout("/");

    const sidebarBefore = wrapper.find(".app-sidebar").element;
    await wrapper.find('a[href="/devices"]').trigger("click");
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(wrapper.find(".app-sidebar").element).toBe(sidebarBefore);
    expect(wrapper.find(".app-topbar__title").text()).toBe("Devices");

    wrapper.unmount();
  });
});
