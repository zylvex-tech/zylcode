import { useEffect, useState } from "react";
import { Panel } from "../Panel";
import { ExplorerTree } from "../ExplorerTree";
import { fetchGitStatus, type GitStatusState } from "../../lib/gitStatus";
import { fetchEvidence, type EvidenceState } from "../../lib/surfaces";

interface ContextSidebarProps {
  activeActivity: "explorer" | "search" | "source-control" | "missions" | "run" | "evidence" | "forge" | "extensions" | "settings" | "account";
  children?: React.ReactNode;
}

export function ContextSidebar({ activeActivity, children }: ContextSidebarProps) {
  const renderContent = () => {
    switch (activeActivity) {
      case "explorer":
        return <ExplorerTree />;

      case "search":
        return <SearchSummary />;

      case "source-control":
        return <SourceControlSummary />;

      case "missions":
        return (
          <Panel title="MISSIONS">
            <div className="space-y-2 text-sm">
              <p className="text-text-muted">Mission history will appear here</p>
              <button className="btn btn-primary text-xs w-full justify-start">New Mission</button>
            </div>
          </Panel>
        );

      case "run":
        return (
          <Panel title="RUN">
            <div className="space-y-2 text-sm">
              <p className="text-text-muted">Runtime targets will appear here</p>
            </div>
          </Panel>
        );

      case "evidence":
        return <EvidenceSummary />;

      case "forge":
        return (
          <Panel title="FORGE">
            <div className="space-y-2 text-sm">
              <p className="text-text-muted">Installed capability packs will appear here</p>
            </div>
          </Panel>
        );

      case "extensions":
        return (
          <Panel title="EXTENSIONS">
            <div className="space-y-2 text-sm">
              <p className="text-text-muted">Extension management coming soon</p>
            </div>
          </Panel>
        );

      default: {
        const label = String(activeActivity).toUpperCase();
        return (
          <Panel title={label}>
            <p className="text-sm text-text-muted">Select an activity</p>
          </Panel>
        );
      }
    }
  };

  return (
    <aside className="w-64 shrink-0 border-r border-border bg-surface/50 flex flex-col min-h-0">
      {children || renderContent()}
    </aside>
  );
}

/** Compact live search for the search activity. */
function SearchSummary() {
  const [query, setQuery] = useState("");
  const [state, setState] = useState<
    { kind: "idle" } | { kind: "unavailable"; reason: string } | { kind: "ready"; count: number; top: string | null }
  >({ kind: "idle" });

  const run = () => {
    if (query.trim().length === 0) return;
    import("../../lib/surfaces").then(({ fetchSearch }) =>
      fetchSearch(query).then((next) => {
        if (next.kind === "ready") {
          setState({ kind: "ready", count: next.data.results.length, top: next.data.results[0]?.resource ?? null });
        } else if (next.kind === "unavailable") {
          setState({ kind: "unavailable", reason: next.reason });
        }
      }),
    );
  };

  return (
    <Panel title="SEARCH">
      <div className="space-y-2">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && run()}
          placeholder="Search symbols, files…"
          className="input text-xs"
        />
        {state.kind === "unavailable" && <p className="text-xs text-text-muted">{state.reason}</p>}
        {state.kind === "ready" && (
          <p className="text-xs text-text-muted">
            {state.count} result{state.count === 1 ? "" : "s"}
            {state.top ? ` · top: ${state.top.split(/[\\/]/).pop()}` : ""}
          </p>
        )}
        {state.kind === "idle" && (
          <p className="text-xs text-text-muted">Results come from the real index, ranked.</p>
        )}
        <button className="btn btn-primary text-xs w-full justify-start" onClick={run} disabled={query.trim().length === 0}>
          Search
        </button>
      </div>
    </Panel>
  );
}

/** Compact live evidence summary for the evidence activity. */
function EvidenceSummary() {
  const [state, setState] = useState<EvidenceState>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    fetchEvidence().then((next) => {
      if (!cancelled) setState(next);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <Panel title="EVIDENCE">
      <div className="space-y-2 text-sm">
        {state.kind === "unavailable" && <p className="text-text-muted">{state.reason}</p>}
        {state.kind === "loading" && <p className="text-text-muted">Reading the ledger…</p>}
        {state.kind === "empty" && <p className="text-text-muted">No evidence recorded yet.</p>}
        {state.kind === "ready" && (
          <>
            <p className="text-text-muted">
              {state.entry_count} entr{state.entry_count === 1 ? "y" : "ies"} ·{" "}
              {state.session_count} session{state.session_count === 1 ? "" : "s"}
            </p>
            <p className={`font-mono text-xs ${state.chain_intact ? "text-primary" : "text-[#c0392b]"}`}>
              {state.chain_intact ? "✓ hash chain intact" : "✗ chain broken"}
            </p>
          </>
        )}
        <button className="btn btn-secondary text-xs w-full justify-start" onClick={() => fetchEvidence().then(setState)}>
          Refresh
        </button>
      </div>
    </Panel>
  );
}

/** Compact live summary for the source-control activity (count + branch). */
function SourceControlSummary() {
  const [state, setState] = useState<GitStatusState>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    fetchGitStatus().then((next) => {
      if (!cancelled) setState(next);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <Panel title="SOURCE CONTROL">
      <div className="space-y-2 text-sm">
        {state.kind === "unavailable" && (
          <p className="text-text-muted">{state.reason}</p>
        )}
        {state.kind === "loading" && <p className="text-text-muted">Reading the git state…</p>}
        {state.kind === "ready" && (
          <>
            <p>
              <span className="font-mono text-xs">{state.data.branch}</span>
              {(state.data.ahead !== 0 || state.data.behind !== 0) && (
                <span className="ml-1 font-mono text-[10px] text-text-muted">
                  ↑{state.data.ahead} ↓{state.data.behind}
                </span>
              )}
            </p>
            <p className="text-text-muted">
              {state.data.clean
                ? "No changes"
                : `${state.data.counts.total} change${state.data.counts.total === 1 ? "" : "s"}`}
            </p>
          </>
        )}
        <div className="flex gap-2">
          <button className="btn btn-primary text-xs flex-1">Commit</button>
          <button className="btn btn-secondary text-xs flex-1">Refresh</button>
        </div>
      </div>
    </Panel>
  );
}