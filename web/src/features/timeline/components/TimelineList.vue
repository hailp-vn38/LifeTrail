<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { usePlaybackStore } from "../../../stores/playback.store";
import TimelineItem from "./TimelineItem.vue";
import type { TimelineEvent } from "../types";

const props = defineProps<{ events: TimelineEvent[]; selectedId: string | null }>();
defineEmits<{ select: [id: string] }>();

const playback = usePlaybackStore();
const itemInstances = ref(new Map<string, { $el?: unknown }>());

function setItemRef(id: string, instance: unknown) {
  if (instance && typeof instance === "object" && "$el" in instance) {
    itemInstances.value.set(id, instance as { $el?: unknown });
  } else {
    itemInstances.value.delete(id);
  }
}

// When the map (or playback) selects an event, bring it into view.
watch(
  () => props.selectedId,
  async (id) => {
    if (!id) return;
    await nextTick();
    const element = itemInstances.value.get(id)?.$el;
    if (element instanceof HTMLElement) {
      element.scrollIntoView({ block: "nearest", behavior: "smooth" });
    }
  },
);
</script>

<template>
  <ul class="timeline-list" aria-label="Danh sách sự kiện">
    <TimelineItem
      v-for="event in events"
      :key="event.id"
      :ref="(instance) => setItemRef(event.id, instance)"
      :event="event"
      :selected="selectedId === event.id"
      :future="playback.durationMs > 0 && event.recordedAtMs !== undefined && event.recordedAtMs > playback.startTimeMs + playback.currentTimeMs"
      @select="$emit('select', $event)"
    />
    <slot />
  </ul>
</template>

<style scoped>
.timeline-list {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  flex: 1;
  min-height: 0;
  margin: 0;
  padding: .2rem .25rem .5rem .5rem;
  scrollbar-gutter: stable;
  scrollbar-width: auto;
  scrollbar-color: #94a3b8 #f1f5f9;
  overflow-y: auto;
  max-height: 100%;
}
@media (max-width: 767px) { .timeline-list { overflow: visible; max-height: none; } }
</style>
