import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Frontend unit/integration tests for the ZylCode desktop shell.
// Tests run in jsdom; the runtime bridge must classify the environment as
// TEST (never TAURI_DESKTOP), so no Tauri IPC is ever touched here.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: false,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
