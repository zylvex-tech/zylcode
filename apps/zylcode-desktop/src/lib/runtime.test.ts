import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  DESKTOP_REQUIRED_MESSAGE,
  clearDiagnostics,
  detectEnvironment,
  getDiagnostics,
  getTauriRuntime,
  isDesktopRuntime,
  recordDiagnostic,
  safeInvoke,
  unwrapInvoke,
  type RuntimeEnvironment,
} from "./runtime";

// `safeInvoke` reaches Tauri through a dynamic import, so the module has to be
// stubbed for the desktop branch to be exercisable outside a real shell.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

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

  it("degrades to UNKNOWN and records a diagnostic when probing throws", () => {
    clearDiagnostics();
    Object.defineProperty(globalThis, "__TAURI_INTERNALS__", {
      configurable: true,
      get() {
        throw new Error("probe exploded");
      },
    });

    expect(detectEnvironment()).toBe("UNKNOWN");
    const entry = getDiagnostics().find((d) => d.source === "runtime.detectEnvironment");
    expect(entry?.message).toContain("probe exploded");
  });
});

describe("isDesktopRuntime", () => {
  const envs: RuntimeEnvironment[] = ["TAURI_DESKTOP", "BROWSER_PREVIEW", "TEST", "UNKNOWN"];
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

describe("safeInvoke inside the desktop runtime", () => {
  const invokeMock = vi.mocked(invoke);

  beforeEach(() => {
    clearDiagnostics();
    invokeMock.mockReset();
  });

  it("returns the payload the engine resolved with", async () => {
    invokeMock.mockResolvedValue({ tools: ["terminal_exec"] });

    const result = await safeInvoke<{ tools: string[] }>("TAURI_DESKTOP", "list_tools");

    expect(result).toEqual({ ok: true, data: { tools: ["terminal_exec"] } });
    expect(invokeMock).toHaveBeenCalledWith("list_tools", undefined);
  });

  it("forwards the command arguments it was given", async () => {
    invokeMock.mockResolvedValue(null);

    await safeInvoke("TAURI_DESKTOP", "execute_tool", { id: "read_file", params: {} });

    expect(invokeMock).toHaveBeenCalledWith("execute_tool", {
      id: "read_file",
      params: {},
    });
  });

  it("captures a rejected invoke as a failure instead of throwing", async () => {
    invokeMock.mockRejectedValue(new Error("engine exploded"));

    const result = await safeInvoke("TAURI_DESKTOP", "execute_tool");

    expect(result).toEqual({ ok: false, reason: "Error: engine exploded" });
  });

  it("mirrors the raw failure to Diagnostics under the command name", async () => {
    invokeMock.mockRejectedValue(new Error("engine exploded"));

    await safeInvoke("TAURI_DESKTOP", "execute_tool");

    const entry = getDiagnostics().find((d) => d.source === "ipc.execute_tool");
    expect(entry?.message).toContain("engine exploded");
  });

  it("refuses to touch IPC when the caller passes a non-desktop environment", async () => {
    const result = await safeInvoke("BROWSER_PREVIEW", "list_tools");

    expect(result.ok).toBe(false);
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

describe("diagnostics buffer bound", () => {
  it("drops the oldest entry once the cap is exceeded", () => {
    clearDiagnostics();
    for (let i = 0; i < 205; i++) recordDiagnostic("overflow", `m${i}`);

    const entries = getDiagnostics();
    expect(entries.length).toBe(200);
    // The first five were shifted out; the oldest survivor is m5.
    expect(entries[0].message).toBe("m5");
    expect(entries[entries.length - 1].message).toBe("m204");

    clearDiagnostics();
  });
});

describe("getTauriRuntime", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    delete (window as unknown as Record<string, unknown>).__TAURI__;
  });

  it("stays a no-op outside a window, as well as outside the desktop", () => {
    vi.stubGlobal("window", undefined);
    expect(getTauriRuntime()).toBeNull();
  });

  it("reports a present-but-nullish injection as absent rather than crashing", () => {
    (window as unknown as Record<string, unknown>).__TAURI__ = null;
    expect(getTauriRuntime()).toBeNull();
  });
});
