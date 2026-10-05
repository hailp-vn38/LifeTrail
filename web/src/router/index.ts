import {
  createWebHistory,
  createRouter,
  type RouterHistory,
} from "vue-router";
import DailyView from "../views/DailyView.vue";
import DeviceListView from "../views/DeviceListView.vue";

export function createLifeTrailRouter(history: RouterHistory = createWebHistory()) {
  return createRouter({
    history,
    routes: [
      { path: "/", component: DeviceListView, name: "device-list" },
      {
        path: "/devices/:deviceId/day/:date",
        component: DailyView,
        name: "daily-view",
      },
    ],
  });
}
