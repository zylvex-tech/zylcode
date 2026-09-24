// ---------------------------------------------------------------------------
// Search / file tree / evidence ledger services (lib/surfaces.ts)
//
// Transport parity with lib/repoIntel.ts and lib/gitStatus.ts: Tauri desktop
// invokes the engine commands; the browser preview fetches through the Vite
// proxy to `zylcode serve-intel`. Failed fetches resolve to controlled
// unavailable states — the UI never invents data.
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic } from "./runtime";

export type SearchResult = {
  query: string;
  elapsed_ms: number;
  indexed_files: number;
  results: {
    resource: string;
    resource_type: string;
    relevance: number;
    reason: string;
  }[];
};

export type FileTreeResult = {
  total_scanned: number;
  truncated: boolean;
  elapsed_ms: number;
  rows: { path: string; language: string }[];
};

export type EvidenceState =
  | { kind: "loading" }
  | { kind: "unavailable"; reason: string }
  | {
      kind: "empty";
      reason: string;
    }
  | {
      kind: "ready";
      chain_intact: boolean;
      session_count: number;
      entry_count: number;
      entries: {
        id: string;
        session_id: string;
        action_id: string;
        state: string;
        timestamp: string;
        error: string | null;
        payload: unknown;
      }[];
    };

async function getJson<T>(url: string): Promise<T> {
  const response = await fetch(url, { headers: { Accept: "application/json" } });
  if (!response.ok) {
    throw new Error(`service returned HTTP ${response.status}`);
  }
  const payload = await response.json();
  if (payload && typeof payload === "object" && "error" in payload) {
    throw new Error(String(payload.error));
  }
  return payload as T;
}

async function viaDesktop<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  source: string,
  fallbackReason: string,
): Promise<T | { __unavailable: string }> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(command, args);
  } catch (e) {
    recordDiagnostic(source, String(e));
    return { __unavailable: fallbackReason };
  }
}

const UNAVAILABLE = "__unavailable" as const;

function isUnavailable<T>(v: T | { __unavailable: string }): v is { __unavailable: string } {
  return typeof v === "object" && v !== null && UNAVAILABLE in v;
}

/** Live search over the intelligence pipeline. */
export async function fetchSearch(
  q: string,
): Promise<{ kind: "loading" } | { kind: "unavailable"; reason: string } | { kind: "ready"; data: SearchResult }> {
  const env = detectEnvironment();
  if (env === "TAURI_DESKTOP") {
    const result = await viaDesktop<SearchResult>("repo_search", { q }, "search", "desktop engine search failed");
    if (isUnavailable(result)) return { kind: "unavailable", reason: result.__unavailable };
    return { kind: "ready", data: result };
  }
  try {
    const data = await getJson<SearchResult>(`/api/search?q=${encodeURIComponent(q)}`);
    return { kind: "ready", data };
  } catch (e) {
    recordDiagnostic("search", String(e));
    return { kind: "unavailable", reason: "start `zylcode serve-intel` to search the repository" };
  }
}

/** The scanner's file tree for Explorer. */
export async function fetchFileTree(): Promise<
  { kind: "loading" } | { kind: "unavailable"; reason: string } | { kind: "ready"; data: FileTreeResult }
> {
  const env = detectEnvironment();
  if (env === "TAURI_DESKTOP") {
    const result = await viaDesktop<FileTreeResult>("repo_file_tree", undefined, "fileTree", "desktop engine file tree failed");
    if (isUnavailable(result)) return { kind: "unavailable", reason: result.__unavailable };
    return { kind: "ready", data: result };
  }
  try {
    const data = await getJson<FileTreeResult>("/api/files");
    return { kind: "ready", data };
  } catch (e) {
    recordDiagnostic("fileTree", String(e));
    return { kind: "unavailable", reason: "start `zylcode serve-intel` to browse the repository tree" };
  }
}

/** The evidence ledger timeline (empty/ready/error states are honest). */
export async function fetchEvidence(): Promise<EvidenceState> {
  const env = detectEnvironment();
  if (env === "TAURI_DESKTOP") {
    const result = await viaDesktop<{ kind: string } & Record<string, unknown>>(
      "evidence_ledger",
      undefined,
      "evidence",
      "desktop engine could not read the evidence ledger",
    );
    if (isUnavailable(result)) return { kind: "unavailable", reason: result.__unavailable };
    return result as EvidenceState;
  }
  try {
    return await getJson<EvidenceState>("/api/evidence");
  } catch (e) {
    recordDiagnostic("evidence", String(e));
    return {
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to read the evidence ledger",
    };
  }
}
