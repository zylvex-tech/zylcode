import {
  Component,
  useCallback,
  useMemo,
  useRef,
  useState,
  type ErrorInfo,
  type ReactNode,
} from "react";
import { Panel } from "./Panel";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import { recordDiagnostic } from "../lib/runtime";
import {
  byEvidenceThen,
  filterByEvidence,
  formatLocation,
  groupByFile,
  matchesQuery,
  queryNav,
  type Evidence,
  type EdgeKind,
  type ImpactClass,
  type NavArgs,
  type NavCallHierarchyResult,
  type NavDefinitionResult,
  type NavFileDependenciesResult,
  type NavGraphResult,
  type NavImpactResult,
  type NavReferenceResult,
  type NavState,
  type NavSymbol,
  type NavTool,
  type NavTransport,
} from "../lib/navIntel";

// ---------------------------------------------------------------------------
// References · Call hierarchy · Dependency graph · Impact
//
// Every row on this surface is rendered from a `zylcode-nav` payload. The
// panel never re-derives a relationship itself and never upgrades an
// HEURISTIC row to look like a DETERMINISTIC one — the evidence badge is
// the engine's label, shown verbatim.
// ---------------------------------------------------------------------------

const TABS = [
  { id: "references", label: "References" },
  { id: "calls", label: "Call hierarchy" },
  { id: "graph", label: "Dependency graph" },
  { id: "impact", label: "Impact" },
] as const;

type NavTab = (typeof TABS)[number]["id"];

// ---------------------------------------------------------------------------
// Answer ownership (G11)
//
// The defect this replaces: the panel kept one untyped `NavState<unknown>` and
// cast `state.data` into whichever view the *active* tab selected. A response
// produced for one query mode could therefore be rendered under another — the
// classic "request A consumed as a mode B result" race.
//
// Two things now make that impossible:
//   * `run()` claims a monotonically increasing request id, and only the
//     newest id may write state — a stale answer is dropped, not re-shaped;
//   * `ReadyNavState` stores `tab` and `data` in *one* discriminated member,
//     so the payload's type is derived from narrowing on its owner rather
//     than from a cast the compiler never checked.
// ---------------------------------------------------------------------------

/** Which payload each view is entitled to render — one identity per mode. */
export type NavTabPayload = {
  references: NavReferenceResult;
  calls: NavCallHierarchyResult;
  graph: NavGraphResult;
  impact: NavImpactResult;
};

/**
 * A resolved answer and the view it belongs to, held together.
 *
 * There is deliberately no way to construct a `ready` state without naming
 * the tab that asked for it, and no way to reach `data` typed as
 * `NavImpactResult` without having narrowed on `tab === "impact"` first.
 */
export type ReadyNavState =
  | { kind: "ready"; tab: "references"; data: NavReferenceResult; via: NavTransport }
  | { kind: "ready"; tab: "calls"; data: NavCallHierarchyResult; via: NavTransport }
  | { kind: "ready"; tab: "graph"; data: NavGraphResult; via: NavTransport }
  | { kind: "ready"; tab: "impact"; data: NavImpactResult; via: NavTransport };

/** The states that carry no payload at all: idle / loading / rejected / unavailable. */
type AnswerlessNavState = Exclude<NavState<never>, { kind: "ready" }>;

/** Everything this panel can hold. `data` only ever exists beside its owner. */
export type RepoNavState = AnswerlessNavState | ReadyNavState;

const EVIDENCE_STYLE: Record<Evidence, string> = {
  DETERMINISTIC: "bg-success/15 text-success border-success/30",
  HEURISTIC: "bg-warning/15 text-warning border-warning/30",
  UNSUPPORTED: "bg-text-muted/10 text-text-muted border-text-muted/30",
};

const EVIDENCE_HINT: Record<Evidence, string> = {
  DETERMINISTIC: "resolved from the parsed index",
  HEURISTIC: "inferred — shown alongside the deterministic results, never instead of them",
  UNSUPPORTED: "the grammar gave no name to attribute this to",
};

const IMPACT_ORDER: ImpactClass[] = [
  "DIRECT_DEPENDENT",
  "TRANSITIVE_DEPENDENT",
  "POSSIBLE_TEXTUAL_REFERENCE",
  "UNRESOLVED",
];

const EDGE_ORDER: EdgeKind[] = [
  "imports",
  "calls",
  "depends_on",
  "exports",
  "references",
  "contains",
];

const PAGE = 60;

function EvidenceBadge({ evidence }: { evidence: Evidence }) {
  return (
    <span
      title={EVIDENCE_HINT[evidence]}
      className={`shrink-0 rounded border px-1 py-px font-mono text-[9px] tracking-wide ${EVIDENCE_STYLE[evidence]}`}
    >
      {evidence}
    </span>
  );
}

function DefinitionLine({ symbol }: { symbol: NavSymbol }) {
  return (
    <p className="truncate font-mono text-[11px] text-text">
      <span className="text-text-muted">{symbol.kind}</span> {symbol.qualified_name}
      {symbol.signature && <span className="text-text-muted"> · {symbol.signature}</span>}
    </p>
  );
}

/**
 * Shared shell for idle / loading / rejected / unavailable.
 *
 * `rejected` and `unavailable` render as *themselves* — never as an empty
 * result list, because "nothing references this" and "we could not ask" are
 * different claims and only the first one is evidence.
 */
function QueryStatus({ state }: { state: NavState<unknown> }) {
  if (state.kind === "idle") {
    return (
      <p className="text-[11px] text-text-muted">
        Choose a symbol (or a file + line) and run a query — answers come from the parsed repository
        index, not from text search.
      </p>
    );
  }
  if (state.kind === "loading") {
    return <p className="text-[11px] text-text-muted">Querying the repository index…</p>;
  }
  if (state.kind === "rejected") {
    return (
      <p className="text-[11px] text-warning">The engine refused this query — {state.reason}.</p>
    );
  }
  if (state.kind === "unavailable") {
    return <p className="text-[11px] text-error">{state.reason}</p>;
  }
  return (
    <p className="text-[10px] font-mono text-text-muted">
      answer returned by zylcode-nav · via{" "}
      {state.via === "desktop" ? "desktop engine" : "intel service"}
    </p>
  );
}

function EmptyFor({ message }: { message: string }) {
  return <p className="text-[11px] text-text-muted">{message}</p>;
}

// ---------------------------------------------------------------------------
// Payload ⇄ mode ownership checks (G11)
//
// Structural, not cosmetic: these are what stops an engine answer shaped for
// one mode being accepted as an answer for another, on top of (never instead
// of) the request-id check in `run()`.
// ---------------------------------------------------------------------------

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isNavReferenceResult(data: unknown): data is NavReferenceResult {
  return isRecord(data) && Array.isArray(data.references) && Array.isArray(data.possible);
}

function isNavCallHierarchyResult(data: unknown): data is NavCallHierarchyResult {
  return isRecord(data) && Array.isArray(data.relations);
}

function isNavGraphResult(data: unknown): data is NavGraphResult {
  return isRecord(data) && Array.isArray(data.nodes) && Array.isArray(data.edges);
}

function isNavImpactResult(data: unknown): data is NavImpactResult {
  return isRecord(data) && Array.isArray(data.items);
}

/**
 * Adopt an engine payload as the answer that owns `tab`.
 *
 * `null` means "this payload does not answer `tab`" — the caller reports a
 * controlled failure instead of rendering it. Every `NavTabPayload` field is
 * always present in the engine's serialization (plain `Serialize` derives, no
 * `skip_serializing_if`), so an unresolved subject still arrives with its
 * empty `items` / `references` arrays and is never mistaken for a foreign
 * payload.
 */
function readyStateFor(tab: NavTab, data: unknown, via: NavTransport): ReadyNavState | null {
  switch (tab) {
    case "references":
      return isNavReferenceResult(data) ? { kind: "ready", tab, data, via } : null;
    case "calls":
      return isNavCallHierarchyResult(data) ? { kind: "ready", tab, data, via } : null;
    case "graph":
      return isNavGraphResult(data) ? { kind: "ready", tab, data, via } : null;
    case "impact":
      return isNavImpactResult(data) ? { kind: "ready", tab, data, via } : null;
    default:
      return null;
  }
}

// ---------------------------------------------------------------------------

export interface RepoNavPanelProps {
  /** Click-to-navigate: opens the file in the editor. */
  onNavigateToFile: (path: string) => void;
}

export function RepoNavPanel({ onNavigateToFile }: RepoNavPanelProps) {
  const [tab, setTab] = useState<NavTab>("references");
  const [state, setState] = useState<RepoNavState>({ kind: "idle" });
  const [hasRun, setHasRun] = useState(false);

  // Request ownership (G11). Each `run()` takes the next id; only the newest
  // id may write. An answer that arrives after a newer request has started is
  // stale: it is discarded whole, never re-shaped into the answer for whatever
  // view is now active.
  const requestRef = useRef(0);

  // Selector inputs.
  const [symbol, setSymbol] = useState("");
  const [file, setFile] = useState("");
  const [line, setLine] = useState("");

  // Scope / filter inputs.
  const [callMode, setCallMode] = useState<"callers" | "callees">("callers");
  const [depth, setDepth] = useState(2);
  const [focus, setFocus] = useState("");
  const [filterText, setFilterText] = useState("");
  const [evidenceFilter, setEvidenceFilter] = useState<Evidence | "all">("all");
  const [edgeKindFilter, setEdgeKindFilter] = useState<EdgeKind | "all">("all");
  const [visible, setVisible] = useState(PAGE);

  const busy = state.kind === "loading";

  const selectorArgs = useCallback((): NavArgs | null => {
    const f = file.trim();
    const s = symbol.trim();
    const l = line.trim();
    if (l !== "" && f !== "") {
      const parsed = Number(l);
      if (!Number.isFinite(parsed)) return null;
      return { file: f, line: parsed };
    }
    if (s === "") return null;
    return f === "" ? { name: s } : { name: s, file: f };
  }, [file, symbol, line]);

  const run = useCallback(
    async (target: NavTab) => {
      // Claim ownership of this run before anything can await.
      const requestId = requestRef.current + 1;
      requestRef.current = requestId;
      const isCurrent = () => requestRef.current === requestId;

      const need = (reason: string): false => {
        setHasRun(true);
        setState({ kind: "rejected", reason });
        return false;
      };
      let tool: NavTool;
      let args: NavArgs = {};
      if (target === "references") {
        const sel = selectorArgs();
        if (!sel) return void need("a symbol name — or a file plus line — is required");
        tool = "find_references";
        args = { ...sel, limit: 500 };
      } else if (target === "calls") {
        const sel = selectorArgs();
        if (!sel) return void need("a symbol name — or a file plus line — is required");
        tool = callMode === "callers" ? "find_callers" : "find_callees";
        args = { ...sel, limit: 500 };
      } else if (target === "graph") {
        tool = "repository_graph_query";
        args = { depth: 2, limit: 400, include_cycles: true };
        if (focus.trim() !== "") args = { ...args, focus: focus.trim(), depth };
      } else {
        const f = file.trim();
        if (f !== "") {
          tool = "symbol_impact";
          args = { subject: "file", file: f, depth, limit: 500 };
        } else {
          const sel = selectorArgs();
          if (!sel) return void need("a symbol name — or a file plus line — is required");
          tool = "symbol_impact";
          args = { ...sel, depth, limit: 500 };
        }
      }
      setHasRun(true);
      setVisible(PAGE);
      setState({ kind: "loading" });

      const outcome = await queryNav<unknown>(tool, args);

      // Stale-response rejection. A newer request already owns the panel, so
      // this answer has no view to go to — drop it untouched. (Switching tabs
      // while a query is in flight is exactly how this id becomes stale.)
      if (!isCurrent()) return;

      if (outcome.kind !== "ready") {
        setState(outcome);
        return;
      }

      // The answer is current, but it still has to answer *this* mode.
      const adopted = readyStateFor(target, outcome.data, outcome.via);
      if (!adopted) {
        setState({
          kind: "unavailable",
          reason: `the engine answered a ${target} query with an unrelated payload; it was discarded rather than rendered`,
        });
        return;
      }
      setState(adopted);
    },
    [callMode, depth, file, focus, selectorArgs],
  );

  // Re-running on a tab switch only happens after the first deliberate run,
  // so opening the panel never triggers an index build on its own.
  const switchTab = useCallback(
    (next: NavTab) => {
      setTab(next);
      setFilterText("");
      if (hasRun) void run(next);
    },
    [hasRun, run],
  );

  // An answer counts as ready only when it belongs to the view asking for it.
  // Outside of a construction bug this always holds — `switchTab` claims a new
  // request id and clears to `loading` — so the second check is the invariant
  // the whole panel leans on, not a workaround.
  const ready = state.kind === "ready" && state.tab === tab;
  const status: CapabilityStatus = ready
    ? "AVAILABLE"
    : state.kind === "loading" || state.kind === "idle"
      ? "LIMITED"
      : "BLOCKED";

  return (
    <Panel title="CODE NAVIGATION" right={<StatusBadge status={status} size="xs" />}>
      <div className="space-y-3">
        {/* ---- selector -------------------------------------------------- */}
        <form
          className="grid grid-cols-[1fr_1.4fr_4.5rem_auto] gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            void run(tab);
          }}
        >
          <input
            className="input text-xs"
            value={symbol}
            onChange={(e) => setSymbol(e.target.value)}
            placeholder="symbol (e.g. helper)"
            aria-label="Symbol name"
          />
          <input
            className="input text-xs"
            value={file}
            onChange={(e) => setFile(e.target.value)}
            placeholder="file (e.g. src/lib.rs)"
            aria-label="Repository-relative file"
          />
          <input
            className="input text-xs"
            value={line}
            onChange={(e) => setLine(e.target.value)}
            inputMode="numeric"
            placeholder="line"
            aria-label="Line number"
          />
          <button type="submit" className="btn btn-primary text-xs" disabled={busy}>
            {busy ? "Querying…" : "Analyze"}
          </button>
        </form>
        <p className="text-[10px] text-text-muted">
          A symbol alone is scoped by module; a file + line resolves exactly what is at that
          position. Duplicate names are never collapsed into one.
        </p>

        {/* ---- tabs ------------------------------------------------------ */}
        <div className="flex border-b border-border" role="tablist">
          {TABS.map((t) => (
            <button
              key={t.id}
              role="tab"
              aria-selected={tab === t.id}
              onClick={() => switchTab(t.id)}
              className={`px-3 py-1.5 text-[11px] font-medium border-b-2 transition-colors ${
                tab === t.id
                  ? "border-primary text-primary"
                  : "border-transparent text-text-muted hover:text-text"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>

        {state.kind !== "ready" && <QueryStatus state={state} />}
        {/* Defensive: an answer for a *different* view is stated, never drawn. */}
        {state.kind === "ready" && state.tab !== tab && (
          <p className="text-[11px] text-warning">
            The answer on hand belongs to a different view, so it was not drawn here — run this
            view&apos;s query again.
          </p>
        )}

        {/* ---- scope / filter bar ---------------------------------------- */}
        {ready && (
          <div className="flex flex-wrap items-center gap-2">
            {tab === "calls" && (
              <div className="flex overflow-hidden rounded border border-border">
                {(["callers", "callees"] as const).map((m) => (
                  <button
                    key={m}
                    onClick={() => {
                      setCallMode(m);
                      void run(tab);
                    }}
                    className={`px-2 py-1 text-[10px] ${
                      callMode === m ? "bg-primary/15 text-primary" : "text-text-muted"
                    }`}
                  >
                    {m}
                  </button>
                ))}
              </div>
            )}

            {tab === "graph" && (
              <>
                <input
                  className="input text-[10px] w-56"
                  value={focus}
                  onChange={(e) => setFocus(e.target.value)}
                  placeholder="focus node (file path or id)"
                  aria-label="Graph focus node"
                />
                <label className="text-[10px] text-text-muted">
                  depth
                  <select
                    className="input ml-1 w-14 text-[10px]"
                    value={depth}
                    onChange={(e) => setDepth(Number(e.target.value))}
                    aria-label="Graph depth"
                  >
                    {[1, 2, 3].map((d) => (
                      <option key={d} value={d}>
                        {d}
                      </option>
                    ))}
                  </select>
                </label>
                <button
                  className="btn text-[10px]"
                  onClick={() => void run(tab)}
                  title="Re-run the query with the current focus and depth"
                >
                  Re-scope
                </button>
              </>
            )}

            {(tab === "impact" || tab === "calls") && (
              <label className="text-[10px] text-text-muted">
                depth
                <select
                  className="input ml-1 w-14 text-[10px]"
                  value={depth}
                  onChange={(e) => setDepth(Number(e.target.value))}
                  aria-label="Traversal depth"
                >
                  {[1, 2, 3, 4].map((d) => (
                    <option key={d} value={d}>
                      {d}
                    </option>
                  ))}
                </select>
              </label>
            )}

            <input
              className="input text-[10px] min-w-32 flex-1"
              value={filterText}
              onChange={(e) => {
                setFilterText(e.target.value);
                setVisible(PAGE);
              }}
              placeholder="filter results…"
              aria-label="Filter results"
            />

            <select
              className="input text-[10px]"
              value={evidenceFilter}
              onChange={(e) => setEvidenceFilter(e.target.value as Evidence | "all")}
              aria-label="Evidence filter"
            >
              <option value="all">all evidence</option>
              <option value="DETERMINISTIC">deterministic only</option>
              <option value="HEURISTIC">heuristic only</option>
              <option value="UNSUPPORTED">unsupported only</option>
            </select>
          </div>
        )}

        {/* ---- results ---------------------------------------------------
            Each view is gated on `state.tab === <that view>` and on that view
            being the active one: the payload's type comes from narrowing on
            its owner, so there is no `as` cast left to get wrong (G11). */}
        {state.kind === "ready" && state.tab === "references" && tab === "references" && (
          <ReferencesView
            data={state.data}
            filterText={filterText}
            evidenceFilter={evidenceFilter}
            onNavigateToFile={onNavigateToFile}
          />
        )}
        {state.kind === "ready" && state.tab === "calls" && tab === "calls" && (
          <CallsView
            data={state.data}
            filterText={filterText}
            evidenceFilter={evidenceFilter}
            onNavigateToFile={onNavigateToFile}
          />
        )}
        {state.kind === "ready" && state.tab === "graph" && tab === "graph" && (
          <GraphView
            data={state.data}
            filterText={filterText}
            evidenceFilter={evidenceFilter}
            edgeKindFilter={edgeKindFilter}
            setEdgeKindFilter={setEdgeKindFilter}
            visible={visible}
            showMore={() => setVisible((v) => v + PAGE)}
            onNavigateToFile={onNavigateToFile}
          />
        )}
        {state.kind === "ready" && state.tab === "impact" && tab === "impact" && (
          <ImpactView
            data={state.data}
            filterText={filterText}
            evidenceFilter={evidenceFilter}
            onNavigateToFile={onNavigateToFile}
          />
        )}
      </div>
    </Panel>
  );
}

// ---------------------------------------------------------------------------
// References
// ---------------------------------------------------------------------------

function ReferencesView({
  data,
  filterText,
  evidenceFilter,
  onNavigateToFile,
}: {
  data: NavReferenceResult;
  filterText: string;
  evidenceFilter: Evidence | "all";
  onNavigateToFile: (path: string) => void;
}) {
  if (data.unresolved) {
    return <EmptyFor message={`Unresolved: ${data.unresolved}`} />;
  }

  const kept = filterByEvidence(
    [...data.references, ...data.possible].filter(
      (r) => matchesQuery(r.file, filterText) || matchesQuery(r.excerpt, filterText),
    ),
    evidenceFilter,
  ).sort(byEvidenceThen);

  const groups = groupByFile(kept);

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-3 text-[10px] font-mono text-text-muted">
        <span className="text-success">{data.deterministic_count} deterministic</span>
        <span className="text-warning">{data.heuristic_count} heuristic</span>
        <span>{data.possible.length} unattributed name matches</span>
      </div>

      {data.definition && (
        <div className="rounded border border-border px-2 py-1.5">
          <p className="text-[10px] uppercase tracking-widest text-text-muted">Definition</p>
          <button
            className="text-left hover:underline"
            onClick={() => onNavigateToFile(data.definition!.file)}
          >
            <DefinitionLine symbol={data.definition} />
            <span className="font-mono text-[10px] text-text-muted">
              {data.definition.file}:{formatLocation(data.definition.name_range)}
            </span>
          </button>
        </div>
      )}

      {groups.length === 0 ? (
        <EmptyFor message="No reference matched the current filters." />
      ) : (
        groups.map((group) => (
          <details key={group.file} open>
            <summary className="cursor-pointer select-none font-mono text-[11px] text-text">
              {group.file} <span className="text-text-muted">({group.rows.length})</span>
            </summary>
            <ul className="mt-1 space-y-0.5 border-l border-border pl-2">
              {group.rows.map((row, i) => (
                <li key={`${row.range.start_byte}-${i}`} className="flex items-start gap-2">
                  <button
                    className="shrink-0 font-mono text-[10px] text-primary hover:underline"
                    onClick={() => onNavigateToFile(row.file)}
                    title={`Open ${row.file}`}
                  >
                    {formatLocation(row.range)}
                  </button>
                  <span className="shrink-0 font-mono text-[9px] text-text-muted">{row.kind}</span>
                  <span className="min-w-0 flex-1 truncate font-mono text-[10px] text-text-muted">
                    {row.excerpt}
                  </span>
                  <span className="hidden shrink-0 font-mono text-[9px] text-text-muted sm:inline">
                    via {row.via}
                  </span>
                  <EvidenceBadge evidence={row.evidence} />
                </li>
              ))}
            </ul>
          </details>
        ))
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Call hierarchy
// ---------------------------------------------------------------------------

function CallsView({
  data,
  filterText,
  evidenceFilter,
  onNavigateToFile,
}: {
  data: NavCallHierarchyResult;
  filterText: string;
  evidenceFilter: Evidence | "all";
  onNavigateToFile: (path: string) => void;
}) {
  if (data.unresolved) {
    return <EmptyFor message={`Unresolved: ${data.unresolved}`} />;
  }

  const rows = filterByEvidence(
    data.relations.filter(
      (r) =>
        matchesQuery(r.site.file, filterText) ||
        matchesQuery(r.site.callee_name, filterText) ||
        matchesQuery(r.site.excerpt, filterText),
    ),
    evidenceFilter,
  ).sort(byEvidenceThen);

  if (rows.length === 0) {
    return <EmptyFor message="No relation matched the current filters." />;
  }

  const groups = groupByFile(rows.map((r) => ({ file: r.site.file, relation: r })));

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-3 text-[10px] font-mono text-text-muted">
        <span className="text-success">{data.deterministic_count} deterministic</span>
        <span className="text-warning">{data.heuristic_count} heuristic</span>
        <span>{data.unsupported_count} unsupported</span>
      </div>

      {groups.map((group) => (
        <details key={group.file} open>
          <summary className="cursor-pointer select-none font-mono text-[11px] text-text">
            {group.file} <span className="text-text-muted">({group.rows.length})</span>
          </summary>
          <ul className="mt-1 space-y-0.5 border-l border-border pl-2">
            {group.rows.map(({ relation: r }, i) => (
              <li key={`${r.site.range.start_byte}-${i}`} className="flex items-start gap-2">
                <button
                  className="shrink-0 font-mono text-[10px] text-primary hover:underline"
                  onClick={() => onNavigateToFile(r.site.file)}
                  title={`Open ${r.site.file}`}
                >
                  {formatLocation(r.site.range)}
                </button>
                <span className="min-w-0 flex-1 truncate font-mono text-[10px] text-text">
                  {r.resolved ? r.resolved.qualified_name : r.site.callee_name}
                  <span className="text-text-muted"> · {r.site.excerpt}</span>
                </span>
                <EvidenceBadge evidence={r.evidence} />
              </li>
            ))}
          </ul>
        </details>
      ))}

      {data.relations.some((r) => r.evidence !== "DETERMINISTIC") && (
        <p className="text-[10px] text-text-muted">
          Heuristic rows are text-adjacent matches the index could not prove; they are listed after
          the deterministic ones and are never promoted.
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Dependency graph
// ---------------------------------------------------------------------------

function GraphView({
  data,
  filterText,
  evidenceFilter,
  edgeKindFilter,
  setEdgeKindFilter,
  visible,
  showMore,
  onNavigateToFile,
}: {
  data: NavGraphResult;
  filterText: string;
  evidenceFilter: Evidence | "all";
  edgeKindFilter: EdgeKind | "all";
  setEdgeKindFilter: (next: EdgeKind | "all") => void;
  visible: number;
  showMore: () => void;
  onNavigateToFile: (path: string) => void;
}) {
  const matching = useMemo(
    () =>
      filterByEvidence(
        data.edges.filter(
          (e) =>
            (edgeKindFilter === "all" || e.kind === edgeKindFilter) &&
            (matchesQuery(e.detail, filterText) ||
              matchesQuery(e.from.label, filterText) ||
              matchesQuery(e.to.label, filterText)),
        ),
        evidenceFilter,
      ).sort(byEvidenceThen),
    [data.edges, edgeKindFilter, evidenceFilter, filterText],
  );

  const shown = matching.slice(0, visible);

  // Collapse by source node: a 400-edge graph becomes a scannable list of
  // expandable clusters instead of one unbounded wall of rows.
  const clusters = useMemo(() => {
    const order: string[] = [];
    const map = new Map<string, typeof shown>();
    for (const edge of shown) {
      const key = edge.from.id;
      const bucket = map.get(key);
      if (bucket) bucket.push(edge);
      else {
        map.set(key, [edge]);
        order.push(key);
      }
    }
    return order.map((key) => ({ key, edges: map.get(key) ?? [] }));
  }, [shown]);

  const clickable = (path: string) => {
    if (/\.(rs|ts|tsx|js|jsx|py|go|java|md)$/.test(path)) onNavigateToFile(path);
  };

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-[10px] text-text-muted">edge kind</span>
        <select
          className="input text-[10px]"
          value={edgeKindFilter}
          onChange={(e) => setEdgeKindFilter(e.target.value as EdgeKind | "all")}
          aria-label="Edge kind filter"
        >
          <option value="all">all kinds</option>
          {EDGE_ORDER.map((k) => (
            <option key={k} value={k}>
              {k}
            </option>
          ))}
        </select>
        <span className="text-[10px] font-mono text-text-muted">
          {matching.length} of {data.total_edges_matched} edges
          {data.truncated ? " (query truncated)" : ""}
        </span>
      </div>

      {matching.length === 0 ? (
        <EmptyFor message="No edge matched the current filters." />
      ) : (
        <div className="space-y-1">
          {clusters.map((cluster) => (
            <details key={cluster.key}>
              <summary className="cursor-pointer select-none font-mono text-[11px] text-text">
                {cluster.edges[0].from.label}{" "}
                <span className="text-text-muted">
                  {cluster.edges[0].from.kind} → ({cluster.edges.length})
                </span>
              </summary>
              <ul className="mt-1 space-y-0.5 border-l border-border pl-2">
                {cluster.edges.map((edge, i) => (
                  <li key={`${edge.to.id}-${edge.kind}-${i}`} className="flex items-start gap-2">
                    <button
                      className="shrink-0 font-mono text-[9px] text-primary hover:underline"
                      onClick={() => clickable(edge.to.id)}
                      title={
                        /\.(rs|ts|tsx|js|jsx)$/.test(edge.to.id)
                          ? `Open ${edge.to.id}`
                          : "Not a source file — no navigation"
                      }
                    >
                      → {edge.to.label}
                    </button>
                    <span className="min-w-0 flex-1 truncate font-mono text-[10px] text-text-muted">
                      {edge.detail}
                    </span>
                    <EvidenceBadge evidence={edge.evidence} />
                  </li>
                ))}
              </ul>
            </details>
          ))}
        </div>
      )}

      {shown.length < matching.length && (
        <button className="btn text-[10px]" onClick={showMore}>
          Show {Math.min(PAGE, matching.length - shown.length)} more of {matching.length}
        </button>
      )}

      {data.cycles.length > 0 && (
        <details>
          <summary className="cursor-pointer select-none text-[11px] text-warning">
            {data.cycles.length} import cycle{data.cycles.length === 1 ? "" : "s"} detected
          </summary>
          <ul className="mt-1 space-y-0.5 border-l border-border pl-2">
            {data.cycles.map((cycle, i) => (
              <li key={i} className="font-mono text-[10px] text-text-muted">
                {cycle.join(" → ")} → {cycle[0]}
              </li>
            ))}
          </ul>
        </details>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Impact
// ---------------------------------------------------------------------------

function ImpactView({
  data,
  filterText,
  evidenceFilter,
  onNavigateToFile,
}: {
  data: NavImpactResult;
  filterText: string;
  evidenceFilter: Evidence | "all";
  onNavigateToFile: (path: string) => void;
}) {
  if (data.unresolved_subject) {
    return <EmptyFor message={`Unresolved subject: ${data.unresolved_subject}`} />;
  }

  const kept = filterByEvidence(
    data.items.filter(
      (item) =>
        matchesQuery(item.label, filterText) || matchesQuery(item.via.join(" "), filterText),
    ),
    evidenceFilter,
  );

  const counts: Record<ImpactClass, number> = {
    DIRECT_DEPENDENT: data.direct_count,
    TRANSITIVE_DEPENDENT: data.transitive_count,
    POSSIBLE_TEXTUAL_REFERENCE: data.possible_count,
    UNRESOLVED: data.unresolved_count,
  };

  return (
    <div className="space-y-2">
      <p className="text-[10px] text-text-muted">{data.summary}</p>

      <div className="flex flex-wrap gap-3 text-[10px] font-mono">
        <span className="text-success">{counts.DIRECT_DEPENDENT} direct</span>
        <span className="text-primary">{counts.TRANSITIVE_DEPENDENT} transitive</span>
        <span className="text-warning">{counts.POSSIBLE_TEXTUAL_REFERENCE} possible</span>
        <span className="text-text-muted">{counts.UNRESOLVED} unresolved</span>
      </div>

      {kept.length === 0 ? (
        <EmptyFor message="No dependent matched the current filters." />
      ) : (
        IMPACT_ORDER.map((cls) => {
          const rows = kept.filter((item) => item.class === cls);
          if (rows.length === 0) return null;
          return (
            <details key={cls} open>
              <summary className="cursor-pointer select-none font-mono text-[11px] text-text">
                {cls} <span className="text-text-muted">({rows.length})</span>
              </summary>
              <ul className="mt-1 space-y-0.5 border-l border-border pl-2">
                {rows.map((item) => (
                  <li key={`${item.class}-${item.id}`} className="flex items-start gap-2">
                    <button
                      className="shrink-0 font-mono text-[10px] text-primary hover:underline"
                      onClick={() => onNavigateToFile(item.id)}
                      title={item.target_kind === "file" ? `Open ${item.id}` : item.id}
                    >
                      {item.label}
                    </button>
                    <span className="min-w-0 flex-1 truncate font-mono text-[9px] text-text-muted">
                      via {item.via.join(" → ")}
                    </span>
                    <EvidenceBadge evidence={item.evidence} />
                  </li>
                ))}
              </ul>
            </details>
          );
        })
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Secondary containment (G11 defence in depth)
//
// The race itself is fixed above — this exists so that a *rendering* failure
// inside Repository Intelligence can never unmount the IDE around it, which is
// what turned a bad payload into a blank window. It sits at the surface that
// mounts the panel, so it contains, reports truthfully and offers a retry; it
// does not swallow, re-interpret or pretend the underlying error did not
// happen, and the detail is mirrored to Diagnostics.
// ---------------------------------------------------------------------------

interface NavErrorBoundaryProps {
  children: ReactNode;
}

interface NavErrorBoundaryState {
  error: Error | null;
}

export class NavErrorBoundary extends Component<NavErrorBoundaryProps, NavErrorBoundaryState> {
  state: NavErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): NavErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    recordDiagnostic("RepoNavPanel.render", `${error.message} · ${info.componentStack ?? ""}`);
  }

  render() {
    if (this.state.error) {
      return (
        <div role="alert" className="rounded border border-error/40 bg-error/10 p-3">
          <p className="text-[11px] font-semibold text-error">
            Repository Intelligence could not be rendered.
          </p>
          <p className="mt-1 font-mono text-[10px] text-text-muted">{this.state.error.message}</p>
          <p className="mt-1 text-[10px] text-text-muted">
            The rest of ZylCode is unaffected. The full detail is recorded in Diagnostics.
          </p>
          <button
            type="button"
            className="btn mt-2 text-[10px]"
            onClick={() => this.setState({ error: null })}
          >
            Retry
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

// `NavFileDependenciesResult` and `NavDefinitionResult` are re-exported types
// the panel's callers (and tests) build fixtures from.
export type { NavFileDependenciesResult, NavDefinitionResult };
