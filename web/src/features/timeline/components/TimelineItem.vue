<script setup lang="ts">
import { Bike, Car, Footprints, CircleDot, Flag, Info, Image, Mic, Pause, Route, Unplug } from "lucide-vue-next";
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

const icon = computed(() => props.event.kind === "trip" && props.event.transportMode
  ? ({ walk: Footprints, bike: Bike, car: Car, unknown: Route })[props.event.transportMode]
  : KIND_ICON[props.event.kind] ?? KIND_FALLBACK);
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
        <component :is="icon" :size="14" />
      </span>
      <span class="timeline-item__text">
        <span class="timeline-item__title" :title="event.travelLabel ?? event.title">{{ event.travelLabel ?? event.title }}</span>
        <span v-if="event.timeLabel" class="timeline-item__time tabular">{{ event.timeLabel }}</span>
        <span v-if="event.subtitle && !event.timeLabel" class="timeline-item__subtitle text-muted">
          {{ event.subtitle }}
        </span>
      </span>
      <span v-if="event.durationLabel" class="timeline-item__duration">{{ event.durationLabel }}</span>
    </button>
    <details v-if="event.subtitle && event.timeLabel" class="timeline-item__details">
      <summary aria-label="Chi tiết quan sát" title="Chi tiết quan sát"><Info :size="12" /></summary>
      <span>{{ event.subtitle }}</span>
    </details>
  </li>
</template>

<style scoped src="./timeline-item.css"></style>
