<script setup lang="ts">
import { computed } from "vue";
import type { DailyView } from "../../../api/queries/daily-view.query";
import { formatTimestamp } from "../../../lib/format";

const props = defineProps<{ dailyView: DailyView }>();
const intervals = computed(() => [...(props.dailyView.evidence_holes ?? []), ...(props.dailyView.unresolved_intervals ?? [])].sort((a, b) => a.observed_from_at.localeCompare(b.observed_from_at)));
const labels = {
  missing_observations: "Thiếu quan sát GPS",
  unusable_observations: "Có GPS nhưng chất lượng chưa đủ",
  unresolved_activity: "Chưa xác định hoạt động từ quan sát GPS",
};
</script>

<template>
  <ul v-if="intervals.length" class="evidence-notice" aria-label="Khoảng chưa xác định">
    <li v-for="(hole, index) in intervals" :key="index">
      {{ labels[hole.reason] }}:
      {{ formatTimestamp(hole.observed_from_at, dailyView.timezone) }} –
      {{ formatTimestamp(hole.observed_until_at, dailyView.timezone) }}
    </li>
  </ul>
</template>

<style scoped>
.evidence-notice { margin-top: 0.35rem; color: var(--color-text-muted); padding-left: 1.25rem; }
</style>
