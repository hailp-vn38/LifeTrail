<script setup lang="ts">
import type { NavItem } from "./use-nav-items";

defineProps<{ item: NavItem; active: boolean; collapsed: boolean }>();
</script>

<template>
  <RouterLink
    :to="item.to"
    class="sidebar-nav-item"
    :class="{ 'is-active': active }"
    :aria-current="active ? 'page' : undefined"
    :title="collapsed ? item.label : undefined"
  >
    <component :is="item.icon" :size="20" class="sidebar-nav-item__icon" aria-hidden="true" />
    <span class="sidebar-nav-item__label">{{ item.label }}</span>
  </RouterLink>
</template>

<style scoped>
.sidebar-nav-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.6rem 0.75rem;
  border-radius: var(--radius-sm);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  font-weight: 550;
  transition: background 0.15s ease, color 0.15s ease;
}
.sidebar-nav-item:hover {
  background: #eef3fb;
  color: var(--color-text);
  text-decoration: none;
}
.sidebar-nav-item.is-active {
  background: var(--color-primary-soft);
  color: var(--color-primary);
}
.sidebar-nav-item__icon {
  flex: none;
}
.sidebar-nav-item__label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sidebar-collapsed .sidebar-nav-item {
  justify-content: center;
  padding: 0.6rem 0;
}
.sidebar-collapsed .sidebar-nav-item__label {
  display: none;
}
@media (max-width: 1279px) {
  .sidebar-nav-item {
    justify-content: center;
    padding: 0.6rem 0;
  }
  .sidebar-nav-item__label {
    display: none;
  }
}
</style>
