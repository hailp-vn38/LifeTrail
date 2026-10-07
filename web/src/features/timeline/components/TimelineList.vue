<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { usePlaybackStore } from "../../../stores/playback.store";
import TimelineItem from "./TimelineItem.vue";
import type { TimelineEvent } from "../types";

const props = defineProps<{ events: TimelineEvent[]; selectedId: string | null }>();
defineEmits<{ select: [id: string] }>();

const playback = usePlaybackStore();
const currentId = computed(() => {
  if (!playback.durationMs || playback.status === "idle") return null;
  return props.events.find((event, index) => event.recordedAtMs !== undefined
    && event.recordedAtMs <= playback.currentEpochMs
    && playback.currentEpochMs < (event.recordedUntilMs ?? props.events[index + 1]?.recordedAtMs ?? Infinity))?.id ?? null;
});
const itemInstances = ref(new Map<string, { $el?: unknown }>());

function setItemRef(id: string, instance: unknown) {
  if (instance && typeof instance === "object" && "$el" in instance) {
    itemInstances.value.set(id, instance as { $el?: unknown });
  } else {
    itemInstances.value.delete(id);
  }
}

// Scroll only when the selection or the current activity changes.
async function scrollToEvent(id: string | null) {
  if (!id) return;
  await nextTick();
  const element = itemInstances.value.get(id)?.$el;
  if (element instanceof HTMLElement) {
    element.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }
}
watch(() => props.selectedId, scrollToEvent);
watch(currentId, scrollToEvent);
</script>

<template>
  <ul class="timeline-list" aria-label="Danh sách sự kiện">
    <TimelineItem
      v-for="event in events"
      :key="event.id"
      :ref="(instance) => setItemRef(event.id, instance)"
      :event="event"
      :selected="selectedId === event.id"
      :current="currentId === event.id"
      :future="playback.durationMs > 0 && event.recordedAtMs !== undefined && event.recordedAtMs > playback.currentEpochMs"
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
