import { useEffect, useState } from "react";
import { Panel } from "./Panel";
import { StatusBadge } from "./CapabilityStatus";
import {
  REPO_QUERY_METHODS,
  renderResultLines,
  queryNeedsArg,
  runRepoQuery,
  type RepoQueryMethod,
  type RepoQueryState,
} from "../lib/repoQuery";
import {
  watchRepoContext,
  type RepoContextSignal,
} from "../lib/repoContext";

// ---------------------------------------------------------------------------
// The 12 repository query methods, runnable from the UI.
//
// One selector field plus a method picker dispatch to the engine's
// `RepoQuery` layer (`repo_query` Tauri command on desktop,
// `GET /api/repo-query` in the browser preview). Results are rendered from
// the engine payload verbatim; a rejection shows the engine's reason, and an
// unreachable engine shows how to start it — nothing is ever rendered as an
// empty answer when nothing ran.
//
// P15 W4: the panel also subscribes to the freshness signal
// (`GET /api/repo-context-stream` in the browser, the polled
// `repo_context_status` command on desktop): `fresh`, `stale` (the tree
// changed since the index was written — run a query to re-index), and
// `reindexing` (a scan is running right now) are the engine's own states,
// rendered as they arrive; an unreachable observer says so and is never
// painted as `fresh`.
// ---------------------------------------------------------------------------

export function RepoQueryPanel() {
  const [method, setMethod] = useState<RepoQueryMethod>("find_symbol");
  const [arg, setArg] = useState("");
  const [state, setState] = useState<RepoQueryState>({ kind: "idle" });
  const [freshness, setFreshness] = useState<RepoContextSignal>({
    kind: "connecting",
  });

  useEffect(() => watchRepoContext(setFreshness), []);

  const run = async () => {
    setState({ kind: "loading" });
    const next = await runRepoQuery(method, arg);
    setState(next);
  };

  const status =
    state.kind === "ready"
      ? "AVAILABLE"
      : state.kind === "loading" || state.kind === "idle"
        ? "LIMITED"
        : "BLOCKED";

  return (
    <Panel title="REPOSITORY QUERIES">
      <div className="space-y-2" data-testid="repo-query">
        <div className="flex items-center justify-between">
          <StatusBadge status={status} size="xs" />
          {state.kind === "ready" && (
            <span className="text-[10px] font-mono text-text-muted">
              {state.data.elapsed_ms.toFixed(0)} ms ·{" "}
              {state.via === "desktop" ? "desktop engine" : "intel service"}
            </span>
          )}
        </div>

        <p
          role="status"
          data-testid="repo-query-freshness"
          className={`text-[10px] font-mono ${
            freshness.kind === "stale"
              ? "text-warning"
              : freshness.kind === "unavailable"
                ? "text-text-muted"
                : "text-text-secondary"
          }`}
        >
          {freshnessText(freshness)}
        </p>

        <div className="flex gap-2" data-testid="repo-query-form">
          <select
            value={method}
            onChange={(e) => setMethod(e.target.value as RepoQueryMethod)}
            aria-label="Repository query method"
            data-testid="repo-query-method"
            className="bg-surface border border-border rounded px-1.5 py-1 text-xs outline-none focus:border-primary/50"
          >
            {REPO_QUERY_METHODS.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
          <input
            value={arg}
            onChange={(e) => setArg(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void run();
            }}
            placeholder={
              queryNeedsArg(method)
                ? "selector (symbol / path / subject / limit)"
                : "no selector needed"
            }
            disabled={!queryNeedsArg(method) && method !== "recent_changes"}
            aria-label="Repository query selector"
            data-testid="repo-query-arg"
            className="input text-xs flex-1"
          />
          <button
            onClick={() => void run()}
            disabled={state.kind === "loading"}
            data-testid="repo-query-run"
            className="btn btn-primary text-xs"
          >
            {state.kind === "loading" ? "Running…" : "Query"}
          </button>
        </div>

        {state.kind === "idle" && (
          <p className="text-[11px] text-text-muted" data-testid="repo-query-idle">
            Pick a method and run it — the answer is the engine's, with its own
            provenance and timing.
          </p>
        )}

        {state.kind === "loading" && (
          <p className="text-[11px] text-text-muted" role="status">
            Querying the persisted repository index…
          </p>
        )}

        {state.kind === "rejected" && (
          <p className="text-[11px] text-warning" role="alert" data-testid="repo-query-rejected">
            {state.reason}
          </p>
        )}

        {state.kind === "unavailable" && (
          <p className="text-[11px] text-text-muted" role="alert" data-testid="repo-query-unavailable">
            {state.reason}
          </p>
        )}

        {state.kind === "ready" && (
          <div data-testid="repo-query-results">
            <p className="text-[10px] font-mono text-text-muted">
              {state.data.method}
              {state.data.arg !== null ? ` · arg: ${state.data.arg}` : ""}
            </p>
            <ul className="mt-0.5 max-h-44 space-y-0.5 overflow-auto">
              {renderResultLines(state.data.results).map((line) => (
                <li
                  key={line.key}
                  className="font-mono text-[10px] text-text-secondary truncate"
                >
                  {line.text}
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </Panel>
  );
}

/** The freshness line: the engine's own states, verbatim, absences stated. */
function freshnessText(signal: RepoContextSignal): string {
  switch (signal.kind) {
    case "connecting":
      return "index freshness: connecting…";
    case "fresh":
      return `index fresh · ${signal.fingerprint}`;
    case "stale": {
      const changed =
        signal.changedFiles === null
          ? "no trusted index yet"
          : `${signal.changedFiles} file(s) changed since the index`;
      return `index stale · ${changed} — run a query to re-index`;
    }
    case "reindexing":
      return "index reindexing… (a scan is running now)";
    case "unavailable":
      return `index freshness unavailable — ${signal.reason}`;
  }
}

export default RepoQueryPanel;
