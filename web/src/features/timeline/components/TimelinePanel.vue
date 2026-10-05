<script setup lang="ts">
import { computed } from "vue";
import type { DailyView } from "../../../api/queries/daily-view.query";
import { useMapStore } from "../../../stores/map.store";
import AppEmptyState from "../../../components/ui/AppEmptyState.vue";
import { buildTimelineEvents } from "../events";
import TimelineHeader from "./TimelineHeader.vue";
import TimelineList from "./TimelineList.vue";

const props = defineProps<{ dailyView: DailyView }>();

const mapStore = useMapStore();
const events = computed(() => buildTimelineEvents(props.dailyView));
</script>

<template>
  <section class="timeline-panel" aria-label="Timeline">
    <TimelineHeader
      :event-count="events.length"
      :has-selection="mapStore.selectedEventId !== null"
      @clear="mapStore.clearSelection()"
    />
    <AppEmptyState
      v-if="events.length === 0"
      title="Chưa có sự kiện"
      description="Ngày này chưa có điểm bắt đầu/kết thúc nào được ghi nhận."
    />
    <TimelineList
      v-else
      :events="events"
      :selected-id="mapStore.selectedEventId"
      @select="mapStore.selectEvent($event)"
    />
  </section>
</template>

<style scoped>
.timeline-panel {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  min-height: 0;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-card);
  padding: 1rem;
}
</style>
