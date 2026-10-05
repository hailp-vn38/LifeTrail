<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "sm" | "md";
    disabled?: boolean;
    type?: "button" | "submit" | "reset";
  }>(),
  { variant: "secondary", size: "md", disabled: false, type: "button" },
);
</script>

<template>
  <button
    :type="type"
    :disabled="disabled"
    class="app-button"
    :class="[`app-button--${variant}`, `app-button--${size}`]"
  >
    <slot />
  </button>
</template>

<style scoped>
.app-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  font-weight: 600;
  transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease;
}
.app-button--md {
  padding: 0.55rem 1rem;
  font-size: var(--font-size-sm);
}
.app-button--sm {
  padding: 0.35rem 0.75rem;
  font-size: var(--font-size-xs);
}
.app-button--primary {
  background: var(--color-primary);
  color: #fff;
}
.app-button--primary:hover:not(:disabled) {
  background: #0f66e0;
}
.app-button--secondary {
  background: var(--color-surface);
  border-color: var(--color-border);
  color: var(--color-text);
}
.app-button--secondary:hover:not(:disabled) {
  border-color: var(--color-primary);
  color: var(--color-primary);
}
.app-button--ghost {
  background: transparent;
  color: var(--color-text-muted);
}
.app-button--ghost:hover:not(:disabled) {
  background: var(--color-primary-soft);
  color: var(--color-primary);
}
.app-button--danger {
  background: var(--color-danger);
  color: #fff;
}
.app-button:disabled {
  opacity: 0.55;
}
</style>
