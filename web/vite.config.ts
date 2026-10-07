import { defineConfig } from "vitest/config";
import { loadEnv } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, ".", "LT_");
  return {
    plugins: [vue()],
    test: { environment: "jsdom" },
    server: {
      host: env.LT_DEV_HOST || "localhost",
      proxy: {
        "/api": env.LT_API_PROXY_TARGET || "http://127.0.0.1:8081",
      },
    },
  };
});
