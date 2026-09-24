// ---------------------------------------------------------------------------
// Repository Intelligence service (lib/repoIntel.ts)
//
// One typed data path with two transports:
//   * Tauri desktop  -> `repo_context` IPC command (same engine process).
//   * Browser preview -> GET /api/repo-intel via the Vite proxy, served by
//     `zylcode serve-intel` running the real intelligence pipeline.
//
// Honesty rules (Product Architecture v3 §11): a failed fetch resolves to a
// controlled `unavailable` state — the UI never invents data and never
// renders a raw exception.
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic } from "./runtime";

export type RepoIntelResult = {
  task: string;
  elapsed_ms: number;
  summary: {
    file_count: number;
    symbol_count: number;
    package_count: number;
    packages: string[];
    languages: string[];
    entry_points: string[];
    architecture_facts: { fact: string; value: string }[];
    recent_changes: { short_id: string; message: string; author: string }[];
  };
  results: {
    resource: string;
    resource_type: string;
    relevance: number;
    reason: string;
  }[];
};

export type RepoIntelState =
  | { kind: "loading" }
  | { kind: "ready"; data: RepoIntelResult; via: "desktop" | "service" }
  | { kind: "unavailable"; reason: string };

async function fetchFromService(task: string): Promise<RepoIntelResult> {
  const url = `/api/repo-intel?task=${encodeURIComponent(task)}`;
  const response = await fetch(url, { headers: { Accept: "application/json" } });
  if (!response.ok) {
    throw new Error(`repo-intel service returned HTTP ${response.status}`);
  }
  const payload = await response.json();
  if (payload && typeof payload === "object" && "error" in payload) {
    throw new Error(String(payload.error));
  }
  return payload as RepoIntelResult;
}

/**
 * Fetch the Repository Intelligence payload.
 * Desktop first (same process as the engine); browser preview falls back
 * to the local intel service through the Vite proxy.
 */
export async function fetchRepoIntel(task: string): Promise<RepoIntelState> {
  const env = detectEnvironment();

  if (env === "TAURI_DESKTOP") {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const data = await invoke<RepoIntelResult>("repo_context", { task });
      return { kind: "ready", data, via: "desktop" };
    } catch (e) {
      recordDiagnostic("repoIntel", String(e));
      return { kind: "unavailable", reason: "desktop engine could not build the intelligence payload" };
    }
  }

  try {
    const data = await fetchFromService(task);
    return { kind: "ready", data, via: "service" };
  } catch (e) {
    recordDiagnostic("repoIntel", String(e));
    return {
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to see live repository data",
    };
  }
}
