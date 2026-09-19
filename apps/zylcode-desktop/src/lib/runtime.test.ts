import { afterEach, describe, expect, it } from "vitest";
import {
  DESKTOP_REQUIRED_MESSAGE,
  clearDiagnostics,
  detectEnvironment,
  getDiagnostics,
  isDesktopRuntime,
  recordDiagnostic,
  safeInvoke,
  unwrapInvoke,
  type RuntimeEnvironment,
} from "./runtime";

// ---------------------------------------------------------------------------
// Environment detection + safe IPC bridge (Product Architecture v3 §11).
// The core guarantee: outside the Tauri desktop runtime, NO Tauri IPC is
// invoked and failures surface as controlled results, not raw exceptions.
// ---------------------------------------------------------------------------

describe("detectEnvironment", () => {
  const originalInternals = (globalThis as Record<string, unknown>).__TAURI_INTERNALS__;

  afterEach(() => {
    if (originalInternals === undefined) {
      delete (globalThis as Record<string, unknown>).__TAURI_INTERNALS__;
    } else {
      (globalThis as Record<string, unknown>).__TAURI_INTERNALS__ = originalInternals;
    }
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("classifies the test runner as TEST, not TAURI_DESKTOP", () => {
    expect(detectEnvironment()).toBe("TEST");
  });

  it("classifies a browser page with Tauri internals as TAURI_DESKTOP", () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = { some: "internals" };
    expect(detectEnvironment()).toBe("TAURI_DESKTOP");
  });

  it("classifies globalThis Tauri internals as TAURI_DESKTOP", () => {
    (globalThis as Record<string, unknown>).__TAURI_INTERNALS__ = { some: "internals" };
    expect(detectEnvironment()).toBe("TAURI_DESKTOP");
  });

  it("never reports TAURI_DESKTOP in plain jsdom", () => {
    expect(detectEnvironment()).not.toBe("TAURI_DESKTOP");
  });
});

describe("isDesktopRuntime", () => {
  const envs: RuntimeEnvironment[] = [
    "TAURI_DESKTOP",
    "BROWSER_PREVIEW",
    "TEST",
    "UNKNOWN",
  ];
  it("is true only for TAURI_DESKTOP", () => {
    for (const env of envs) {
      expect(isDesktopRuntime(env)).toBe(env === "TAURI_DESKTOP");
    }
  });
});

describe("safeInvoke outside the desktop runtime", () => {
  it("resolves with a controlled failure and never invokes Tauri IPC", async () => {
    const result = await safeInvoke<{ tools: string[] }>("TEST", "list_tools");
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.reason).toBe(DESKTOP_REQUIRED_MESSAGE);
    }
  });

  it("also refuses in BROWSER_PREVIEW", async () => {
    const result = await safeInvoke("BROWSER_PREVIEW", "verify_logic");
    expect(result.ok).toBe(false);
  });

  it("does not record a diagnostic for the controlled non-desktop refusal", async () => {
    const before = getDiagnostics().length;
    await safeInvoke("TEST", "list_tools");
    expect(getDiagnostics().length).toBe(before);
  });
});

describe("diagnostics sink", () => {
  it("records and clears entries", () => {
    recordDiagnostic("test.source", "boom");
    const entries = getDiagnostics();
    expect(entries.length).toBeGreaterThan(0);
    const last = entries[entries.length - 1];
    expect(last.source).toBe("test.source");
    expect(last.message).toBe("boom");
    expect(last.timestamp).toBeTruthy();
    clearDiagnosticsAndAssert();
  });

  function clearDiagnosticsAndAssert() {
    clearDiagnostics();
    expect(getDiagnostics().length).toBe(0);
  }
});

describe("unwrapInvoke", () => {
  it("returns data on success", () => {
    expect(unwrapInvoke({ ok: true, data: 42 })).toBe(42);
  });

  it("throws so existing try/catch handling keeps working", () => {
    expect(() => unwrapInvoke({ ok: false, reason: "nope" })).toThrow("nope");
  });
});
