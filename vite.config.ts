import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;
const page = (name: string) => fileURLToPath(new URL(name, import.meta.url));

// Cấu hình theo mẫu của Tauri: cổng cố định để `tauri dev` tìm được dev server.
// Hai trang: cửa sổ nhật ký (index.html) và menu tròn (menu.html).
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    rollupOptions: {
      input: { main: page("index.html"), menu: page("menu.html") },
    },
  },
});
