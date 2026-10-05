<script setup lang="ts">
/**
 * Timeline statistics card.
 *
 * Thin wrapper over the Daily Map statistics so the right column keeps the
 * spec's `TimelineStats` name while the markup lives in one place.
 */
import type { DailyView } from "../../../api/queries/daily-view.query";
import DailyMapStats from "../../daily-map/components/DailyMapStats.vue";

defineProps<{ dailyView: DailyView }>();
</script>

<template>
  <DailyMapStats :daily-view="dailyView" />
</template>
