import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// En Android, `tauri android dev` define TAURI_DEV_HOST con la IP de tu PC
// para que el teléfono/emulador pueda alcanzar el servidor de Vite.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
