// ---------------------------------------------------------------------------
// Tool runner service (lib/toolRunner.ts)
//
// Lets any surface *execute* a real MCP tool — the catalogue lists what
// exists, this is what runs it. One typed data path with two transports,
// mirroring `navIntel.ts`:
//
//   * Tauri desktop  -> `execute_tool` IPC command (the engine's registry;
//                       evidence attributes the actor as "user:desktop").
//   * Browser preview -> POST /api/execute-tool via the Vite proxy, served
//                       by `zylcode serve-intel` (the actor is attributed
//                       as "user:serve-intel").
//
// Honesty rules (main status report §Phase A — "wire it, never fake it"):
//   * a tool whose catalogue row lacks an executor is declined in the UI
//     before any transport is touched (fail-closed is the product contract,
//     not an error state to hide);
//   * a *failed* execution is rendered as a failure with the engine's
//     message — never re-shaped into an empty/OK result;
//   * `unavailable` means there was no runtime to ask at all.
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic } from "./runtime";

import type { ToolRow } from "./service";

/** Engine payload for one invocation (DynamicTool::call serialization). */
export type ToolRunResult = {
  tool: string;
  executed: boolean;
  success: boolean;
  output: unknown;
  bound_operation?: string;
  risk?: string;
  approval_decision?: string;
  invocation_id?: string;
  changed_files?: string[];
};

export type ToolRunState =
  | { kind: "idle" }
  | { kind: "running"; id: string }
  | { kind: "done"; result: ToolRunResult; via: "desktop" | "service" }
  /** The engine ran it and the tool failed — the engine's word is shown. */
  | { kind: "failed"; id: string; via: "desktop" | "service"; error: string }
  /** Declined up front: no executor is bound to this id. */
  | { kind: "declined"; id: string; reason: string }
  /** No runtime reachable at all. */
  | { kind: "unavailable"; reason: string };

/**
 * Run one tool. `catalogue` is the fetched `/api/tools` row list; a row
 * without `executor_bound` is declined client-side so the UI never asks the
 * backend to run what the catalogue already says fails closed.
 */
export async function runTool(
  id: string,
  params: unknown,
  catalogue: ToolRow[],
): Promise<ToolRunState> {
  const row = catalogue.find((t) => t.id === id);
  if (row && !row.executor_bound) {
    return {
      kind: "declined",
      id,
      reason: "the catalogue lists no executor for this id (fails closed at dispatch)",
    };
  }
  if (!params || typeof params !== "object" || Array.isArray(params)) {
    return { kind: "declined", id, reason: "arguments must be a JSON object" };
  }

  const env = detectEnvironment();
  if (env === "TAURI_DESKTOP") {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const data = await invoke<ToolRunResult>("execute_tool", {
        id,
        params: params as Record<string, unknown>,
      });
      if (data && data.success === false) {
        return { kind: "failed", id, via: "desktop", error: describeFailure(data) };
      }
      return { kind: "done", result: data, via: "desktop" };
    } catch (e) {
      recordDiagnostic("toolRunner", String(e));
      // The engine rejects unbound ids by erroring; show that honestly.
      return { kind: "failed", id, via: "desktop", error: String(e) };
    }
  }

  try {
    const response = await fetch("/api/execute-tool", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ id, params: params as Record<string, unknown> }),
    });
    const data = (await response.json()) as Record<string, unknown>;
    if (data.error) {
      return { kind: "failed", id, via: "service", error: String(data.error) };
    }
    if (data && data.success === false) {
      return { kind: "failed", id, via: "service", error: describeFailure(data) };
    }
    return { kind: "done", result: data as unknown as ToolRunResult, via: "service" };
  } catch (e) {
    recordDiagnostic("toolRunner", String(e));
    return {
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to execute tools from the browser",
    };
  }
}

/** Pull the engine's own failure message out of a payload, never invent one. */
function describeFailure(data: Record<string, unknown>): string {
  const output = data.output;
  if (output && typeof output === "object") {
    const err = (output as Record<string, unknown>).error;
    if (typeof err === "string" && err.trim()) return err;
  }
  const bound =
    typeof data.bound_operation === "string" ? ` (operation: ${data.bound_operation})` : "";
  return `the tool reported failure${bound}`;
}
