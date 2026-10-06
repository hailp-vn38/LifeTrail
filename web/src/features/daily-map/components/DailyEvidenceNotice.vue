<script setup lang="ts">
import { computed } from "vue";
import type { DailyView } from "../../../api/queries/daily-view.query";
import { describeEvidenceHoles } from "../../timeline/evidence";

const props = defineProps<{ dailyView: DailyView }>();
// Only Evidence Holes are listed here. A GPS Gap is absence of observations and
// is explained as its own Timeline item, so the two never share a message.
const holes = computed(() => describeEvidenceHoles(props.dailyView));
</script>

<template>
  <ul v-if="holes.length" class="evidence-notice" aria-label="Khoảng thiếu bằng chứng">
    <li v-for="(hole, index) in holes" :key="index">
      {{ hole }}
    </li>
  </ul>
</template>

<style scoped>
.evidence-notice { margin-top: 0.35rem; color: var(--color-text-muted); padding-left: 1.25rem; }
</style>