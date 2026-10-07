<script setup lang="ts">
import AppShimmerBlock from "../../../components/ui/AppShimmerBlock.vue";
import DailyTimelineLoading from "./DailyTimelineLoading.vue";
</script>

<template>
  <div class="daily-map-loading" aria-busy="true" role="status" aria-live="polite" aria-label="Đang tải Daily Map">
    <span class="daily-map-loading__label">Đang tải Daily View…</span>
    <div class="daily-map-workspace" aria-hidden="true">
      <div class="map-column daily-map-loading__map">
        <AppShimmerBlock class="daily-map-loading__surface" />
        <div class="daily-map-loading__map-tools"><AppShimmerBlock /><AppShimmerBlock /></div>
        <div class="daily-map-loading__playback">
          <AppShimmerBlock class="playback-button" />
          <AppShimmerBlock class="playback-track" />
          <AppShimmerBlock class="playback-clock" />
        </div>
      </div>
      <div class="timeline-column"><DailyTimelineLoading /></div>
    </div>
  </div>
</template>

<style scoped>
.daily-map-loading { display: flex; flex-direction: column; flex: 1; min-height: 0; gap: 8px; }
.daily-map-loading__label { position: absolute; width: 1px; height: 1px; padding: 0; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
.daily-map-loading__map { position: relative; overflow: hidden; border: 1px solid var(--color-border); border-radius: 12px; }
.daily-map-loading__surface { flex: 1; height: 100%; border-radius: 0; background: #eaf0f6; }
.daily-map-loading__map-tools { position: absolute; top: 16px; left: 16px; display: flex; gap: 8px; }
.daily-map-loading__map-tools > span { width: 36px; height: 36px; background: #dce5ef; }
.daily-map-loading__playback { position: absolute; bottom: 16px; left: 16px; right: 16px; display: flex; align-items: center; gap: 16px; height: 60px; padding: 16px; border-radius: 16px; background: var(--color-surface); }
.playback-button { width: 36px; height: 36px; border-radius: 50%; }
.playback-track { flex: 1; height: 8px; min-height: 8px; }
.playback-clock { width: 96px; height: 24px; }
</style>
