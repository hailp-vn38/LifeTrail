import { mount, flushPromises } from "@vue/test-utils";
import { defineComponent, ref } from "vue";
import { beforeEach, afterEach, expect, it, vi } from "vitest";
import { useDailyMap } from "./useDailyMap";
import { useDailyStatus, useDailyView } from "../../../api/queries/daily-view.query";

vi.mock("../../../api/queries/daily-view.query", () => ({
  useDailyStatus: vi.fn(), useDailyView: vi.fn(),
}));

let cleanup: (() => void) | undefined;
beforeEach(() => vi.clearAllMocks());
afterEach(() => { cleanup?.(); vi.restoreAllMocks(); });
function harness(revision: string | null) {
  const status = { data: ref({ published_revision: revision, timezone: "UTC", timezone_generation: 0 }), refetch: vi.fn() };
  const query = { data: ref({ processing: { published_revision: revision } }), error: ref(null), refetch: vi.fn() };
  vi.mocked(useDailyStatus).mockReturnValue(status as unknown as ReturnType<typeof useDailyStatus>);
  vi.mocked(useDailyView).mockReturnValue(query as unknown as ReturnType<typeof useDailyView>);
  const wrapper = mount(defineComponent({ setup() { useDailyMap(ref("device"), ref("2026-10-05")); return () => null; } }));
  cleanup = () => wrapper.unmount();
  return { status, query };
}

it("replaces an initial Raw view when the first snapshot is published", async () => {
  const { status, query } = harness(null);
  status.data.value.published_revision = "snapshot-1";
  await flushPromises();
  expect(query.refetch).toHaveBeenCalledOnce();
});

it("refetches the whole day only when the publication changes", async () => {
  const { status, query } = harness("snapshot-1");
  status.data.value = { ...status.data.value };
  await flushPromises();
  expect(query.refetch).not.toHaveBeenCalled();
  status.data.value.published_revision = "snapshot-2";
  await flushPromises();
  expect(query.refetch).toHaveBeenCalledOnce();
});

it("refreshes status when visibility returns and skips hidden events", () => {
  const { status } = harness("snapshot-1");
  const visibility = vi.spyOn(document, "visibilityState", "get");
  visibility.mockReturnValue("hidden");
  document.dispatchEvent(new Event("visibilitychange"));
  window.dispatchEvent(new Event("focus"));
  expect(status.refetch).not.toHaveBeenCalled();
  visibility.mockReturnValue("visible");
  document.dispatchEvent(new Event("visibilitychange"));
  expect(status.refetch).toHaveBeenCalledOnce();
});
