import { useEffect, useState } from "react";
import { Panel } from "../Panel";
import { fetchGitStatus, type GitStatusState } from "../../lib/gitStatus";

interface ContextSidebarProps {
  activeActivity: "explorer" | "search" | "source-control" | "missions" | "run" | "evidence" | "forge" | "extensions" | "settings" | "account";
  children?: React.ReactNode;
}

export function ContextSidebar({ activeActivity, children }: ContextSidebarProps) {
  const renderContent = () => {
    switch (activeActivity) {
      case "explorer":
        return (
          <Panel title="EXPLORER">
            <div className="space-y-2 text-sm">
              <p className="text-text-muted">No folder opened</p>
              <button className="btn btn-secondary w-full justify-start text-xs">
                Open Folder
              </button>
            </div>
          </Panel>
        );

      case "search":
        return (
          <Panel title="SEARCH">
            <div className="space-y-2">
              <input
                type="text"
                placeholder="Search in files..."
                className="input text-sm"
              />
              <p className="text-xs text-text-muted">Search results will appear here</p>
            </div>
          </Panel>
        );

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
        return (
          <Panel title="EVIDENCE">
            <div className="space-y-2 text-sm">
              <p className="text-text-muted">Evidence timeline will appear here</p>
            </div>
          </Panel>
        );

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