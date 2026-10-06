import { mount, flushPromises } from "@vue/test-utils";
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
  useDailyStatus: () => ({ data: ref(undefined), refetch: vi.fn() }),
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

import { stationaryView, openStop } from "../../../test/fixtures/stationary";
import { tripView, openTrip } from "../../../test/fixtures/trips";
import { useMapStore } from "../../../stores/map.store";

it("shows a stationary Stop map, observed daily totals and synchronized Timeline selection", async () => {
  mockedUseDailyView.mockReturnValue(queryState({ data: ref(stationaryView()) }) as never);
  const wrapper = await mountPage();
  expect(wrapper.find(".route-map-stub").exists()).toBe(true);
  expect(wrapper.text()).toContain("Dừng quan sát trong ngày");
  expect(wrapper.text()).toContain("10 phút");
  expect(wrapper.text()).toContain("Đến: chưa xác định");
  Object.defineProperty(HTMLElement.prototype, "scrollIntoView", { configurable: true, value: vi.fn() });
  const stop = wrapper.find(".timeline-item--stop");
  await stop.trigger("click");
  expect(useMapStore().selectedEventId).toBe(openStop.id);
  expect(stop.attributes("aria-pressed")).toBe("true");
  useMapStore().clearSelection();
  await wrapper.vm.$nextTick();
  useMapStore().selectEvent(openStop.id);
  await wrapper.vm.$nextTick();
  expect(stop.attributes("aria-pressed")).toBe("true");
  await flushPromises();
  expect(HTMLElement.prototype.scrollIntoView).toHaveBeenCalled();
  wrapper.unmount();
  Reflect.deleteProperty(HTMLElement.prototype, "scrollIntoView");
});

it("distinguishes unreliable observations from absent GPS beside a published Stop", async () => {
  const view = stationaryView();
  view.evidence_state = "partial";
  view.evidence_holes = [{ observed_from_at: "2026-10-05T16:40:00Z", observed_until_at: "2026-10-05T16:45:00Z", reason: "insufficient_quality", source_record_count: 3 }];
  view.summary.low_quality_point_count = 2;
  view.summary.excluded_point_count = 1;
  // A GPS Gap is absent observations: its own Timeline item, not hole coverage.
  view.timeline = [
    { ...view.timeline![0] },
    {
      id: "rev:gap:0",
      kind: "gap",
      activity_revision: "rev",
      observed_from_at: "2026-10-05T16:45:00Z",
      observed_until_at: "2026-10-05T16:50:00Z",
      observed_duration_s: 300,
      visible_from_at: "2026-10-05T16:45:00Z",
      visible_until_at: "2026-10-05T16:50:00Z",
      daily_observed_duration_s: 300,
      continues_before: false,
      continues_after: false,
    },
  ];
  mockedUseDailyView.mockReturnValue(queryState({ data: ref(view) }) as never);
  const wrapper = await mountPage();
  // The unreliable interval keeps its own explanation and record count.
  expect(wrapper.text()).toContain("Có GPS nhưng chất lượng chưa đủ");
  expect(wrapper.text()).toContain("3 bản ghi GPS");
  // The absent interval is a distinct Timeline item explaining the absence.
  expect(wrapper.findAll(".timeline-item--gap")).toHaveLength(1);
  expect(wrapper.text()).toContain("Thiếu quan sát GPS");
  expect(wrapper.text()).toContain("Không suy ra di chuyển");
  // An Evidence Hole notice never claims observations are missing.
  const notice = wrapper.find(".evidence-notice");
  expect(notice.exists()).toBe(true);
  expect(notice.text()).not.toContain("Thiếu quan sát GPS");
  // The two poorer quality classes are named separately, so a poor record is
  // never presented as an impossible position.
  expect(notice.text()).toContain("2 bản ghi GPS chất lượng thấp");
  expect(notice.text()).toContain("1 bản ghi GPS có vị trí không hợp lý");
  expect(wrapper.findAll(".timeline-item--stop")).toHaveLength(1);
  wrapper.unmount();
});

it("shows a Trip with its daily totals and synchronizes Timeline selection", async () => {
  mockedUseDailyView.mockReturnValue(queryState({ data: ref(tripView()) }) as never);
  const wrapper = await mountPage();
  expect(wrapper.text()).toContain("Trips trong ngày");
  expect(wrapper.text()).toContain("Thời gian trong Trip");
  expect(wrapper.text()).toContain("Gồm cả dừng ngắn");
  expect(wrapper.text()).toContain("bao gồm dừng ngắn");
  expect(wrapper.text()).toContain("chưa xác định");
  // Chronological projection: the Stop precedes the Trip.
  const kinds = wrapper.findAll(".timeline-item").map((item) => item.classes().join(" "));
  expect(kinds.some((className) => className.includes("timeline-item--stop"))).toBe(true);
  expect(kinds.some((className) => className.includes("timeline-item--trip"))).toBe(true);
  expect(wrapper.findAll(".timeline-item")[0].classes().join(" ")).toContain("timeline-item--stop");

  Object.defineProperty(HTMLElement.prototype, "scrollIntoView", { configurable: true, value: vi.fn() });
  const trip = wrapper.find(".timeline-item--trip");
  await trip.trigger("click");
  expect(useMapStore().selectedEventId).toBe(openTrip.id);
  expect(trip.attributes("aria-pressed")).toBe("true");
  useMapStore().clearSelection();
  await wrapper.vm.$nextTick();
  useMapStore().selectEvent(openTrip.id);
  await wrapper.vm.$nextTick();
  expect(trip.attributes("aria-pressed")).toBe("true");
  wrapper.unmount();
  Reflect.deleteProperty(HTMLElement.prototype, "scrollIntoView");
});

it("keeps the complete-day Raw Route selectable after publishing a Trip", async () => {
  mockedUseDailyView.mockReturnValue(queryState({ data: ref(tripView()) }) as never);
  const wrapper = await mountPage();
  expect(wrapper.text()).not.toContain("Start");
  await wrapper.get('button[aria-label="Xem Raw GPS"]').trigger("click");
  expect(mockedUseDailyView.mock.calls.at(-1)?.[2]).toMatchObject({ value: true });
  wrapper.unmount();
});

it("lets the Owner request Raw GPS after viewing a processed Stop", async () => {
  mockedUseDailyView.mockReturnValue(queryState({ data: ref(stationaryView()) }) as never);
  const wrapper = await mountPage();
  await wrapper.get('button[aria-label="Xem Raw GPS"]').trigger("click");
  const raw = mockedUseDailyView.mock.calls.at(-1)?.[2];
  expect(typeof raw === "object" && raw?.value).toBe(true);
  await wrapper.get('button[aria-label="Xem hoạt động"]').trigger("click");
  expect(typeof raw === "object" && raw?.value).toBe(false);
  wrapper.unmount();
});
