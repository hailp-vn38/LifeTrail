<script setup lang="ts">
import AppSkeleton from "../../../components/ui/AppSkeleton.vue";
import AppSpinner from "../../../components/ui/AppSpinner.vue";
</script>

<template>
  <div class="daily-map-loading" role="status" aria-label="Đang tải Daily Map">
    <p class="daily-map-loading__label"><AppSpinner /> Đang tải Daily View…</p>
    <AppSkeleton :lines="4" />
  </div>
</template>

<style scoped>
.daily-map-loading {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-card);
  padding: 1.5rem;
}
.daily-map-loading__label {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-weight: 600;
}
</style>
