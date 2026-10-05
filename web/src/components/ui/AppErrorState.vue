<script setup lang="ts">
import { TriangleAlert } from "lucide-vue-next";
import AppButton from "./AppButton.vue";

withDefaults(
  defineProps<{
    title?: string;
    message: string;
    retryLabel?: string;
  }>(),
  { title: "Không thể tải dữ liệu", retryLabel: "Thử lại" },
);

defineEmits<{ retry: [] }>();
</script>

<template>
  <div class="app-error-state" role="alert">
    <span class="app-error-state__icon" aria-hidden="true">
      <TriangleAlert :size="28" />
    </span>
    <h2 class="app-error-state__title">{{ title }}</h2>
    <p class="app-error-state__message">{{ message }}</p>
    <AppButton variant="secondary" size="sm" @click="$emit('retry')">{{ retryLabel }}</AppButton>
  </div>
</template>

<style scoped>
.app-error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.6rem;
  text-align: center;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 2.5rem 1.5rem;
}
.app-error-state__icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 3rem;
  height: 3rem;
  border-radius: 999px;
  background: #fdecec;
  color: var(--color-danger);
}
.app-error-state__title {
  font-size: var(--font-size-md);
  font-weight: 650;
}
.app-error-state__message {
  font-size: var(--font-size-sm);
  color: var(--color-text-muted);
  max-width: 28rem;
}
</style>
