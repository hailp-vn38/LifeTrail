import { mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { ref, toValue } from "vue";
import type { MaybeRefOrGetter } from "vue";
import { tripView, tripPlayback } from "../../../test/fixtures/trips";

// Capture the `enabled` gate so we can prove nothing is fetched until engaged.
const gates: Array<MaybeRefOrGetter<boolean>> = [];
const data = ref<{ route_parts: ReturnType<typeof tripPlayback> } | undefined>(undefined);
vi.mock("../../../api/queries/playback.query", () => ({
  usePlaybackQuery: (
    _device: unknown,
    _date: unknown,
    _manifest: unknown,
    enabled: MaybeRefOrGetter<boolean>,
  ) => {
    gates.push(enabled);
    return { data };
  },
}));
vi.mock("../../../components/RouteMap.vue", () => ({
  default: {
    name: "RouteMap",
    props: ["dailyView", "playbackParts"],
    emits: ["request-playback"],
    template: '<button data-test="play" @click="$emit(\'request-playback\')" />',
  },
}));

import DailyMapCanvas from "./DailyMapCanvas.vue";

describe("DailyMapCanvas", () => {
  beforeEach(() => {
    gates.length = 0;
    data.value = undefined;
  });

  it("keeps playback disabled until the user engages it", async () => {
    const wrapper = mount(DailyMapCanvas, {
      props: { dailyView: tripView() },
      global: { plugins: [createPinia()] },
    });
    const gate = gates.at(-1)!;
    expect(toValue(gate)).toBe(false);

    await wrapper.get('[data-test="play"]').trigger("click");
    expect(toValue(gate)).toBe(true);
  });

  it("passes canonical playback parts down once they load", async () => {
    const wrapper = mount(DailyMapCanvas, {
      props: { dailyView: tripView() },
      global: { plugins: [createPinia()] },
    });
    data.value = { route_parts: tripPlayback() };
    await wrapper.vm.$nextTick();
    expect(wrapper.findComponent({ name: "RouteMap" }).props("playbackParts")).toEqual(
      tripPlayback(),
    );
  });
});
