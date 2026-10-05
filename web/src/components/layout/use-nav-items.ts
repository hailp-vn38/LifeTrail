import { computed, type Component } from "vue";
import {
  BarChart3,
  Clock,
  Cpu,
  LayoutDashboard,
  Map,
  Settings,
} from "lucide-vue-next";
import { useUiStore } from "../../stores/ui.store";
import type { RouteMeta } from "vue-router";

export interface NavItem {
  key: string;
  label: string;
  icon: Component;
  to: string;
  nav: NonNullable<RouteMeta["nav"]>;
}

/**
 * Sidebar navigation items (Option 2 spec §9).
 *
 * "Daily Map" deep-links to the last visited Daily Map route so the entry
 * stays useful across sessions; it falls back to the device list.
 */
export function useNavItems() {
  const ui = useUiStore();
  return computed<NavItem[]>(() => [
    { key: "overview", label: "Overview", icon: LayoutDashboard, to: "/", nav: "overview" },
    {
      key: "daily-map",
      label: "Daily Map",
      icon: Map,
      to: ui.lastDailyMapRoute ?? "/devices",
      nav: "daily-map",
    },
    { key: "timeline", label: "Timeline", icon: Clock, to: "/timeline", nav: "timeline" },
    { key: "devices", label: "Devices", icon: Cpu, to: "/devices", nav: "devices" },
    { key: "reports", label: "Reports", icon: BarChart3, to: "/reports", nav: "reports" },
    { key: "settings", label: "Settings", icon: Settings, to: "/settings", nav: "settings" },
  ]);
}
