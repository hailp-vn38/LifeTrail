<script setup lang="ts">
defineProps<{ title: string; description?: string }>();
</script>

<template>
  <div class="page-header">
    <div class="page-header__text">
      <h2 class="page-title">{{ title }}</h2>
      <p v-if="description" class="text-muted text-sm">{{ description }}</p>
    </div>
    <div v-if="$slots.actions" class="page-header__actions">
      <slot name="actions" />
    </div>
  </div>
</template>

<style scoped>
.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
}
.page-header__text {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
.page-header__actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}
</style>
