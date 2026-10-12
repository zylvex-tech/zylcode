import { useEffect, useState } from "react";
import { Panel } from "./Panel";
import { StatusBadge } from "./CapabilityStatus";
import { fetchTools, type ToolsState, type ToolRow } from "../lib/service";
import { runTool, type ToolRunState } from "../lib/toolRunner";

// ---------------------------------------------------------------------------
// Tool runner
//
// The catalogue panel lists what exists; this is the surface that *runs* it.
// Every invocation goes through the real engine dispatch (IPC on desktop,
// `/api/execute-tool` on serve-intel) with its permission gate and evidence
// sink — there is no second, friendlier runtime behind this UI, and a failed
// tool is shown as a failure with the engine's own message.
// ---------------------------------------------------------------------------

/** Example argument skeletons so the JSON box starts from something valid. */
function exampleArgs(row: ToolRow): string {
  switch (row.id) {
    case "fs.read":
      return '{ "path": "README.md" }';
    case "fs.list":
      return '{ "path": "." }';
    case "fs.write":
      return '{ "path": "notes/hello.txt", "content": "written by the tool runner" }';
    case "search.find":
      return '{ "pattern": "*.rs" }';
    case "search.grep":
      return '{ "pattern": "TODO", "path": "crates" }';
    case "git.status":
      return "{}";
    case "git.diff":
      return "{}";
    case "shell.echo":
      return '{ "message": "hello" }';
    case "shell.execute":
    case "cargo.test":
    case "npm.run":
    case "test.single":
      return '{ "action": "", "args": [] }';
    default:
      if (row.id.startsWith("nav.")) {
        return '{ "name": "symbol_name", "file": "crates/zylcode-core/src/lib.rs", "line": 1 }';
      }
      return "{}";
  }
}

const RUNNING_LABEL: Record<string, string> = {
  desktop: "executing via desktop engine",
  service: "executing via serve-intel",
};

export function ToolRunnerPanel() {
  const [tools, setTools] = useState<ToolsState>({ kind: "loading" });
  const [selected, setSelected] = useState<string>("");
  const [argsText, setArgsText] = useState("{}");
  const [run, setRun] = useState<ToolRunState>({ kind: "idle" });

  // Same catalogue source as the catalogue panel; executor-bound ids only
  // are selectable — the rest fail closed by the product's own contract.
  useEffect(() => {
    let cancelled = false;
    fetchTools().then((next) => {
      if (!cancelled) {
        setTools(next);
        if (next.kind === "ready" && !selected) {
          const first = next.tools.find((t) => t.executor_bound);
          if (first) {
            setSelected(first.id);
            setArgsText(exampleArgs(first));
          }
        }
      }
    });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const catalogueRows = tools.kind === "ready" ? tools.tools : [];
  const runnable = catalogueRows.filter((t) => t.executor_bound);
  const selectedRow = catalogueRows.find((t) => t.id === selected);

  const pick = (row: ToolRow) => {
    setSelected(row.id);
    setRun({ kind: "idle" });
    setArgsText(exampleArgs(row));
  };

  const execute = async () => {
    if (!selected) return;
    setRun({ kind: "running", id: selected });
    let params: unknown;
    try {
      params = argsText.trim() ? JSON.parse(argsText) : {};
    } catch (e) {
      setRun({ kind: "declined", id: selected, reason: `arguments are not valid JSON: ${String(e)}` });
      return;
    }
    setRun(await runTool(selected, params, catalogueRows));
  };

  const status = (() => {
    if (run.kind === "running") return "LIMITED" as const;
    if (run.kind === "done") return run.result.success ? "AVAILABLE" : "LIMITED" as const;
    if (run.kind === "declined" || run.kind === "failed") return "BLOCKED" as const;
    return "LIMITED" as const;
  })();

  return (
    <div className="space-y-2" data-testid="tool-runner">
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-medium">Run a tool</h4>
        <span className="flex items-center gap-2">
          {run.kind === "running" && <StatusBadge status="LIMITED" size="xs" />}
          {run.kind !== "running" && <StatusBadge status={status} size="xs" />}
        </span>
      </div>

      {tools.kind === "loading" && <p className="text-xs text-text-muted">Reading the tool catalogue…</p>}
      {tools.kind === "unavailable" && <p className="text-xs text-text-muted">{tools.reason}</p>}

      {tools.kind === "ready" && (
        <>
          {runnable.length === 0 ? (
            <p className="text-xs text-text-muted">
              The catalogue has no executor-bound tools — start `zylcode serve-intel` from the
              repository root (desktop runs them in-process).
            </p>
          ) : (
            <div className="flex flex-wrap gap-1" data-testid="tool-runner-ids">
              {runnable.map((t) => (
                <button
                  key={t.id}
                  onClick={() => pick(t)}
                  aria-pressed={selected === t.id}
                  className={`rounded border px-1.5 py-0.5 font-mono text-[10px] transition-colors ${
                    selected === t.id
                      ? "border-primary/60 bg-primary/15 text-primary"
                      : "border-border text-text-secondary hover:border-primary/40 hover:text-text-primary"
                  }`}
                  title={t.description ?? undefined}
                >
                  {t.id}
                </button>
              ))}
            </div>
          )}

          {selectedRow && (
            <div className="space-y-1.5" data-testid="tool-runner-form">
              <p className="text-[10px] text-text-muted">
                {selectedRow.description ?? ""} — payload is the engine's own result, including
                risk class and evidence id.
              </p>
              <textarea
                value={argsText}
                onChange={(e) => setArgsText(e.target.value)}
                rows={4}
                spellCheck={false}
                aria-label="Tool arguments (JSON)"
                className="w-full rounded-md border border-border bg-background px-2.5 py-2 font-mono text-xs outline-none focus:border-primary focus:ring-2 focus:ring-primary/20 transition-all resize-none"
              />
              <button
                onClick={execute}
                disabled={run.kind === "running"}
                data-testid="tool-runner-execute"
                className="rounded-md border border-border bg-surface-active px-3 py-1.5 text-xs font-medium transition-colors hover:border-primary/60 hover:text-primary disabled:opacity-50"
              >
                {run.kind === "running" ? "Running…" : "Execute"}
              </button>
            </div>
          )}

          <div data-testid="tool-runner-result">{renderRun(run)}</div>
        </>
      )}
    </div>
  );
}

function renderRun(run: ToolRunState) {
  if (run.kind === "idle") {
    return <p className="text-[10px] text-text-muted">Choose a tool and execute — results are the engine's own.</p>;
  }
  if (run.kind === "running") {
    return <p className="text-[11px] text-text-muted">Executing {run.id}…</p>;
  }
  if (run.kind === "declined") {
    return (
      <p className="text-[11px] text-warning" role="alert">
        Declined: {run.reason}
      </p>
    );
  }
  if (run.kind === "failed") {
    return (
      <div role="alert" className="rounded border border-error/30 bg-error/5 p-2">
        <p className="text-[11px] text-error font-mono break-words">{run.error}</p>
        <p className="text-[10px] text-text-muted mt-0.5">via {run.via === "desktop" ? "desktop engine" : "serve-intel"}</p>
      </div>
    );
  }
  if (run.kind === "unavailable") {
    return (
      <p className="text-[11px] text-error" role="alert">
        {run.reason}
      </p>
    );
  }
  const result = run.result;
  return (
    <div className="rounded border border-success/30 bg-success/5 p-2" data-testid="tool-runner-output">
      <p className="text-[10px] font-mono text-text-muted">
        {result.tool} · risk {String(result.risk ?? "n/a")} · operation{" "}
        <span className="text-text-secondary">{String(result.bound_operation ?? "n/a")}</span>
        {result.invocation_id && <> · evidence {String(result.invocation_id)}</>}
        {result.changed_files && result.changed_files.length > 0 && (
          <> · changed {result.changed_files.length} file(s)</>
        )}
      </p>
      <pre className="mt-1 max-h-48 overflow-auto whitespace-pre-wrap break-words font-mono text-[10px] text-text-secondary">
        {typeof result.output === "string"
          ? result.output
          : JSON.stringify(result.output, null, 2) ?? ""}
      </pre>
      <p className="text-[10px] text-text-muted mt-1">
        via {run.via === "desktop" ? "desktop engine" : RUNNING_LABEL.service}
      </p>
    </div>
  );
}

export default ToolRunnerPanel;
