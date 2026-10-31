import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
// Browser preview: forward backend surface requests to the local
// `zylcode serve-intel` service.
//
// `vite dev` and `vite preview` share this table. Without it the production
// bundle served by `preview` has no backend at all: a `POST /api/terminal/exec`
// lands on a bare static file server and comes back as HTTP 501, which the
// terminal then reports honestly as `terminal service error: HTTP 501`.
const backendProxy = {
      // Browser preview: forward backend surface requests to the local
      // `zylcode serve-intel` service.
      "/api/repo-intel": "http://127.0.0.1:17630",
      "/api/nav": "http://127.0.0.1:17630",
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
};

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    proxy: backendProxy,
  },
  preview: {
    port: 4173,
    strictPort: true,
    proxy: backendProxy,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: process.env.TAURI_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
