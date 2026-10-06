<script setup lang="ts">
import { CircleDot, Flag, Image, Mic, Pause, Route, Unplug } from "lucide-vue-next";
import { computed } from "vue";
import type { TimelineEvent, TimelineEventKind } from "../types";

const props = defineProps<{ event: TimelineEvent; selected: boolean; future?: boolean }>();
defineEmits<{ select: [id: string] }>();

const KIND_ICON: Record<TimelineEventKind, typeof Flag> = {
  start: Flag,
  end: Flag,
  trip: Route,
  stop: Pause,
  gap: Unplug,
  photo: Image,
  audio: Mic,
};
const KIND_FALLBACK = CircleDot;

const icon = computed(() => KIND_ICON[props.event.kind] ?? KIND_FALLBACK);
const selectedLabel = computed(() =>
  props.selected ? " (đang chọn)" : "",
);
</script>

<template>
  <li class="timeline-item__wrapper">
    <button
      type="button"
      class="timeline-item"
      :class="[`timeline-item--${event.kind}`, { 'is-selected': selected, 'is-future': future }]"
      :aria-pressed="selected"
      :aria-label="`${event.title}${selectedLabel}`"
      @click="$emit('select', event.id)"
    >
      <span class="timeline-item__marker" aria-hidden="true">
        <component :is="icon" :size="16" />
      </span>
      <span class="timeline-item__text">
        <span class="timeline-item__title">{{ event.travelLabel ?? event.title }}</span>
        <span v-if="event.timeLabel" class="timeline-item__time tabular">{{ event.timeLabel }}</span>
        <span v-if="event.subtitle && !event.timeLabel" class="timeline-item__subtitle text-muted">
          {{ event.subtitle }}
        </span>
      </span>
      <span v-if="event.durationLabel" class="timeline-item__duration">{{ event.durationLabel }}</span>
    </button>
    <details v-if="event.subtitle && event.timeLabel" class="timeline-item__details"><summary>Chi tiết quan sát</summary>{{ event.subtitle }}</details>
  </li>
</template>

<style scoped>
.timeline-item__wrapper {
  list-style: none;
}
.timeline-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  width: 100%;
  text-align: left;
  padding: 0.65rem 0.75rem;
  border: 1px solid #e2e8f0;
  border-radius: var(--radius-sm);
  background: white;
  transition: background 0.15s ease, border-color 0.15s ease;
}
.timeline-item.is-future { opacity: .4; }
.timeline-item__time { color: var(--color-text-muted); font-size: .78rem; }
.timeline-item__duration { margin-left: auto; flex: none; border-radius: 999px; background: #dbeafe; color: #1d4ed8; padding: .3rem .5rem; font-size: .72rem; }
.timeline-item--stop .timeline-item__marker { background: #dbeafe; color: #2563eb; }
.timeline-item--trip { border: 0; border-left: 2px dashed #cbd5e1; border-radius: 0; margin-left: 1rem; width: calc(100% - 1rem); color: var(--color-text-muted); background: transparent; }
.timeline-item__details { font-size: .7rem; color: var(--color-text-muted); padding: .3rem .5rem .3rem 3rem; }
.timeline-item:hover {
  background: #f2f6fc;
}
.timeline-item.is-selected {
  background: var(--color-primary-soft);
  border-color: var(--color-primary);
}
.timeline-item__marker {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2rem;
  height: 2rem;
  border-radius: 999px;
  flex: none;
  background: #eef1f6;
  color: var(--color-text-muted);
}
.timeline-item--start .timeline-item__marker {
  background: #e6f9f0;
  color: #0e9f6e;
}
.timeline-item--end .timeline-item__marker {
  background: #fdecec;
  color: var(--color-danger);
}
/* A Gap is an absence, so it reads as a break rather than activity. */
.timeline-item--gap .timeline-item__marker {
  background: #f3f4f6;
  color: #6b7280;
}
.timeline-item--gap .timeline-item__title {
  font-weight: 600;
}
.timeline-item.is-selected .timeline-item__marker {
  background: var(--color-primary);
  color: #fff;
}
.timeline-item__text {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  min-width: 0;
}
.timeline-item__title {
  font-weight: 650;
  font-size: var(--font-size-sm);
}
.timeline-item__subtitle {
  font-size: var(--font-size-xs);
}
</style>
