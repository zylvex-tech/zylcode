import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    proxy: {
      // Browser preview: forward backend surface requests to the local
      // `zylcode serve-intel` service.
      "/api/repo-intel": "http://127.0.0.1:17630",
      "/api/git": "http://127.0.0.1:17630",
      "/api/version": "http://127.0.0.1:17630",
      "/api/tools": "http://127.0.0.1:17630",
      "/api/providers": "http://127.0.0.1:17630",
      "/api/metrics": "http://127.0.0.1:17630",
      "/api/search": "http://127.0.0.1:17630",
      "/api/files": "http://127.0.0.1:17630",
      "/api/file-content": "http://127.0.0.1:17630",
      "/api/evidence": "http://127.0.0.1:17630",
      "/api/terminal": "http://127.0.0.1:17630",
      "/api/missions": "http://127.0.0.1:17630",
      "/api/recent-files": "http://127.0.0.1:17630",
      "/api/projects": "http://127.0.0.1:17630",
      "/api/artifacts": "http://127.0.0.1:17630",
      "/api/proofs": "http://127.0.0.1:17630",
      "/api/models": "http://127.0.0.1:17630",
      "/api/deploy": "http://127.0.0.1:17630",
      "/api/build": "http://127.0.0.1:17630",
      "/api/package": "http://127.0.0.1:17630",
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: process.env.TAURI_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
