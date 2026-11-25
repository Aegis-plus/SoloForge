import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

// Tauri expects a fixed port and no clearing of its logs.
export default defineConfig({
  plugins: [solid()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: "127.0.0.1" },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: { target: "es2022", sourcemap: false, outDir: "dist" },
  test: { environment: "jsdom", include: ["src/**/*.test.{ts,tsx}"] },
});
