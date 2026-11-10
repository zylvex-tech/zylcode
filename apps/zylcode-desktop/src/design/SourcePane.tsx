// ---------------------------------------------------------------------------
// Source navigation (design/SourcePane.tsx)
//
// The index records `file:line` for everything it discovered, and this pane
// opens that file through the same service the editor uses (`lib/fileContent`):
// the Tauri `file_content` command in the desktop app, the proxied HTTP service
// in the browser preview. The window is centred on the indexed line, which is
// what makes "go to source" land on the component rather than at the top of a
// 700-line file.
//
// When the service is unavailable the pane says so and keeps the reference; it
// never shows remembered source that could be stale.
// ---------------------------------------------------------------------------

import { useEffect, useState } from "react";
import { fetchFileContent } from "../lib/fileContent";

type SourceState =
  | { status: "loading" }
  | { status: "unavailable"; reason: string }
  | { status: "ready"; start: number; end: number; total: number; lines: string[]; lossy: boolean };

const BEFORE = 12;
const AFTER = 48;

export function SourcePane({ file, line }: { file: string; line: number }) {
  const [state, setState] = useState<SourceState>({ status: "loading" });
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setState({ status: "loading" });

    void fetchFileContent(file).then((result) => {
      if (cancelled || result.kind === "loading") return;
      if (result.kind === "unavailable") {
        setState({ status: "unavailable", reason: result.reason });
        return;
      }
      const lines = result.data.content.split(/\r?\n/);
      // The indexed line can outrun the file if the index is stale; clamp the
      // window to the file and let the pane say the line is not there.
      const target = Math.min(line, lines.length);
      const start = Math.max(1, target - BEFORE);
      const end = Math.min(lines.length, target + AFTER);
      setState({
        status: "ready",
        start,
        end,
        total: lines.length,
        lines: lines.slice(start - 1, end),
        lossy: result.data.lossy,
      });
    });

    return () => {
      cancelled = true;
    };
  }, [file, line]);

  return (
    <div className="flex min-h-0 flex-col">
      <div className="flex items-center justify-between gap-2 border-b border-border px-2 py-1.5">
        <p className="truncate font-mono text-[11px] text-text-muted">
          {file}:{line}
        </p>
        <button
          type="button"
          className="shrink-0 rounded border border-border px-2 py-0.5 text-[10px] font-mono text-text-muted"
          onClick={() => {
            void navigator.clipboard?.writeText(`${file}:${line}`).then(() => {
              setCopied(true);
              window.setTimeout(() => setCopied(false), 1500);
            });
          }}
        >
          {copied ? "copied" : "copy reference"}
        </button>
      </div>

      {state.status === "loading" && <p className="p-3 text-xs text-text-muted">reading source…</p>}

      {state.status === "unavailable" && (
        <div className="p-3">
          <p className="text-[10px] font-mono uppercase tracking-[0.16em] text-warning">
            SOURCE UNAVAILABLE
          </p>
          <p className="mt-1 text-xs text-text-muted">{state.reason}</p>
        </div>
      )}

      {state.status === "ready" && (
        <>
          <div className="max-h-80 overflow-auto bg-background/60" data-testid="source-lines">
            <pre className="min-w-full font-mono text-[11px] leading-5">
              {state.lines.map((text, index) => {
                const number = state.start + index;
                const target = number === line;
                return (
                  <div
                    key={number}
                    data-line={number}
                    className={`flex ${target ? "bg-primary/10" : ""}`}
                  >
                    <span className="w-10 shrink-0 select-none pr-2 text-right text-text-muted/70">
                      {number}
                    </span>
                    <span
                      className={`whitespace-pre ${target ? "text-text-primary" : "text-text-secondary"}`}
                    >
                      {text || " "}
                    </span>
                  </div>
                );
              })}
            </pre>
          </div>
          <p className="border-t border-border px-2 py-1 text-[10px] text-text-muted">
            lines {state.start}–{state.end} of {state.total}
            {state.lossy ? " · file read lossily (binary or oversized)" : ""}
            {line > state.total
              ? ` · indexed line ${line} is beyond this file — the index may be stale`
              : ""}
          </p>
        </>
      )}
    </div>
  );
}
