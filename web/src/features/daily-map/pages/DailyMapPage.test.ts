import { mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { ref } from "vue";
import { createMemoryHistory, createRouter } from "vue-router";
import { ApiRequestError } from "../../../api/errors/api-error";
import { useDailyView } from "../../../api/queries/daily-view.query";
import type { DailyView } from "../../../api/queries/daily-view.query";
import DailyMapPage from "./DailyMapPage.vue";

vi.mock("../../../api/queries/daily-view.query", () => ({
  useDailyView: vi.fn(),
}));

vi.mock("../../../api/queries/devices.query", () => ({
  useDevices: () => ({
    isPending: ref(false),
    isError: ref(false),
    data: ref([]),
    refetch: vi.fn(),
  }),
}));

const mockedUseDailyView = vi.mocked(useDailyView);

function dailyViewFixture(pointCount: number): DailyView {
  const coordinates =
    pointCount > 0
      ? [
          [106.7, 10.776],
          [106.7005, 10.7764],
        ]
      : [];
  return {
    device_id: "device-1",
    date: "2026-10-05",
    timezone: "Asia/Ho_Chi_Minh",
    processing_state: "raw",
    summary: {
      point_count: pointCount,
      distance_m: 120,
      duration_s: 30,
      first_fix_at: "2026-10-05T10:00:00.000Z",
      last_fix_at: "2026-10-05T10:00:30.000Z",
    },
    route:
      pointCount > 0
        ? {
            type: "Feature",
            properties: {},
            geometry: { type: "LineString", coordinates },
          }
        : null,
    start:
      pointCount > 0
        ? {
            type: "Feature",
            properties: {},
            geometry: { type: "Point", coordinates: coordinates[0] },
          }
        : null,
    end:
      pointCount > 0
        ? {
            type: "Feature",
            properties: {},
            geometry: { type: "Point", coordinates: coordinates[1] },
          }
        : null,
  } as DailyView;
}

function queryState(partial: Record<string, unknown>) {
  return {
    isPending: ref(false),
    isError: ref(false),
    data: ref<DailyView | null>(null),
    error: ref<unknown>(null),
    refetch: vi.fn(),
    ...partial,
  };
}

async function mountPage() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: "/devices/:deviceId/day/:date", component: DailyMapPage }],
  });
  await router.push("/devices/device-1/day/2026-10-05");
  await router.isReady();
  return mount(DailyMapPage, {
    global: {
      plugins: [router, createPinia()],
      stubs: {
        // MapLibre cannot run in jsdom; the canvas integration is covered
        // by RouteMap.test.ts with a mocked map.
        RouteMap: { template: '<div class="route-map-stub" />' },
      },
    },
  });
}

describe("DailyMapPage", () => {
  it("shows insufficient processed evidence without invented timeline activity", async () => {
    const view = { ...dailyViewFixture(1), processing_state: "processed",
      evidence_state: "insufficient", route: null, start: null, end: null, timeline: [], route_parts: [],
      processing: { state: "idle", data_freshness: "current", published_revision: "snapshot-1" },
    };
    mockedUseDailyView.mockReturnValue(queryState({ data: ref(view) }) as never);
    const wrapper = await mountPage();
    expect(wrapper.text()).toContain("Chưa đủ dữ liệu để xác định hoạt động");
    expect(wrapper.text()).toContain("Đã xử lý");
    expect(wrapper.text()).not.toContain("Không có dữ liệu GPS cho ngày này");
    expect(wrapper.findAll(".timeline-item")).toHaveLength(0);
    wrapper.unmount();
  });

  it("shows queued processing separately from Raw GPS", async () => {
    const view = { ...dailyViewFixture(1),
      processing: { state: "queued", data_freshness: "unavailable", published_revision: null },
    };
    mockedUseDailyView.mockReturnValue(queryState({ data: ref(view) }) as never);
    const wrapper = await mountPage();
    expect(wrapper.text()).toContain("Đang chờ xử lý");
    expect(wrapper.text()).toContain("Raw GPS");
    wrapper.unmount();
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the loading state", async () => {
    mockedUseDailyView.mockReturnValue(queryState({ isPending: ref(true) }) as never);
    const wrapper = await mountPage();

    expect(wrapper.text()).toContain("Đang tải Daily View");

    wrapper.unmount();
  });

  it("renders the empty state for zero-point days", async () => {
    mockedUseDailyView.mockReturnValue(
      queryState({ data: ref(dailyViewFixture(0)) }) as never,
    );
    const wrapper = await mountPage();

    expect(wrapper.text()).toContain("Không có dữ liệu GPS cho ngày này");
    // Timeline still renders its own empty state inside the layout.
    expect(wrapper.text()).toContain("Chưa có sự kiện");
    // Stats render zero values instead of crashing.
    expect(wrapper.text()).toContain("GPS records");

    wrapper.unmount();
  });

  it("renders the device 404 state inside the page", async () => {
    mockedUseDailyView.mockReturnValue(
      queryState({
        isError: ref(true),
        error: ref(new ApiRequestError(404, "not found")),
      }) as never,
    );
    const wrapper = await mountPage();

    expect(wrapper.text()).toContain("Không tìm thấy Device");

    wrapper.unmount();
  });

  it("renders the server error state with retry", async () => {
    const refetch = vi.fn();
    mockedUseDailyView.mockReturnValue(
      queryState({
        isError: ref(true),
        error: ref(new ApiRequestError(500, "boom")),
        refetch,
      }) as never,
    );
    const wrapper = await mountPage();

    expect(wrapper.text()).toContain("Không thể tải Daily View");
    await wrapper.find(".app-error-state .app-button").trigger("click");
    expect(refetch).toHaveBeenCalled();

    wrapper.unmount();
  });

  it("renders map, timeline and stats for a loaded day", async () => {
    mockedUseDailyView.mockReturnValue(
      queryState({ data: ref(dailyViewFixture(4)) }) as never,
    );
    const wrapper = await mountPage();

    expect(wrapper.find(".route-map-stub").exists()).toBe(true);
    expect(wrapper.find(".timeline-panel").exists()).toBe(true);
    expect(wrapper.text()).toContain("Start");
    expect(wrapper.text()).toContain("End");
    expect(wrapper.text()).toContain("Statistics");

    wrapper.unmount();
  });
});
