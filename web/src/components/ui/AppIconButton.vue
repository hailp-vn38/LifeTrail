<script setup lang="ts">
import type { Component } from "vue";

withDefaults(
  defineProps<{
    icon: Component;
    label: string;
    variant?: "default" | "primary" | "ghost";
    disabled?: boolean;
    active?: boolean;
  }>(),
  { variant: "default", disabled: false, active: false },
);
</script>

<template>
  <button
    type="button"
    class="app-icon-button"
    :class="[`app-icon-button--${variant}`, { 'is-active': active }]"
    :aria-label="label"
    :title="label"
    :disabled="disabled"
  >
    <component :is="icon" :size="18" aria-hidden="true" />
  </button>
</template>

<style scoped>
.app-icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.25rem;
  height: 2.25rem;
  border-radius: var(--radius-sm);
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  color: var(--color-text-muted);
  transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease;
}
.app-icon-button:hover:not(:disabled) {
  color: var(--color-primary);
  border-color: var(--color-primary);
}
.app-icon-button--primary {
  background: var(--color-primary);
  border-color: var(--color-primary);
  color: #fff;
}
.app-icon-button--primary:hover:not(:disabled) {
  background: #0f66e0;
  color: #fff;
}
.app-icon-button--ghost {
  border-color: transparent;
  background: transparent;
}
.app-icon-button.is-active {
  background: var(--color-primary-soft);
  border-color: var(--color-primary);
  color: var(--color-primary);
}
.app-icon-button:disabled {
  opacity: 0.5;
}
</style>
