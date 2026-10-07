<script setup lang="ts">
import { computed, ref, watch } from "vue";
import RouteMap from "../../../components/RouteMap.vue";
import type { DailyView } from "../../../api/queries/daily-view.query";
import { usePlaybackQuery } from "../../../api/queries/playback.query";
import { logMapMetrics } from "../../../map/map-metrics";

const props = defineProps<{ dailyView: DailyView }>();

// Canonical playback geometry is a separate, lazily-loaded resource: nothing is
// fetched until playback is engaged. The manifest version pins the request to
// the snapshot the client is showing.
const requested = ref(false);
const requestedAt = ref(0);
const manifestVersion = computed(() => props.dailyView.provenance?.manifest_version);
const playback = usePlaybackQuery(
  computed(() => props.dailyView.device_id),
  computed(() => props.dailyView.date),
  manifestVersion,
  requested,
);
const playbackParts = computed(() => playback.data.value?.route_parts);

function requestPlayback() {
  if (!requested.value) requestedAt.value = performance.now();
  requested.value = true;
}

watch(playbackParts, (parts) => {
  if (!parts?.length) return;
  logMapMetrics("playback_loaded", {
    route_parts: parts.length,
    playback_bytes: JSON.stringify(parts).length,
    latency_ms: Math.round(performance.now() - requestedAt.value),
  });
});
</script>

<template>
  <div class="daily-map-canvas">
    <RouteMap
      :key="`${dailyView.device_id}:${dailyView.date}:${dailyView.timezone}:${dailyView.processing_state}:${dailyView.processing?.published_revision ?? 'raw'}`"
      :daily-view="dailyView"
      :playback-parts="playbackParts"
      day-clock
      @request-playback="requestPlayback"
    />
  </div>
</template>

<style scoped>
.daily-map-canvas {
  min-width: 0;
  min-height: 0;
  height: 100%;
  flex: 1;
}
</style>
