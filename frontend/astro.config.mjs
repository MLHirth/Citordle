import { defineConfig } from "astro/config";
import react from "@astrojs/react";

export default defineConfig({
  integrations: [react()],
  vite: {
    server: {
      proxy: {
        "/api": "http://127.0.0.1:8080",
        "/health": "http://127.0.0.1:8080"
      }
    }
  }
});
