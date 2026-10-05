<script setup lang="ts">
withDefaults(defineProps<{ lines?: number }>(), { lines: 3 });
</script>

<template>
  <div class="app-skeleton" role="status" aria-label="Đang tải">
    <div v-for="index in lines" :key="index" class="app-skeleton__line" />
  </div>
</template>

<style scoped>
.app-skeleton {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.app-skeleton__line {
  height: 0.9rem;
  border-radius: var(--radius-sm);
  background: linear-gradient(90deg, #edf1f7 25%, #f7f9fc 50%, #edf1f7 75%);
  background-size: 200% 100%;
  animation: app-skeleton-shimmer 1.4s ease-in-out infinite;
}
.app-skeleton__line:last-child {
  width: 60%;
}
@keyframes app-skeleton-shimmer {
  0% {
    background-position: 200% 0;
  }
  100% {
    background-position: -200% 0;
  }
}
@media (prefers-reduced-motion: reduce) {
  .app-skeleton__line {
    animation: none;
  }
}
</style>
