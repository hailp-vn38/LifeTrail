<script setup lang="ts">
import { Inbox } from "lucide-vue-next";
import type { Component } from "vue";
import AppButton from "./AppButton.vue";

withDefaults(
  defineProps<{
    icon?: Component;
    title: string;
    description?: string;
    actionLabel?: string;
  }>(),
  { icon: undefined, description: undefined, actionLabel: undefined },
);

defineEmits<{ action: [] }>();
</script>

<template>
  <div class="app-empty-state">
    <span class="app-empty-state__icon" aria-hidden="true">
      <component :is="icon ?? Inbox" :size="28" />
    </span>
    <h2 class="app-empty-state__title">{{ title }}</h2>
    <p v-if="description" class="app-empty-state__description">{{ description }}</p>
    <AppButton v-if="actionLabel" variant="primary" size="sm" @click="$emit('action')">
      {{ actionLabel }}
    </AppButton>
  </div>
</template>

<style scoped>
.app-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.6rem;
  text-align: center;
  background: var(--color-surface);
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-md);
  padding: 2.5rem 1.5rem;
}
.app-empty-state__icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 3rem;
  height: 3rem;
  border-radius: 999px;
  background: var(--color-primary-soft);
  color: var(--color-primary);
}
.app-empty-state__title {
  font-size: var(--font-size-md);
  font-weight: 650;
}
.app-empty-state__description {
  font-size: var(--font-size-sm);
  color: var(--color-text-muted);
  max-width: 28rem;
}
</style>
