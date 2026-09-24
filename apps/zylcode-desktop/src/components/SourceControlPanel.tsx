import { useCallback, useEffect, useState } from "react";
import { Panel } from "./Panel";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import { fetchGitStatus, type GitStatusState } from "../lib/gitStatus";

/**
 * Live Source Control surface: the actual git status (branch, upstream,
 * ahead/behind, per-file entries) and the uncommitted diff stat, read
 * from git by the engine. A failed fetch renders a controlled unavailable
 * state; nothing is synthesized — a dirty tree is shown dirty.
 */
export function SourceControlPanel() {
  const [state, setState] = useState<GitStatusState>({ kind: "loading" });

  const load = useCallback(() => {
    setState({ kind: "loading" });
    let cancelled = false;
    fetchGitStatus().then((next) => {
      if (!cancelled) setState(next);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const headerStatus: CapabilityStatus =
    state.kind === "ready" ? "AVAILABLE" : state.kind === "loading" ? "LIMITED" : "BLOCKED";

  return (
    <Panel title="SOURCE CONTROL">
      <div className="space-y-3">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <StatusBadge status={headerStatus} size="xs" />
            {state.kind === "ready" && (
              <span className="text-[10px] font-mono text-text-muted">
                via {state.via === "desktop" ? "desktop engine" : "intel service"}
              </span>
            )}
          </div>
          <button
            type="button"
            onClick={load}
            className="btn btn-secondary text-xs"
            disabled={state.kind === "loading"}
          >
            {state.kind === "loading" ? "Reading…" : "Refresh"}
          </button>
        </div>

        {state.kind === "unavailable" && (
          <p className="text-xs text-text-muted">{state.reason}</p>
        )}

        {state.kind === "loading" && (
          <p className="text-xs text-text-muted">Reading the git state…</p>
        )}

        {state.kind === "ready" && (
          <>
            <div className="text-xs">
              <span className="font-mono">{state.data.branch}</span>
              {state.data.upstream && (
                <span className="text-text-muted font-mono"> → {state.data.upstream}</span>
              )}
              {(state.data.ahead !== 0 || state.data.behind !== 0) && (
                <span className="ml-1 font-mono text-[10px]">
                  {state.data.ahead !== 0 && <span className="text-primary">↑{state.data.ahead}</span>}
                  {state.data.behind !== 0 && <span className="text-primary">↓{state.data.behind}</span>}
                </span>
              )}
              <span className="ml-2 font-mono text-[10px] text-text-muted">@{state.data.head}</span>
            </div>

            <div className="grid grid-cols-4 gap-2 text-center">
              <div className="rounded border border-border px-1 py-1.5">
                <p className="font-mono text-sm">{state.data.counts.total}</p>
                <p className="text-[10px] text-text-muted">total</p>
              </div>
              <div className="rounded border border-border px-1 py-1.5">
                <p className="font-mono text-sm">{state.data.counts.staged}</p>
                <p className="text-[10px] text-text-muted">staged</p>
              </div>
              <div className="rounded border border-border px-1 py-1.5">
                <p className="font-mono text-sm">{state.data.counts.unstaged}</p>
                <p className="text-[10px] text-text-muted">unstaged</p>
              </div>
              <div className="rounded border border-border px-1 py-1.5">
                <p className="font-mono text-sm">{state.data.counts.untracked}</p>
                <p className="text-[10px] text-text-muted">untracked</p>
              </div>
            </div>

            {state.data.clean ? (
              <p className="text-xs text-text-muted">No changes — the working tree is clean.</p>
            ) : (
              <>
                <div>
                  <p className="text-[11px] font-medium text-text">CHANGES</p>
                  <ul className="mt-0.5 max-h-44 space-y-0.5 overflow-y-auto">
                    {state.data.entries.map((e) => (
                      <li key={`${e.status}:${e.path}`} className="flex items-center gap-2 text-[11px]">
                        <span className="w-6 shrink-0 text-center font-mono text-[10px] text-text-muted">
                          {e.status.trim() === "" ? "??" : e.status}
                        </span>
                        <span className="truncate font-mono">{e.path}</span>
                      </li>
                    ))}
                  </ul>
                </div>

                {state.data.diffstat.files_changed > 0 && (
                  <div>
                    <p className="text-[11px] font-medium text-text">
                      UNCOMMITTED (vs HEAD): {state.data.diffstat.files_changed} file
                      {state.data.diffstat.files_changed === 1 ? "" : "s"},{" "}
                      <span className="text-primary">+{state.data.diffstat.insertions}</span>{" "}
                      <span className="text-[11px] text-[#c0392b]">
                        −{state.data.diffstat.deletions}
                      </span>
                    </p>
                    <ul className="mt-0.5 max-h-32 space-y-0.5 overflow-y-auto">
                      {state.data.diffstat.rows.map((r) => (
                        <li key={r.path} className="flex items-center gap-2 text-[11px]">
                          <span className="truncate font-mono">{r.path}</span>
                          <span className="ml-auto shrink-0 font-mono text-[10px]">
                            <span className="text-primary">+{r.added}</span>{" "}
                            <span className="text-[#c0392b]">−{r.removed}</span>
                          </span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </>
            )}
          </>
        )}
      </div>
    </Panel>
  );
}
