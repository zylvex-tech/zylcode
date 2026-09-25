import React, { useEffect, useMemo, useState } from "react";
import { fetchFileContent, type FileContentState } from "../lib/fileContent";

interface EditorPaneProps {
  openFiles: string[];
  activePath: string | null;
  onSelectTab: (path: string) => void;
  onCloseTab: (path: string) => void;
  /** Prompt line shown when no file is open (keep honest). */
  serviceNote?: string;
}

/** Language display for the breadcrumb tail. Plain-text rendering; no fake highlighting. */
function langLabel(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    rs: "Rust",
    ts: "TypeScript",
    tsx: "TypeScript JSX",
    js: "JavaScript",
    jsx: "JavaScript JSX",
    json: "JSON",
    toml: "TOML",
    md: "Markdown",
    css: "CSS",
    html: "HTML",
    yml: "YAML",
    yaml: "YAML",
    py: "Python",
    sh: "Shell",
    bat: "Batch",
    lock: "Lockfile",
  };
  return map[ext] ?? (ext ? ext.toUpperCase() : "Plain Text");
}

/**
 * Central tabbed code editor. Opens real workspace files through the
 * file-content API — plain, honest text with line numbers. No simulated
 * syntax highlighting; content is exactly what the service serves.
 */
export const EditorPane: React.FC<EditorPaneProps> = ({
  openFiles,
  activePath,
  onSelectTab,
  onCloseTab,
  serviceNote,
}) => {
  const [state, setState] = useState<FileContentState | null>(null);

  useEffect(() => {
    if (!activePath) {
      setState(null);
      return;
    }
    let cancelled = false;
    setState({ kind: "loading" });
    fetchFileContent(activePath).then((next) => {
      if (!cancelled) setState(next);
    });
    return () => {
      cancelled = true;
    };
  }, [activePath]);

  const lines = useMemo(() => {
    if (!state || state.kind !== "ready") return [];
    return state.data.content.split("\n");
  }, [state]);

  if (openFiles.length === 0 || !activePath) {
    return (
      <div className="flex-1 min-h-0 flex flex-col items-center justify-center gap-2 bg-code-background text-code-text">
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.5} className="text-text-muted/40">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
          <polyline points="14 2 14 8 20 8" />
        </svg>
        <p className="text-sm text-text-muted">No file open</p>
        <p className="text-xs text-text-muted/70 max-w-sm text-center">
          {serviceNote ?? "Open a file from the Explorer to read its real content here."}
        </p>
      </div>
    );
  }

  return (
    <div className="flex-1 min-h-0 flex flex-col bg-code-background text-code-text">
      {/* Tab strip */}
      <div className="h-9 shrink-0 flex items-stretch border-b border-border bg-surface/60 overflow-x-auto">
        {openFiles.map((p) => {
          const active = p === activePath;
          return (
            <div
              key={p}
              className={`group flex items-center gap-1.5 pl-3 pr-1.5 border-r border-border cursor-pointer text-xs whitespace-nowrap ${
                active
                  ? "bg-code-background text-text-primary border-t-2 border-t-primary"
                  : "text-text-muted hover:text-text-primary hover:bg-surface"
              }`}
              onClick={() => onSelectTab(p)}
              title={p}
            >
              <span className="font-mono">{p.split("/").pop()}</span>
              <button
                aria-label={`Close ${p}`}
                className={`ml-1 w-4 h-4 rounded flex items-center justify-center ${
                  active ? "opacity-60 hover:opacity-100 hover:bg-surface-hover" : "opacity-0 group-hover:opacity-60 hover:!opacity-100"
                }`}
                onClick={(e) => {
                  e.stopPropagation();
                  onCloseTab(p);
                }}
              >
                ×
              </button>
            </div>
          );
        })}
      </div>

      {/* Breadcrumbs */}
      <div className="h-7 shrink-0 flex items-center gap-1 px-3 border-b border-border/60 text-[11px] text-text-muted font-mono overflow-x-auto">
        {activePath.split("/").map((seg, i, arr) => (
          <span key={i} className="flex items-center gap-1 whitespace-nowrap">
            {i > 0 && <span className="text-text-muted/50">›</span>}
            <span className={i === arr.length - 1 ? "text-text-secondary" : ""}>{seg}</span>
          </span>
        ))}
        <span className="ml-auto text-[10px]">{langLabel(activePath)}</span>
      </div>

      {/* Content */}
      <div className="flex-1 min-h-0 overflow-auto">
        {state?.kind === "loading" && (
          <p className="p-4 text-xs text-text-muted">Loading {activePath}…</p>
        )}
        {state?.kind === "unavailable" && (
          <div className="p-4 space-y-1">
            <p className="text-xs text-red-400">Cannot open {activePath}</p>
            <p className="text-[11px] text-text-muted">{state.reason}</p>
          </div>
        )}
        {state?.kind === "ready" && (
          <>
            {state.data.lossy && (
              <p className="px-4 py-1 text-[11px] text-amber-400 border-b border-border/60">
                Binary or non-UTF-8 content — shown lossily.
              </p>
            )}
            <div className="flex text-xs font-mono leading-5">
              <div className="select-none text-right pl-3 pr-2 py-2 text-text-muted/40 border-r border-border/40 min-w-[3.5rem]">
                {lines.map((_, i) => (
                  <div key={i}>{i + 1}</div>
                ))}
              </div>
              <pre className="px-3 py-2 whitespace-pre">{state.data.content}</pre>
            </div>
          </>
        )}
      </div>
    </div>
  );
};

export default EditorPane;
