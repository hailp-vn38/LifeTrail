import { createApp } from "vue";
import { createPinia } from "pinia";
import { VueQueryPlugin } from "@tanstack/vue-query";
import App from "../App.vue";
import { configureMapLibreWorker } from "../maplibre-worker";
import "../styles/main.css";
import { createLifeTrailRouter } from "./router";
import { createQueryClient } from "./query-client";

export function bootstrap(selector = "#app") {
  configureMapLibreWorker();

  const app = createApp(App);
  app.use(createPinia());
  app.use(createLifeTrailRouter());
  app.use(VueQueryPlugin, { queryClient: createQueryClient() });
  app.mount(selector);

  return app;
}
