<script setup lang="ts">
withDefaults(defineProps<{ variant?: "info" | "success" | "danger" | "muted" | "warning" }>(), {
  variant: "muted",
});
</script>

<template>
  <span class="app-badge" :class="`app-badge--${variant}`"><slot /></span>
</template>

<style scoped>
.app-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.2rem 0.6rem;
  border-radius: 999px;
  font-size: var(--font-size-xs);
  font-weight: 600;
  white-space: nowrap;
}
.app-badge--info {
  background: var(--color-primary-soft);
  color: var(--color-primary);
}
.app-badge--success {
  background: #e6f9f0;
  color: #0e9f6e;
}
.app-badge--danger {
  background: #fdecec;
  color: var(--color-danger);
}
.app-badge--warning {
  background: #fff4e5;
  color: var(--color-warning);
}
.app-badge--muted {
  background: #eef1f6;
  color: var(--color-text-muted);
}
</style>
