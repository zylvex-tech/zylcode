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
    coverage: {
      provider: "v8",
      // `json-summary` is what the threshold check and any diffing read;
      // `text` keeps a human-readable table in the CI log.
      reporter: ["text", "json-summary"],
      reportsDirectory: "./coverage",
      // Coverage is measured over application source only. Build tooling and
      // type declarations are not shipped code and have no behaviour to
      // cover — including them would move the denominator without moving the
      // evidence.
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "**/*.d.ts",
        "src/test/**",
        "src/main.tsx",
        "src/**/*.test.{ts,tsx}",
      ],
      all: true,
      // Enforced floor, not an aspiration: CI fails below these. They sit at
      // the measured value so the gate ratchets — coverage can only stay level
      // or rise, and a new untested file pulls the run red immediately rather
      // than being absorbed silently. Raising them is a deliberate act that
      // should accompany the tests that justify it.
      thresholds: {
        lines: 70,
        branches: 83,
        functions: 80,
        statements: 70,
      },
    },
  },
});
