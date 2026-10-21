// ---------------------------------------------------------------------------
// Repository navigation service (lib/navIntel.ts)
//
// One typed data path with two transports, mirroring `repoIntel.ts`:
//   * Tauri desktop  -> `nav_query` IPC command (same engine process).
//   * Browser preview -> POST /api/nav via the Vite proxy, served by
//     `zylcode serve-intel`.
//
// Both transports reach `zylcode-nav`, so the References panel, the call
// hierarchy, the dependency graph and the impact view all read the *same*
// index the MCP tools read. There is no second, text-search-backed
// implementation hiding behind the UI.
//
// Honesty rules (Repository Intelligence Wave §4):
//   * every relationship carries `evidence`; heuristic rows are shown
//     alongside deterministic ones and never promoted to them;
//   * an unresolvable selector arrives as `unresolved` inside a successful
//     payload — the UI renders that message rather than an empty "no
//     results" that would read as a fact;
//   * a query that could not *run* resolves to a controlled state
//     (`rejected` / `unavailable`); an engine error is never rendered as a
//     navigation answer.
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic } from "./runtime";

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

/** The navigation tools this client can call, in catalogue order. */
export const NAV_TOOL_IDS = [
  "find_definition",
  "find_references",
  "find_callers",
  "find_callees",
  "file_dependencies",
  "file_dependents",
  "symbol_impact",
  "repository_graph_query",
] as const;

export type NavTool = (typeof NAV_TOOL_IDS)[number];

/** Arguments are passed through to the engine mostly verbatim. */
export type NavArgs = Record<string, unknown>;

export type NavTransport = "desktop" | "service";

/**
 * Outcome of a navigation query.
 *
 * `rejected` and `unavailable` are deliberately distinct: the first means the
 * *engine* refused the request (a missing selector — the caller can fix it),
 * the second means there was no engine to ask (nothing was answered at all).
 * Neither is ever converted into an empty result set, because "no references
 * found" and "the index could not be built" are different facts.
 */
export type NavState<T> =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "ready"; data: T; via: NavTransport }
  | { kind: "rejected"; reason: string }
  | { kind: "unavailable"; reason: string };

// ---------------------------------------------------------------------------
// Engine vocabulary (kept structurally aligned with zylcode-nav's model.rs)
// ---------------------------------------------------------------------------

export type Evidence = "DETERMINISTIC" | "HEURISTIC" | "UNSUPPORTED";

export type ImpactClass =
  "DIRECT_DEPENDENT" | "TRANSITIVE_DEPENDENT" | "POSSIBLE_TEXTUAL_REFERENCE" | "UNRESOLVED";

export type EdgeKind = "imports" | "calls" | "depends_on" | "contains" | "exports" | "references";

export type NodeKind = "file" | "symbol" | "package";

export type ImpactTargetKind = "file" | "symbol" | "package";

export type TextRange = {
  start_byte: number;
  end_byte: number;
  start_line: number;
  start_col: number;
  end_line: number;
  end_col: number;
};

export type NavSymbol = {
  id: string;
  file: string;
  qualified_name: string;
  name: string;
  kind: string;
  range: TextRange;
  name_range: TextRange;
  container: string | null;
  signature: string | null;
  exported: boolean;
  module: string;
};

export type NavReference = {
  file: string;
  range: TextRange;
  kind: string;
  evidence: Evidence;
  excerpt: string;
  via: string;
};

export type NavDefinitionResult = {
  engine: string;
  definition: NavSymbol | null;
  candidates: NavSymbol[];
  unresolved: string | null;
};

export type NavReferenceResult = {
  engine: string;
  definition: NavSymbol | null;
  references: NavReference[];
  possible: NavReference[];
  unresolved: string | null;
  deterministic_count: number;
  heuristic_count: number;
};

export type NavCallSite = {
  file: string;
  range: TextRange;
  callee_range: TextRange;
  callee_name: string;
  callee_path: string | null;
  shape: string;
  enclosing: string | null;
  excerpt: string;
};

export type NavCallRelation = {
  site: NavCallSite;
  resolved: NavSymbol | null;
  evidence: Evidence;
  note: string;
};

export type NavCallHierarchyResult = {
  engine: string;
  target: NavSymbol | null;
  relations: NavCallRelation[];
  unresolved: string | null;
  deterministic_count: number;
  heuristic_count: number;
  unsupported_count: number;
};

export type NavFileDependency = {
  file: string;
  kind: string;
  specifier: string;
  resolution: string;
  range: TextRange;
  excerpt: string;
  evidence: Evidence;
  /** Human-readable edge, e.g. `src/a.ts --imports--> src/b.ts`. */
  detail: string;
};

export type NavFileDependenciesResult = {
  engine: string;
  file: string;
  dependencies: NavFileDependency[];
  resolved_count: number;
  external_count: number;
  unresolved_count: number;
};

export type NavImpactItem = {
  class: ImpactClass;
  target_kind: ImpactTargetKind;
  id: string;
  label: string;
  evidence: Evidence;
  provenance: string;
  /** Chain of edges establishing this item, root-first. */
  via: string[];
  excerpt?: string | null;
};

export type NavImpactResult = {
  engine: string;
  subject: { kind: string } & Record<string, string>;
  items: NavImpactItem[];
  direct_count: number;
  transitive_count: number;
  possible_count: number;
  unresolved_count: number;
  /** Plain-language scope statement plus the non-prediction caveat. */
  summary: string;
  unresolved_subject: string | null;
};

export type NavGraphNode = {
  kind: NodeKind;
  id: string;
  label: string;
  language?: string | null;
};

export type NavGraphEdge = {
  from: NavGraphNode;
  to: NavGraphNode;
  kind: EdgeKind;
  provenance: string;
  evidence: Evidence;
  /** Human-readable edge, e.g. `foo() --calls--> bar()`. */
  detail: string;
  source_file?: string | null;
  range?: TextRange | null;
  excerpt?: string | null;
};

export type NavGraphResult = {
  engine: string;
  nodes: NavGraphNode[];
  edges: NavGraphEdge[];
  cycles: string[][];
  truncated: boolean;
  total_edges_matched: number;
};

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

/**
 * Classify a failure message coming back from either transport.
 *
 * The engine's own vocabulary is the discriminator: `NavError::Rejected`
 * renders as "request rejected: …" and `NavError::Unavailable` as
 * "repository index unavailable: …". Matching on it — rather than on the
 * transport-specific wrapper — keeps the classification correct for the
 * HTTP body, the IPC error string and any future transport at once.
 */
export function classifyNavFailure(message: string): "rejected" | "unavailable" {
  return message.includes("request rejected") ? "rejected" : "unavailable";
}

const REJECTED_MESSAGE = "the repository could not answer this query with the given selector";

async function fetchFromService<T>(tool: NavTool, args: NavArgs): Promise<T> {
  const response = await fetch("/api/nav", {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({ tool, args }),
  });
  if (!response.ok) {
    throw new Error(`nav service returned HTTP ${response.status}`);
  }
  const payload = await response.json();
  if (payload && typeof payload === "object" && "error" in payload) {
    // The service reports engine failures as a 200 body with an `error`
    // key; that is a refusal, not a navigation answer.
    const reason = String((payload as { error?: unknown }).error ?? "");
    throw new EngineRejection(reason);
  }
  return payload as T;
}

/** Raised when the engine itself refused the request. */
class EngineRejection extends Error {}

/**
 * Run one navigation query.
 *
 * Failures never resolve to `ready`: an engine refusal becomes
 * `rejected`, an unreachable engine becomes `unavailable`, and the raw
 * message is recorded to Diagnostics rather than shown as if it were a
 * result.
 */
export async function queryNav<T>(tool: NavTool, args: NavArgs = {}): Promise<NavState<T>> {
  const env = detectEnvironment();

  if (env === "TAURI_DESKTOP") {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const data = await invoke<T>("nav_query", { tool, args });
      return { kind: "ready", data, via: "desktop" };
    } catch (e) {
      const message = String(e);
      recordDiagnostic("navIntel", message);
      if (classifyNavFailure(message) === "rejected") {
        return { kind: "rejected", reason: REJECTED_MESSAGE };
      }
      return {
        kind: "unavailable",
        reason: "the repository index could not be built for this workspace",
      };
    }
  }

  try {
    const data = await fetchFromService<T>(tool, args);
    return { kind: "ready", data, via: "service" };
  } catch (e) {
    recordDiagnostic("navIntel", String(e));
    if (e instanceof EngineRejection) {
      return { kind: "rejected", reason: REJECTED_MESSAGE };
    }
    return {
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to query repository navigation",
    };
  }
}

// ---------------------------------------------------------------------------
// Presentation helpers (pure — unit-tested so the panel cannot drift)
// ---------------------------------------------------------------------------

/** `L12:C5` — what a human clicks on. */
export function formatLocation(range: TextRange): string {
  return `L${range.start_line}:C${range.start_col}`;
}

/** Case-insensitive substring test used by every filter box in the panel. */
export function matchesQuery(haystack: string, needle: string): boolean {
  const query = needle.trim().toLowerCase();
  if (query === "") return true;
  return haystack.toLowerCase().includes(query);
}

/**
 * Deterministic rows first, then heuristic, then unsupported — so a large
 * result list always leads with the strongest evidence instead of burying it
 * behind a text-search hit.
 */
const EVIDENCE_RANK: Record<Evidence, number> = {
  DETERMINISTIC: 0,
  HEURISTIC: 1,
  UNSUPPORTED: 2,
};

export function byEvidenceThen<T extends { evidence: Evidence }>(a: T, b: T): number {
  return EVIDENCE_RANK[a.evidence] - EVIDENCE_RANK[b.evidence];
}

/**
 * Keep rows that survive the text filter and the evidence filter.
 *
 * `all` keeps everything; the narrower settings exist so a reviewer can see
 * *only* the deterministic claim, or audit *only* the guesses.
 */
export function filterByEvidence<T extends { evidence: Evidence }>(
  rows: readonly T[],
  evidence: Evidence | "all",
): T[] {
  if (evidence === "all") return [...rows];
  return rows.filter((row) => row.evidence === evidence);
}

/** Group rows by their repository-relative file, preserving first-seen order. */
export function groupByFile<T extends { file: string }>(
  rows: readonly T[],
): { file: string; rows: T[] }[] {
  const order: string[] = [];
  const buckets = new Map<string, T[]>();
  for (const row of rows) {
    const bucket = buckets.get(row.file);
    if (bucket) {
      bucket.push(row);
    } else {
      buckets.set(row.file, [row]);
      order.push(row.file);
    }
  }
  return order.map((file) => ({ file, rows: buckets.get(file) ?? [] }));
}
