import type { RouteRecordRaw } from "vue-router";
import AppLayout from "../layouts/AppLayout.vue";

declare module "vue-router" {
  interface RouteMeta {
    /** Page title shown in the topbar. */
    title: string;
    /** Sidebar entry this route belongs to (for active highlighting). */
    nav?: "overview" | "daily-map" | "timeline" | "devices" | "reports" | "settings";
  }
}

/**
 * Application routes.
 *
 * The layout is part of routing: the root route renders `AppLayout` once and
 * every page renders inside it, so the sidebar/topbar stay mounted across
 * page navigation. The canonical Daily Map URL
 * `/devices/:deviceId/day/:date` is preserved.
 */
export const appRoutes: RouteRecordRaw[] = [
  {
    path: "/",
    component: AppLayout,
    children: [
      {
        path: "",
        name: "overview",
        component: () => import("../features/overview/pages/OverviewPage.vue"),
        meta: { title: "Overview", nav: "overview" },
      },
      {
        path: "devices",
        name: "devices",
        component: () => import("../features/devices/pages/DeviceListPage.vue"),
        meta: { title: "Devices", nav: "devices" },
      },
      {
        path: "devices/:deviceId",
        name: "device-detail",
        component: () => import("../features/devices/pages/DeviceDetailPage.vue"),
        meta: { title: "Device", nav: "devices" },
      },
      {
        path: "devices/:deviceId/day/:date",
        name: "daily-view",
        component: () => import("../features/daily-map/pages/DailyMapPage.vue"),
        meta: { title: "Daily Map", nav: "daily-map" },
      },
      {
        path: "timeline",
        name: "timeline",
        component: () => import("../features/timeline/pages/TimelinePage.vue"),
        meta: { title: "Timeline", nav: "timeline" },
      },
      {
        path: "reports",
        name: "reports",
        component: () => import("../features/reports/pages/ReportsPage.vue"),
        meta: { title: "Reports", nav: "reports" },
      },
      {
        path: "settings",
        name: "settings",
        component: () => import("../features/settings/pages/SettingsPage.vue"),
        meta: { title: "Settings", nav: "settings" },
      },
    ],
  },
  { path: "/:pathMatch(.*)*", redirect: "/" },
];
