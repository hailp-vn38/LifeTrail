<script setup lang="ts">
import { computed } from "vue";
import type { DailyView } from "../../../api/queries/daily-view.query";
import { describeEvidenceHoles, describeWithheldCoverage } from "../../timeline/evidence";

const props = defineProps<{ dailyView: DailyView }>();
// Only Evidence Holes are listed here. A GPS Gap is absence of observations and
// is explained as its own Timeline item, so the two never share a message.
const holes = computed(() => describeEvidenceHoles(props.dailyView));
// The two poorer quality classes behind those intervals, named so the Owner can
// tell a poor record from an impossible one.
const withheld = computed(() => describeWithheldCoverage(props.dailyView.summary));
</script>

<template>
  <ul
    v-if="withheld || holes.length"
    class="evidence-notice"
    aria-label="Khoảng thiếu bằng chứng"
  >
    <li v-if="withheld">{{ withheld }}</li>
    <li v-for="(hole, index) in holes" :key="index">
      {{ hole }}
    </li>
  </ul>
</template>

<style scoped>
.evidence-notice { margin-top: 0.35rem; color: var(--color-text-muted); padding-left: 1.25rem; }
</style>
