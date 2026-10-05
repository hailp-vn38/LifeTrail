<script setup lang="ts">
import RouteMap from "../../../components/RouteMap.vue";
import type { DailyView } from "../../../api/queries/daily-view.query";

defineProps<{ dailyView: DailyView }>();
</script>

<template>
  <div class="daily-map-canvas">
    <RouteMap
      :key="`${dailyView.device_id}:${dailyView.date}`"
      :daily-view="dailyView"
    />
  </div>
</template>

<style scoped>
.daily-map-canvas {
  min-width: 0;
}
</style>
