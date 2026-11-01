import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

const root = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
  },
  build: {
    rollupOptions: {
      input: {
        widget: resolve(root, "widget.html"),
        overlay: resolve(root, "overlay.html"),
        bubble: resolve(root, "bubble.html"),
        settings: resolve(root, "settings.html"),
        chat: resolve(root, "chat.html"),
      },
    },
  },
});
