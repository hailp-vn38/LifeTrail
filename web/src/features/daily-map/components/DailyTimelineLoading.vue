<script setup lang="ts">
import AppShimmerBlock from "../../../components/ui/AppShimmerBlock.vue";
</script>

<template>
  <div class="daily-timeline-loading" aria-hidden="true">
    <AppShimmerBlock class="daily-timeline-loading__title" />
    <div class="daily-timeline-loading__summary">
      <AppShimmerBlock v-for="item in 3" :key="item" />
    </div>
    <div v-for="item in 6" :key="item" class="daily-timeline-loading__event">
      <AppShimmerBlock class="daily-timeline-loading__icon" />
      <div class="daily-timeline-loading__details">
        <AppShimmerBlock class="daily-timeline-loading__name" />
        <AppShimmerBlock class="daily-timeline-loading__time" />
        <AppShimmerBlock class="daily-timeline-loading__caption" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.daily-timeline-loading { display: flex; flex-direction: column; gap: 16px; height: 100%; min-height: 0; overflow: hidden; padding: 12px; background: var(--color-surface); border: 1px solid var(--color-border); border-radius: 12px; }
.daily-timeline-loading__title { height: 12px; width: 60%; flex-shrink: 0; }
.daily-timeline-loading__summary { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; padding: 12px; border-radius: 8px; background: #f5f8fc; }
.daily-timeline-loading__summary > span { height: 48px; }
.daily-timeline-loading__event { display: flex; gap: 12px; align-items: flex-start; }
.daily-timeline-loading__icon { width: 28px; height: 28px; border-radius: 50%; flex-shrink: 0; }
.daily-timeline-loading__details { display: grid; gap: 8px; flex: 1; padding: 8px; }
.daily-timeline-loading__name { width: 75%; height: 14px; }
.daily-timeline-loading__time { width: 55%; height: 10px; }
.daily-timeline-loading__caption { width: 90%; height: 10px; }
@media (max-width: 767px) { .daily-timeline-loading { height: auto; } }
</style>
