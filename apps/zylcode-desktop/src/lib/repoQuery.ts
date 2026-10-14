// ---------------------------------------------------------------------------
// Repository query service (lib/repoQuery.ts)
//
// The 12 `RepoQuery` methods (find_symbol, dependencies_of, entry_points…)
// exposed to the product: the same dispatcher `serve-intel` serves at
// `GET /api/repo-query`, reached on desktop through the `repo_query` Tauri
// command. One engine, one payload shape, two transports.
//
// Honesty rules: a method that could not run resolves to `unavailable`; an
// engine rejection (unknown method, missing selector) resolves to `rejected`
// with the engine's own message. Neither is ever rendered as "no results".
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic } from "./runtime";

/** The query methods the engine dispatches, in catalogue order. */
export const REPO_QUERY_METHODS = [
  "find_symbol",
  "symbol_definition",
  "symbols_in_file",
  "package_for_file",
  "dependencies_of",
  "dependents_of",
  "entry_points",
  "test_targets",
  "build_commands",
  "recent_changes",
  "architecture",
  "relevant_context",
] as const;

export type RepoQueryMethod = (typeof REPO_QUERY_METHODS)[number];

/** Methods that need a selector in the `arg` input. */
export const REPO_QUERY_METHODS_NEEDING_ARG: RepoQueryMethod[] = [
  "find_symbol",
  "symbol_definition",
  "symbols_in_file",
  "package_for_file",
  "dependencies_of",
  "dependents_of",
  "recent_changes",
];

export type RepoQueryPayload = {
  method: string;
  arg: string | null;
  elapsed_ms: number;
  // Results are engine-shaped and method-specific; the panel renders them
  // generically and never re-shapes them into something they are not.
  results: unknown;
};

export type RepoQueryState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "ready"; data: RepoQueryPayload; via: "desktop" | "service" }
  | { kind: "rejected"; reason: string }
  | { kind: "unavailable"; reason: string };

/** True when the method can run without an `arg` (stateless methods). */
export function queryNeedsArg(method: RepoQueryMethod): boolean {
  return REPO_QUERY_METHODS_NEEDING_ARG.includes(method);
}

async function fetchFromService(
  method: RepoQueryMethod,
  arg: string,
): Promise<RepoQueryPayload> {
  const params = new URLSearchParams({ method });
  if (arg.trim()) params.set("arg", arg.trim());
  const response = await fetch(`/api/repo-query?${params.toString()}`, {
    headers: { Accept: "application/json" },
  });
  const payload = await response.json();
  if (!response.ok || (payload && typeof payload === "object" && "error" in payload)) {
    throw new Error(
      String(payload && typeof payload === "object" && "error" in payload ? payload.error : `HTTP ${response.status}`),
    );
  }
  return payload as RepoQueryPayload;
}

/** Run one repository query. Desktop first; browser preview via intel service. */
export async function runRepoQuery(
  method: RepoQueryMethod,
  arg: string,
): Promise<RepoQueryState> {
  if (queryNeedsArg(method) && !arg.trim()) {
    return { kind: "rejected", reason: `the '${method}' query needs a selector` };
  }
  const env = detectEnvironment();

  if (env === "TAURI_DESKTOP") {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const data = await invoke<RepoQueryPayload>("repo_query", {
        method,
        arg: arg.trim() || null,
      });
      if (data && typeof data === "object" && "error" in data) {
        return { kind: "rejected", reason: String((data as { error: unknown }).error) };
      }
      return { kind: "ready", data, via: "desktop" };
    } catch (e) {
      recordDiagnostic("repoQuery", String(e));
      return {
        kind: "unavailable",
        reason: "desktop engine could not run the repository query",
      };
    }
  }

  try {
    const data = await fetchFromService(method, arg);
    return { kind: "ready", data, via: "service" };
  } catch (e) {
    recordDiagnostic("repoQuery", String(e));
    return {
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to run repository queries",
    };
  }
}

/** Uniform, ordered JSON rendering of any engine result payload. */
export function renderResultLines(results: unknown): { key: string; text: string }[] {
  const odd = (v: unknown): v is Record<string, unknown> =>
    typeof v === "object" && v !== null;
  const scalar = (v: unknown): string => (typeof v === "string" ? v : JSON.stringify(v));
  const row = (label: string, v: unknown, i: number): { key: string; text: string } => ({
    key: `${i}:${label}`,
    text: `${label} ${scalar(v)}`,
  });

  if (Array.isArray(results)) {
    return results.slice(0, 40).flatMap((v, i) => {
      if (odd(v)) {
        const label =
          (v.resource as string) ??
          (v.name as string) ??
          (v.id as string) ??
          (v.short_id as string) ??
          (v.command as string) ??
          (v.fact === undefined ? "" : `${v.fact}:`);
        const tail =
          label !== undefined
            ? " ".repeat(0) + restOf(v, label)
            : Object.entries(v)
                .slice(0, 4)
                .map(([k, x]) => `${k}=${scalar(x)}`)
                .join("  ");
        return [{ key: String(i), text: `${label === undefined ? "" : label} ${tail}`.trim() }];
      }
      return [row("#", v, i)];
    });
  }
  if (results === null || results === undefined) {
    return [{ key: "none", text: "(no result — the engine returned null)" }];
  }
  return [{ key: "single", text: scalar(results) }];

  function restOf(v: Record<string, unknown>, label: string): string {
    const parts: string[] = [];
    for (const [k, x] of Object.entries(v)) {
      if (typeof x === "string" && x === label) continue;
      if (["resource", "name", "id", "short_id", "command"].includes(k)) continue;
      parts.push(`${k}=${scalar(x)}`);
      if (parts.length >= 4) break;
    }
    return parts.join("  ");
  }
}
