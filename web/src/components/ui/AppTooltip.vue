<script setup lang="ts">
defineProps<{ text: string }>();
</script>

<template>
  <span class="app-tooltip" :data-tooltip="text">
    <slot />
  </span>
</template>

<style scoped>
.app-tooltip {
  position: relative;
  display: inline-flex;
}
.app-tooltip::after {
  content: attr(data-tooltip);
  position: absolute;
  bottom: calc(100% + 6px);
  left: 50%;
  transform: translateX(-50%) translateY(4px);
  padding: 0.3rem 0.6rem;
  border-radius: var(--radius-sm);
  background: #1f2937;
  color: #fff;
  font-size: var(--font-size-xs);
  white-space: nowrap;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.15s ease, transform 0.15s ease;
  z-index: 50;
}
.app-tooltip:hover::after,
.app-tooltip:focus-within::after {
  opacity: 1;
  transform: translateX(-50%) translateY(0);
}
</style>
