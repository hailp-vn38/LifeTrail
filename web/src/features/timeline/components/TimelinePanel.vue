<script setup lang="ts">
import { computed } from "vue";
import type { DailyView } from "../../../api/queries/daily-view.query";
import { usePlaybackStore } from "../../../stores/playback.store";
import TimelineStats from "./TimelineStats.vue";
import TimelineSummary from "./TimelineSummary.vue";
import { useMapStore } from "../../../stores/map.store";
import AppEmptyState from "../../../components/ui/AppEmptyState.vue";
import { buildTimelineEvents } from "../events";
import TimelineHeader from "./TimelineHeader.vue";
import TimelineList from "./TimelineList.vue";

const props = defineProps<{ dailyView: DailyView }>();

const playback = usePlaybackStore();
function selectEvent(id: string) {
  const event = events.value.find(item => item.id === id);
  if (event?.recordedAtMs !== undefined) playback.requestSeek(event.recordedAtMs);
  mapStore.selectEvent(id);
}
const mapStore = useMapStore();
const events = computed(() => buildTimelineEvents(props.dailyView));
</script>

<template>
  <section class="timeline-panel" aria-label="Timeline">
    <TimelineHeader
      :event-count="events.length"
      :date="dailyView.date"
      :has-selection="mapStore.selectedEventId !== null"
      @clear="mapStore.clearSelection()"
    />
    <TimelineSummary :daily-view="dailyView" />
    <AppEmptyState
      v-if="events.length === 0"
      title="Chưa có sự kiện"
      :description="dailyView.evidence_state === 'insufficient' ? 'Chưa đủ dữ liệu để xác định hoạt động.' : 'Ngày này chưa có điểm bắt đầu/kết thúc nào được ghi nhận.'"
    />
    <details v-if="events.length === 0"><summary class="text-muted text-sm">Thống kê chi tiết</summary><TimelineStats :daily-view="dailyView" /></details>
    <TimelineList
      v-else
      :events="events"
      :selected-id="mapStore.selectedEventId"
      @select="selectEvent($event)"
    ><li><details><summary class="text-muted text-sm">Thống kê chi tiết</summary><TimelineStats :daily-view="dailyView" /></details></li></TimelineList>
  </section>
</template>

<style scoped>
.timeline-panel {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-height: 0;
  height: 100%;
  overflow: hidden;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-card);
  padding: 0.65rem;
}
.timeline-panel > details > summary, .timeline-panel :deep(li > details > summary) { font-size: .6875rem; }
</style>
