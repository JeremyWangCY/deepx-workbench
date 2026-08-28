import { defineConfig } from "vite";
import { resolve } from "node:path";
export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        "index.html": resolve(import.meta.dirname, "index.html"),
        "update.html": resolve(import.meta.dirname, "update.html"),
      },
    },
  },
});
