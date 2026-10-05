<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import TimelineItem from "./TimelineItem.vue";
import type { TimelineEvent } from "../types";

const props = defineProps<{ events: TimelineEvent[]; selectedId: string | null }>();
defineEmits<{ select: [id: string] }>();

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
      @select="$emit('select', $event)"
    />
  </ul>
</template>

<style scoped>
.timeline-list {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  overflow-y: auto;
  max-height: 100%;
}
</style>
