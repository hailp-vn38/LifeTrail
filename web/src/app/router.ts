import { createRouter, createWebHistory, type RouterHistory } from "vue-router";
import { appRoutes } from "./routes";

export function createLifeTrailRouter(history: RouterHistory = createWebHistory()) {
  return createRouter({
    history,
    routes: appRoutes,
  });
}
