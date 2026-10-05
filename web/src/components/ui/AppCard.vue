<script setup lang="ts">
defineProps<{ title?: string }>();
</script>

<template>
  <section class="app-card">
    <h2 v-if="title" class="app-card__title">{{ title }}</h2>
    <slot />
  </section>
</template>

<style scoped>
.app-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-card);
  padding: 1.25rem;
}
.app-card__title {
  font-size: var(--font-size-sm);
  font-weight: 650;
  letter-spacing: 0.02em;
  margin-bottom: 0.75rem;
}
</style>
