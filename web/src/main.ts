import { createApp } from "vue";
import { QueryClient, VueQueryPlugin } from "@tanstack/vue-query";
import App from "./App.vue";
import "maplibre-gl/dist/maplibre-gl.css";
import "./styles.css";
import { createLifeTrailRouter } from "./router";

const app = createApp(App);
app.use(createLifeTrailRouter());
app.use(VueQueryPlugin, { queryClient: new QueryClient() });
app.mount("#app");
