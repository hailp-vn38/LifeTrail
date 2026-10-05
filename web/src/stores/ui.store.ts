import { defineStore } from "pinia";
import { ref } from "vue";

/**
 * Shared UI chrome state: sidebar, mobile navigation, and the last visited
 * Daily Map route (so the "Daily Map" sidebar entry can deep-link back).
 *
 * This store never holds server data; remote state belongs to TanStack Query.
 */
export const useUiStore = defineStore("ui", () => {
  const sidebarCollapsed = ref(false);
  const lastDailyMapRoute = ref<string | null>(null);

  function toggleSidebar(): void {
    sidebarCollapsed.value = !sidebarCollapsed.value;
  }

  function setLastDailyMapRoute(route: string): void {
    lastDailyMapRoute.value = route;
  }

  return {
    sidebarCollapsed,
    lastDailyMapRoute,
    toggleSidebar,
    setLastDailyMapRoute,
  };
});
