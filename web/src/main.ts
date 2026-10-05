import { createApp } from "vue";
import { QueryClient, VueQueryPlugin } from "@tanstack/vue-query";
import App from "./App.vue";
import "maplibre-gl/dist/maplibre-gl.css";
import { configureMapLibreWorker } from "./maplibre-worker";
import "./styles.css";
import { createLifeTrailRouter } from "./router";

configureMapLibreWorker();

const app = createApp(App);
app.use(createLifeTrailRouter());
app.use(VueQueryPlugin, { queryClient: new QueryClient() });
app.mount("#app");
