import { useCallback, useEffect, useState } from "react";
import { Panel } from "./Panel";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import {
  fetchRepoIntel,
  type RepoIntelState,
} from "../lib/repoIntel";

interface RepoIntelPanelProps {
  /** Initial task used for the first fetch. */
  initialTask?: string;
}

/**
 * Live Repository Intelligence surface: queries the real intelligence
 * pipeline (persisted index + ranked retrieval) and renders its actual
 * output — counts, entry points, recent changes, ranked results with
 * provenance reasons. A failed fetch renders a controlled unavailable
 * state; nothing is synthesized.
 */
export function RepoIntelPanel({ initialTask = "" }: RepoIntelPanelProps) {
  const [state, setState] = useState<RepoIntelState>({ kind: "loading" });
  const [taskInput, setTaskInput] = useState(initialTask);

  const load = useCallback((task: string) => {
    setState({ kind: "loading" });
    let cancelled = false;
    fetchRepoIntel(task).then((next) => {
      if (!cancelled) setState(next);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    load(initialTask);
  }, [initialTask, load]);

  const headerStatus: CapabilityStatus =
    state.kind === "ready" ? "AVAILABLE" : state.kind === "loading" ? "LIMITED" : "BLOCKED";

  return (
    <Panel title="REPOSITORY INTELLIGENCE">
      <div className="space-y-3">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <StatusBadge status={headerStatus} size="xs" />
            {state.kind === "ready" && (
              <span className="text-[10px] font-mono text-text-muted">
                via {state.via === "desktop" ? "desktop engine" : "intel service"} ·{" "}
                {state.data.elapsed_ms.toFixed(0)} ms
              </span>
            )}
          </div>
          <span className="text-[10px] font-mono text-text-muted">live pipeline</span>
        </div>

        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            load(taskInput);
          }}
        >
          <input
            type="text"
            value={taskInput}
            onChange={(e) => setTaskInput(e.target.value)}
            placeholder="Ask the repository a task…"
            className="input text-xs flex-1"
          />
          <button
            type="submit"
            className="btn btn-primary text-xs"
            disabled={state.kind === "loading"}
          >
            {state.kind === "loading" ? "Indexing…" : "Query"}
          </button>
        </form>

        {state.kind === "unavailable" && (
          <p className="text-xs text-text-muted">{state.reason}</p>
        )}

        {state.kind === "loading" && (
          <p className="text-xs text-text-muted">
            Indexing the repository (persisted, content-hash validated)…
          </p>
        )}

        {state.kind === "ready" && (
          <>
            <div className="grid grid-cols-3 gap-2 text-center">
              <div className="rounded border border-border px-2 py-1.5">
                <p className="font-mono text-sm">{state.data.summary.file_count}</p>
                <p className="text-[10px] text-text-muted">files</p>
              </div>
              <div className="rounded border border-border px-2 py-1.5">
                <p className="font-mono text-sm">{state.data.summary.symbol_count}</p>
                <p className="text-[10px] text-text-muted">symbols</p>
              </div>
              <div className="rounded border border-border px-2 py-1.5">
                <p className="font-mono text-sm">{state.data.summary.package_count}</p>
                <p className="text-[10px] text-text-muted">packages</p>
              </div>
            </div>

            {state.data.summary.languages.length > 0 && (
              <p className="text-[11px] text-text-muted">
                <span className="text-text">Languages:</span>{" "}
                {state.data.summary.languages.slice(0, 8).join(", ")}
                {state.data.summary.languages.length > 8 && " …"}
              </p>
            )}

            {state.data.summary.entry_points.length > 0 && (
              <div>
                <p className="text-[11px] font-medium text-text">ENTRY POINTS</p>
                <ul className="mt-0.5 space-y-0.5">
                  {state.data.summary.entry_points.slice(0, 5).map((ep) => (
                    <li key={ep} className="font-mono text-[10px] text-text-muted truncate">
                      {ep}
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {state.data.summary.recent_changes.length > 0 && (
              <div>
                <p className="text-[11px] font-medium text-text">RECENT CHANGES</p>
                <ul className="mt-0.5 space-y-0.5">
                  {state.data.summary.recent_changes.map((c) => (
                    <li key={c.short_id} className="font-mono text-[10px] text-text-muted truncate">
                      <span className="text-primary">{c.short_id}</span> {c.message}
                    </li>
                  ))}
                </ul>
              </div>
            )}

            <div>
              <p className="text-[11px] font-medium text-text">
                RANKED CONTEXT FOR “{state.data.task}”
              </p>
              {state.data.results.length === 0 ? (
                <p className="mt-0.5 text-[11px] text-text-muted">
                  The retriever returned no results for this task.
                </p>
              ) : (
                <ol className="mt-0.5 space-y-1">
                  {state.data.results.slice(0, 10).map((r) => (
                    <li key={`${r.resource_type}:${r.resource}`} className="text-[11px]">
                      <span className="font-mono text-[10px] text-text-muted">
                        {r.relevance.toFixed(2)}
                      </span>{" "}
                      <span className="font-mono">{r.resource}</span>{" "}
                      <span className="text-[10px] text-text-muted">[{r.resource_type}]</span>
                      <p className="text-[10px] text-text-muted truncate">{r.reason}</p>
                    </li>
                  ))}
                </ol>
              )}
            </div>
          </>
        )}
      </div>
    </Panel>
  );
}
