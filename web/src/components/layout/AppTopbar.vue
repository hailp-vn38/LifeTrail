<script setup lang="ts">
import { PanelLeft } from "lucide-vue-next";
import { useRoute } from "vue-router";
import { computed } from "vue";
import AppBadge from "../ui/AppBadge.vue";
import AppIconButton from "../ui/AppIconButton.vue";
import { useSystemStatus } from "../../api/queries/health.query";
import { useUiStore } from "../../stores/ui.store";

const route = useRoute();
const ui = useUiStore();
const statusQuery = useSystemStatus();

const title = computed(() => String(route.meta.title ?? "LifeTrail"));
const apiOk = computed(() => statusQuery.data.value?.ok === true);
</script>

<template>
  <header class="app-topbar">
    <div class="app-topbar__left">
      <AppIconButton
        :icon="PanelLeft"
        :label="ui.sidebarCollapsed ? 'Mở rộng sidebar' : 'Thu gọn sidebar'"
        variant="ghost"
        class="app-topbar__collapse"
        :active="ui.sidebarCollapsed"
        @click="ui.toggleSidebar()"
      />
      <h1 class="app-topbar__title">{{ title }}</h1>
    </div>
    <div class="app-topbar__right">
      <slot name="actions" />
      <AppBadge :variant="apiOk ? 'success' : 'muted'">
        {{ apiOk ? "API online" : "API…" }}
      </AppBadge>
    </div>
  </header>
</template>

<style scoped>
.app-topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  height: var(--topbar-height);
  padding: 0 1.5rem;
  background: var(--color-surface);
  border-bottom: 1px solid var(--color-border);
  position: sticky;
  top: 0;
  z-index: 10;
}
.app-topbar__left {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  min-width: 0;
}
.app-topbar__title {
  font-size: var(--font-size-lg);
  font-weight: 650;
  letter-spacing: -0.01em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.app-topbar__right {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}
@media (max-width: 767px) {
  .app-topbar {
    padding: 0 1rem;
  }
  .app-topbar__collapse {
    display: none;
  }
}
</style>
