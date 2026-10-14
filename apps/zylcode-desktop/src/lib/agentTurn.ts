// ---------------------------------------------------------------------------
// Agent-turn service (lib/agentTurn.ts)
//
// Drives the deterministic agent journey end to end: one user prompt →
// context gathering (real index + navigation citations) → planning → tool
// execution → verification → evidence, via the engine's `process_intent`
// pipeline.
//
// Two transports, mirroring `toolRunner.ts`:
//   * Tauri desktop  -> `process_intent` IPC command
//   * Browser preview -> POST /api/intent via the Vite proxy, served by
//                       `zylcode serve-intel` (same engine pipeline)
//
// Honesty rules: a failed turn renders the engine's failure verbatim —
// the loop reports `success: false` when verification could not complete,
// and this surface never reshapes that into a success.
// ---------------------------------------------------------------------------

import { detectEnvironment } from "./runtime";

/** Engine payload (zylcode_core::IntentResult serialization). */
export type IntentResult = {
  summary: string;
  artifacts: { kind: string; label: string; content: string }[];
  success: boolean;
};

export type AgentTurnState =
  | { kind: "idle" }
  | { kind: "running" }
  | { kind: "done"; result: IntentResult; via: "desktop" | "service" }
  | { kind: "failed"; error: string }
  | { kind: "unavailable"; reason: string };

export async function runAgentTurn(prompt: string): Promise<AgentTurnState> {
  const env = detectEnvironment();
  if (env === "TEST") {
    return { kind: "unavailable", reason: "agent turn requires a runtime" };
  }
  if (env === "TAURI_DESKTOP") {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<IntentResult>("process_intent", {
        prompt,
        model: null,
      });
      return { kind: "done", result, via: "desktop" };
    } catch (e) {
      return { kind: "failed", error: String(e) };
    }
  }
  if (env === "BROWSER_PREVIEW" || env === "UNKNOWN") {
    try {
      const response = await fetch("/api/intent", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ prompt }),
      });
      if (!response.ok) {
        return {
          kind: "unavailable",
          reason: `agent service error: HTTP ${response.status}`,
        };
      }
      const body = (await response.json()) as IntentResult & {
        error?: string;
      };
      if (body.error) {
        return { kind: "failed", error: body.error };
      }
      return { kind: "done", result: body, via: "service" };
    } catch (e) {
      return {
        kind: "unavailable",
        reason: `agent service unreachable: ${String(e)}`,
      };
    }
  }
  return { kind: "unavailable", reason: `unsupported environment: ${env}` };
}
