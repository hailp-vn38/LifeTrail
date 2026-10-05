<script setup lang="ts">
import { useRoute } from "vue-router";
import SidebarNavItem from "./SidebarNavItem.vue";
import { useNavItems } from "./use-nav-items";
import { useUiStore } from "../../stores/ui.store";

const route = useRoute();
const ui = useUiStore();
const items = useNavItems();
</script>

<template>
  <nav class="sidebar-nav" aria-label="Primary">
    <SidebarNavItem
      v-for="item in items"
      :key="item.key"
      :item="item"
      :active="route.meta.nav === item.nav"
      :collapsed="ui.sidebarCollapsed"
    />
  </nav>
</template>

<style scoped>
.sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}
</style>
