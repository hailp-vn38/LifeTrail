<script setup lang="ts">
withDefaults(defineProps<{ label?: string; size?: number }>(), {
  label: "Đang tải",
  size: 20,
});
</script>

<template>
  <span class="app-spinner" role="status" :aria-label="label">
    <svg :width="size" :height="size" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="3" opacity="0.2" />
      <path
        d="M21 12a9 9 0 0 0-9-9"
        stroke="currentColor"
        stroke-width="3"
        stroke-linecap="round"
      />
    </svg>
  </span>
</template>

<style scoped>
.app-spinner {
  display: inline-flex;
  color: var(--color-primary);
}
.app-spinner svg {
  animation: app-spinner-rotate 0.9s linear infinite;
}
@keyframes app-spinner-rotate {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .app-spinner svg {
    animation: none;
  }
}
</style>
