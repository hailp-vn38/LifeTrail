<script setup lang="ts">
import SidebarFooter from "./SidebarFooter.vue";
import SidebarLogo from "./SidebarLogo.vue";
import SidebarNav from "./SidebarNav.vue";
</script>

<template>
  <aside class="app-sidebar" aria-label="Điều hướng chính">
    <div class="app-sidebar__top">
      <SidebarLogo />
    </div>
    <div class="app-sidebar__nav">
      <SidebarNav />
    </div>
    <SidebarFooter />
  </aside>
</template>

<style scoped>
.app-sidebar {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-height: 100dvh;
  padding: 1rem 0.75rem;
  background: var(--color-surface);
  border-right: 1px solid var(--color-border);
  position: sticky;
  top: 0;
}
.app-sidebar__nav {
  flex: 1;
}
@media (max-width: 767px) {
  .app-sidebar {
    display: none;
  }
}
</style>
