// ---------------------------------------------------------------------------
// ZylCode runtime environment bridge (Product Architecture v3 §11)
//
// Typed environment detection + safe IPC. Outside the Tauri desktop runtime,
// NO Tauri IPC is ever invoked: backend-dependent UI renders controlled
// states ("Desktop runtime required"), raw exception details go to
// Diagnostics only, and event subscriptions no-op safely.
// ---------------------------------------------------------------------------

export type RuntimeEnvironment =
  | "TAURI_DESKTOP"
  | "BROWSER_PREVIEW"
  | "TEST"
  | "UNKNOWN";

/** Diagnostics sink: raw technical errors land here, never in normal UI. */
export type DiagnosticEntry = {
  timestamp: string;
  source: string;
  message: string;
};

const diagnostics: DiagnosticEntry[] = [];
const MAX_DIAGNOSTICS = 200;

export function recordDiagnostic(source: string, message: string): void {
  diagnostics.push({ timestamp: new Date().toISOString(), source, message });
  if (diagnostics.length > MAX_DIAGNOSTICS) diagnostics.shift();
}

export function getDiagnostics(): readonly DiagnosticEntry[] {
  return diagnostics;
}

export function clearDiagnostics(): void {
  diagnostics.length = 0;
}

/**
 * Detect the current runtime environment.
 *
 * Order: Tauri desktop (its injected internals global) → test runner →
 * browser preview. `UNKNOWN` only if probing throws.
 */
export function detectEnvironment(): RuntimeEnvironment {
  try {
    const g = globalThis as Record<string, unknown>;
    if (
      g.__TAURI_INTERNALS__ !== undefined ||
      g.__TAURI__ !== undefined ||
      (typeof window !== "undefined" &&
        (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ !==
          undefined)
    ) {
      return "TAURI_DESKTOP";
    }
    // Test runners: vitest injects these globals. Accessed via globalThis so
    // no Node types are required in the browser build.
    const proc = (globalThis as { process?: { env?: Record<string, string | undefined> } })
      .process;
    if (proc?.env?.VITEST === "true" || proc?.env?.NODE_ENV === "test") {
      return "TEST";
    }
    return "BROWSER_PREVIEW";
  } catch (e) {
    recordDiagnostic("runtime.detectEnvironment", String(e));
    return "UNKNOWN";
  }
}

export const ENVIRONMENT_LABEL: Record<RuntimeEnvironment, string> = {
  TAURI_DESKTOP: "Desktop runtime",
  BROWSER_PREVIEW: "Browser preview",
  TEST: "Test",
  UNKNOWN: "Unknown environment",
};

/** True only when full Tauri IPC (invoke/events) is available. */
export function isDesktopRuntime(env: RuntimeEnvironment): boolean {
  return env === "TAURI_DESKTOP";
}

/** Controlled, user-facing explanation for missing desktop capability. */
export const DESKTOP_REQUIRED_MESSAGE =
  "Desktop runtime required — this panel uses the ZylCode desktop engine. " +
  "Run ZylCode as the desktop app to enable it.";

/** Controlled explanation when the environment could not be determined. */
export const ENVIRONMENT_UNKNOWN_MESSAGE =
  "Runtime environment could not be determined — backend panels are disabled. " +
  "See Diagnostics for details.";

/**
 * Safe invoke: never touches Tauri IPC outside the desktop runtime.
 * Returns `{ ok: false, reason }` instead of throwing in browser preview.
 * Inside the desktop runtime, errors are returned (not thrown) and mirrored
 * to Diagnostics.
 */
export async function safeInvoke<T>(
  env: RuntimeEnvironment,
  cmd: string,
  args?: Record<string, unknown>,
): Promise<{ ok: true; data: T } | { ok: false; reason: string }> {
  if (!isDesktopRuntime(env)) {
    return { ok: false, reason: DESKTOP_REQUIRED_MESSAGE };
  }
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const data = await invoke<T>(cmd, args);
    return { ok: true, data };
  } catch (e) {
    recordDiagnostic(`ipc.${cmd}`, String(e));
    return { ok: false, reason: String(e) };
  }
}
