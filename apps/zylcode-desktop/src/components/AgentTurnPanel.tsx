import { useState } from "react";
import { runAgentTurn, type AgentTurnState } from "../lib/agentTurn";

// ---------------------------------------------------------------------------
// Agent turn surface
//
// The UI driver for the deterministic agent journey: one prompt goes through
// the engine's real pipeline — context gathering (repository index +
// navigation citations) → planning → tool execution → verification →
// evidence. Two transports (desktop IPC / serve-intel HTTP), one honesty
// rule: `success: false` is a failure and is shown as one.
//
// This proves the *loop*, complementing the ToolRunnerPanel which proves a
// single tool call.
// ---------------------------------------------------------------------------

export function AgentTurnPanel() {
  const [prompt, setPrompt] = useState("");
  const [turn, setTurn] = useState<AgentTurnState>({ kind: "idle" });
  const [elapsedMs, setElapsedMs] = useState<number | null>(null);

  const execute = async () => {
    if (!prompt.trim() || turn.kind === "running") return;
    const started = Date.now();
    setTurn({ kind: "running" });
    setElapsedMs(null);
    const result = await runAgentTurn(prompt);
    setElapsedMs(Date.now() - started);
    setTurn(result);
  };

  const status = (() => {
    if (turn.kind === "running") return "LIMITED";
    if (turn.kind === "done") return turn.result.success ? "AVAILABLE" : "BLOCKED";
    if (turn.kind === "failed" || turn.kind === "unavailable") return "BLOCKED";
    return "LIMITED";
  })();

  return (
    <div className="space-y-2" data-testid="agent-turn">
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-medium">Agent turn — journey end to end</h4>
        <span className="flex items-center gap-2">
          {turn.kind === "running" && <span data-testid="agent-turn-badge">…</span>}
          {turn.kind !== "running" && <span data-testid="agent-turn-badge">{status}</span>}
        </span>
      </div>

      <p className="text-[10px] text-text-muted">
        One prompt runs the whole loop against this workspace: context gathering → plan →
        tool calls → verification → evidence ledger. Without a funded provider credential
        the model layer degrades to the synthetic offline provider — the loop still
        completes, and every provider failover is recorded.
      </p>

      <textarea
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) void execute();
        }}
        placeholder="e.g. analyze this repository structure"
        rows={2}
        data-testid="agent-turn-prompt"
        aria-label="Agent turn prompt"
        className="w-full rounded-md border border-border bg-background px-2.5 py-2 font-mono text-xs outline-none focus:border-primary focus:ring-2 focus:ring-primary/20 resize-none"
      />

      <button
        onClick={execute}
        disabled={turn.kind === "running" || !prompt.trim()}
        data-testid="agent-turn-run"
        className="rounded-md border border-border bg-surface-active px-3 py-1.5 text-xs font-medium transition-colors hover:border-primary/60 hover:text-primary disabled:opacity-50"
      >
        {turn.kind === "running" ? "Running the journey…" : "Run turn"}
      </button>

      {elapsedMs !== null && turn.kind === "done" && (
        <p className="text-[10px] text-text-muted" data-testid="agent-turn-timing">
          completed in {(elapsedMs / 1000).toFixed(1)}s
        </p>
      )}

      <div data-testid="agent-turn-result">{renderTurn(turn)}</div>
    </div>
  );
}

function renderTurn(turn: AgentTurnState) {
  if (turn.kind === "idle") {
    return (
      <p className="text-[10px] text-text-muted">
        Enter a prompt and run the turn — the result and the summary are the engine's own.
      </p>
    );
  }
  if (turn.kind === "running") {
    return (
      <p className="text-[11px] text-text-muted" role="status">
        Gathering context, planning, executing tools…
      </p>
    );
  }
  if (turn.kind === "unavailable") {
    return (
      <p className="text-[11px] text-error" role="alert">
        {turn.reason}
      </p>
    );
  }
  if (turn.kind === "failed") {
    return (
      <div role="alert" className="rounded border border-error/30 bg-error/5 p-2">
        <p className="text-[11px] text-error font-mono break-words">{turn.error}</p>
      </div>
    );
  }
  const { result } = turn;
  return (
    <div className="rounded border border-border bg-surface p-2" data-testid="agent-turn-output">
      <div className="flex items-center gap-2" data-testid="agent-turn-verdict">
        <span className={result.success ? "text-success" : "text-error"}>
          {result.success ? "✓ success" : "✕ failed"}
        </span>
        <span className="text-[10px] text-text-muted">
          via {turn.via === "desktop" ? "desktop engine" : "serve-intel"}
        </span>
      </div>
      <p className="mt-1 font-mono text-[11px] text-text-secondary break-words">
        {result.summary}
      </p>
      {result.artifacts.length > 0 && (
        <div className="mt-1 space-y-1">
          {result.artifacts.map((a, i) => (
            <pre
              key={i}
              className="max-h-40 overflow-auto whitespace-pre-wrap break-words font-mono text-[10px] text-text-muted"
            >
              [{a.kind}] {a.label}: {a.content}
            </pre>
          ))}
        </div>
      )}
      <p className="text-[10px] text-text-muted mt-1">
        turn summary and verdict are the engine's own — never reshaped by this UI.
      </p>
    </div>
  );
}

export default AgentTurnPanel;
