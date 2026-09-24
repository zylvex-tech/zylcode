// ---------------------------------------------------------------------------
// Git source-control service (lib/gitStatus.ts)
//
// Transport parity with lib/repoIntel.ts: Tauri desktop uses the `git_status`
// IPC command; the browser preview fetches /api/git/status through the Vite
// proxy. Failed fetches resolve to a controlled unavailable state — the UI
// never invents a clean (or dirty) tree.
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic } from "./runtime";

export type GitStatusResult = {
  branch: string;
  upstream: string | null;
  ahead: number;
  behind: number;
  head: string;
  clean: boolean;
  counts: {
    total: number;
    staged: number;
    unstaged: number;
    untracked: number;
  };
  entries: { status: string; path: string }[];
  diffstat: {
    files_changed: number;
    insertions: number;
    deletions: number;
    rows: { path: string; added: number; removed: number }[];
  };
};

export type GitStatusState =
  | { kind: "loading" }
  | { kind: "ready"; data: GitStatusResult; via: "desktop" | "service" }
  | { kind: "unavailable"; reason: string };

async function fetchFromService(): Promise<GitStatusResult> {
  const response = await fetch("/api/git/status", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    throw new Error(`git status service returned HTTP ${response.status}`);
  }
  const payload = await response.json();
  if (payload && typeof payload === "object" && "error" in payload) {
    throw new Error(String(payload.error));
  }
  return payload as GitStatusResult;
}

/** Fetch the git source-control state. Desktop first; browser via service. */
export async function fetchGitStatus(): Promise<GitStatusState> {
  const env = detectEnvironment();

  if (env === "TAURI_DESKTOP") {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const data = await invoke<GitStatusResult>("git_status");
      return { kind: "ready", data, via: "desktop" };
    } catch (e) {
      recordDiagnostic("gitStatus", String(e));
      return { kind: "unavailable", reason: "desktop engine could not read the git state" };
    }
  }

  try {
    const data = await fetchFromService();
    return { kind: "ready", data, via: "service" };
  } catch (e) {
    recordDiagnostic("gitStatus", String(e));
    return {
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to see live source-control data",
    };
  }
}
