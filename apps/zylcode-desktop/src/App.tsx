import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useStreamSubscription } from "./lib/events";
import TokenMetricsWidget from "./components/TokenMetricsWidget";
import McpInspector from "./components/McpInspector";

type IntentResult = {
  summary: string;
  artifacts: { kind: string; label: string; content: string }[];
  success: boolean;
};

type VerificationReport = {
  passed: boolean;
  checks: { name: string; passed: boolean; message: string }[];
  duration_ms: number;
};

type ToolDescriptor = {
  id: string;
  transport: string;
  command: string;
  env_keys: string[];
  description?: string | null;
};

export default function App() {
  const [prompt, setPrompt] = useState("");
  const [model, setModel] = useState("");
  const [intentResult, setIntentResult] = useState<IntentResult | null>(null);
  const [verification, setVerification] = useState<VerificationReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tools, setTools] = useState<ToolDescriptor[]>([]);
  const [metricsKey, setMetricsKey] = useState(0);

  const { deltas, done, mcpCalls, isStreaming } = useStreamSubscription(true);

  useEffect(() => {
    invoke<ToolDescriptor[]>("list_tools")
      .then(setTools)
      .catch(() => invoke<ToolDescriptor[]>("list_mcp_bridges").then(setTools).catch(() => {}));
  }, []);

  useEffect(() => {
    if (done) {
      setIntentResult(done);
      setStreaming(false);
      setBusy(false);
      setMetricsKey((k) => k + 1);
    }
  }, [done]);

  useEffect(() => {
    setStreaming(isStreaming);
  }, [isStreaming]);

  async function handleProcessIntent(useStream: boolean) {
    if (!prompt.trim()) return;
    setBusy(true);
    setStreaming(useStream);
    setError(null);
    setIntentResult(null);
    try {
      if (useStream) {
        const result = await invoke<IntentResult>("process_intent_stream", {
          prompt,
          model: model.trim() || null,
        });
        setIntentResult(result);
      } else {
        const result = await invoke<IntentResult>("process_intent", {
          prompt,
          model: model.trim() || null,
        });
        setIntentResult(result);
      }
      setMetricsKey((k) => k + 1);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setStreaming(false);
    }
  }

  async function handleVerify() {
    setBusy(true);
    setError(null);
    try {
      const report = await invoke<VerificationReport>("verify_logic");
      setVerification(report);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="min-h-screen bg-zyl-bg text-white flex flex-col">
      <header className="border-b border-zyl-border bg-zyl-surface px-6 py-3 flex items-start justify-between gap-4">
        <div>
          <h1 className="text-xl font-bold tracking-tight">
            ZylCode <span className="font-mono text-sm font-normal text-zyl-muted">desktop v0.2.0-dev</span>
          </h1>
          <p className="text-xs text-zyl-muted">Intent workspace — streaming synthesis, MCP tools, artifact preview</p>
        </div>
        <TokenMetricsWidget refreshKey={metricsKey} />
      </header>

      <div className="flex flex-1 min-h-0">
        {/* Left: Stream & Tool Panel */}
        <aside className="w-[380px] shrink-0 border-r border-zyl-border bg-zyl-surface/50 flex flex-col min-h-0">
          <div className="p-4 border-b border-zyl-border space-y-3">
            <h2 className="text-xs font-semibold uppercase tracking-widest text-zyl-muted">Stream & Tool Panel</h2>
            <div className="flex gap-2">
              <input
                value={model}
                onChange={(e) => setModel(e.target.value)}
                placeholder="model override (optional)"
                className="w-full rounded-md border border-zyl-border bg-zyl-bg px-2 py-1.5 text-xs font-mono outline-none focus:border-zyl-accent"
              />
            </div>
            <div className="text-[11px] font-mono text-zyl-muted">
              {tools.length > 0 ? (
                <span>{tools.map((t) => t.id).join(", ")}</span>
              ) : (
                <span>tools: — (add mcp.tools.yaml)</span>
              )}
            </div>
          </div>

          <div className="flex-1 overflow-auto p-4 space-y-4">
            <McpInspector />

            {/* Execution timeline */}
            <div className="rounded border border-zyl-border bg-zyl-bg p-3">
              <div className="text-[11px] font-semibold uppercase tracking-widest text-zyl-muted mb-2">
                Execution Timeline
              </div>
              {deltas.length === 0 && mcpCalls.length === 0 ? (
                <p className="text-xs text-zyl-muted">No streaming events yet.</p>
              ) : (
                <div className="space-y-2">
                  <div className="text-xs font-mono text-zyl-muted">
                    {deltas.length} deltas {streaming ? "· streaming…" : "· idle"}
                  </div>
                  <ul className="space-y-1 max-h-40 overflow-auto pr-1">
                    {deltas.slice(-20).map((d, i) => (
                      <li key={i} className="text-[11px] font-mono truncate text-zyl-muted">
                        [{d.index}] {d.phase ?? ""} {d.delta.slice(0, 80)} {d.done ? "✓" : ""}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </div>
        </aside>

        {/* Right: Artifact Viewer + Intent */}
        <main className="flex-1 flex flex-col min-h-0 overflow-auto">
          <div className="p-6 space-y-6 max-w-4xl w-full">
            {/* Intent */}
            <section className="rounded-lg border border-zyl-border bg-zyl-surface p-5">
              <h2 className="mb-3 text-sm font-semibold uppercase tracking-widest text-zyl-muted">Process Intent</h2>
              <div className="flex gap-3">
                <input
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleProcessIntent(true)}
                  placeholder="Describe what you want to build…"
                  className="flex-1 rounded-md border border-zyl-border bg-zyl-bg px-3 py-2 text-sm outline-none focus:border-zyl-accent"
                />
                <button
                  onClick={() => handleProcessIntent(false)}
                  disabled={busy || !prompt.trim()}
                  className="rounded-md border border-zyl-border px-3 py-2 text-sm font-semibold hover:bg-zyl-bg disabled:opacity-40"
                  title="Non-streaming invoke"
                >
                  Run
                </button>
                <button
                  onClick={() => handleProcessIntent(true)}
                  disabled={busy || !prompt.trim()}
                  className="rounded-md bg-zyl-accent px-4 py-2 text-sm font-semibold text-white hover:opacity-90 disabled:opacity-40"
                  title="Streaming via intent:chunk"
                >
                  {busy ? "…" : "Stream"}
                </button>
              </div>
              {streaming && <p className="mt-2 text-xs font-mono text-sky-300">Streaming… {deltas.length} chunks</p>}
              {intentResult && (
                <div className="mt-4 rounded-md bg-zyl-bg p-3 font-mono text-xs">
                  <p className="text-white">{intentResult.summary}</p>
                  {intentResult.artifacts.length > 0 && (
                    <ul className="mt-2 space-y-1 text-zyl-muted">
                      {intentResult.artifacts.map((a, i) => (
                        <li key={i} className="truncate">
                          [{a.kind}] {a.label}: {a.content.slice(0, 120)}
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              )}
            </section>

            {/* Artifact Viewer placeholder (Phase 3.2 Monaco) */}
            <section className="rounded-lg border border-zyl-border bg-zyl-surface p-5">
              <div className="flex items-center justify-between mb-3">
                <h2 className="text-sm font-semibold uppercase tracking-widest text-zyl-muted">Artifact Viewer</h2>
                <span className="text-[11px] font-mono text-zyl-muted">Phase 3.2 · Monaco + Diff</span>
              </div>
              {intentResult?.artifacts && intentResult.artifacts.length > 0 ? (
                <div className="grid gap-2">
                  {intentResult.artifacts.map((a, i) => (
                    <div key={i} className="rounded border border-zyl-border bg-zyl-bg p-2">
                      <div className="text-xs font-mono text-zyl-muted">
                        {a.kind} · {a.label}
                      </div>
                      <pre className="mt-1 text-xs overflow-auto max-h-40 whitespace-pre-wrap break-words">{a.content.slice(0, 800)}</pre>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-sm text-zyl-muted">Artifacts will appear as streamed tabs (main.rs, mcp.tools.yaml, config.json) — Monaco wired in 3.2.</p>
              )}
              <div className="mt-3 flex gap-2">
                <button
                  disabled={!intentResult}
                  className="rounded bg-zyl-accent px-3 py-1 text-xs font-semibold text-white disabled:opacity-40"
                  onClick={() => invoke("save_workspace_artifact", { label: intentResult?.artifacts[0]?.label ?? "out", content: intentResult?.artifacts[0]?.content ?? "" }).catch((e) => setError(String(e)))}
                  title="Phase 3.2 IPC"
                >
                  Save artifact
                </button>
                <button
                  disabled={!intentResult}
                  className="rounded border border-zyl-border px-3 py-1 text-xs font-semibold disabled:opacity-40"
                  onClick={() => invoke("apply_patch", { patch: intentResult?.artifacts[0]?.content ?? "" }).catch((e) => setError(String(e)))}
                  title="Phase 3.2 IPC"
                >
                  Apply patch
                </button>
              </div>
            </section>

            {/* Verify */}
            <section className="rounded-lg border border-zyl-border bg-zyl-surface p-5">
              <div className="mb-3 flex items-center justify-between">
                <h2 className="text-sm font-semibold uppercase tracking-widest text-zyl-muted">Verify Logic</h2>
                <button
                  onClick={handleVerify}
                  disabled={busy}
                  className="rounded-md border border-zyl-border px-3 py-1 text-xs font-semibold hover:bg-zyl-bg disabled:opacity-40"
                >
                  Run verification
                </button>
              </div>
              {verification && (
                <div className="space-y-2">
                  <p className={`text-sm font-semibold ${verification.passed ? "text-green-400" : "text-red-400"}`}>
                    {verification.passed ? "PASSED" : "FAILED"} — {verification.duration_ms} ms
                  </p>
                  <ul className="space-y-1 font-mono text-xs">
                    {verification.checks.map((c) => (
                      <li key={c.name} className={c.passed ? "text-zyl-muted" : "text-red-300"}>
                        {c.passed ? "✓" : "✗"} {c.name}: {c.message}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </section>

            {error && (
              <div className="rounded-md border border-red-900 bg-red-950/50 p-3 font-mono text-xs text-red-300">{error}</div>
            )}
          </div>
        </main>
      </div>
    </div>
  );
}
