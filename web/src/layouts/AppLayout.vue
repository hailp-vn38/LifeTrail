<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import AppSidebar from "../components/layout/AppSidebar.vue";
import AppTopbar from "../components/layout/AppTopbar.vue";
import { useNavItems } from "../components/layout/use-nav-items";
import { useMediaQuery } from "../composables/useMediaQuery";
import { useUiStore } from "../stores/ui.store";

const ui = useUiStore();
const route = useRoute();
const navItems = useNavItems();
// Tablet and below use the collapsed icon rail; mobile drops the sidebar
// entirely in favour of the bottom navigation.
const isCompactViewport = useMediaQuery("(max-width: 1279px)");
const collapsed = computed(() => ui.sidebarCollapsed || isCompactViewport.value);
</script>

<template>
  <div class="app-shell" :class="{ 'sidebar-collapsed': collapsed }">
    <AppSidebar />
    <div class="app-main">
      <AppTopbar />
      <main class="app-content">
        <RouterView />
      </main>
    </div>

    <nav class="mobile-nav" aria-label="Điều hướng di động">
      <RouterLink
        v-for="item in navItems"
        :key="item.key"
        :to="item.to"
        class="mobile-nav__item"
        :class="{ 'is-active': route.meta.nav === item.nav }"
        :aria-current="route.meta.nav === item.nav ? 'page' : undefined"
      >
        <component :is="item.icon" :size="20" aria-hidden="true" />
        <span class="mobile-nav__label">{{ item.label }}</span>
      </RouterLink>
    </nav>
  </div>
</template>

<style scoped>
.mobile-nav {
  display: none;
}
@media (max-width: 767px) {
  .mobile-nav {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 20;
    background: var(--color-surface);
    border-top: 1px solid var(--color-border);
    padding: 0.35rem 0.25rem calc(0.35rem + env(safe-area-inset-bottom));
  }
  .mobile-nav__item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.15rem;
    padding: 0.35rem 0;
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    font-size: 0.62rem;
    font-weight: 600;
  }
  .mobile-nav__item:hover {
    text-decoration: none;
  }
  .mobile-nav__item.is-active {
    color: var(--color-primary);
    background: var(--color-primary-soft);
  }
  .mobile-nav__label {
    white-space: nowrap;
  }
}
</style>
