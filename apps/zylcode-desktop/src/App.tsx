import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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

export default function App() {
  const [prompt, setPrompt] = useState("");
  const [intentResult, setIntentResult] = useState<IntentResult | null>(null);
  const [verification, setVerification] = useState<VerificationReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleProcessIntent() {
    if (!prompt.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const result = await invoke<IntentResult>("process_intent", { prompt });
      setIntentResult(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
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
    <div className="min-h-screen bg-zyl-bg text-white">
      <header className="border-b border-zyl-border bg-zyl-surface px-6 py-4">
        <h1 className="text-xl font-bold tracking-tight">
          ZylCode <span className="font-mono text-sm font-normal text-zyl-muted">desktop</span>
        </h1>
        <p className="text-sm text-zyl-muted">
          Autonomous developer engine — intent processing, logic verification &amp; live artifact
          preview
        </p>
      </header>

      <main className="mx-auto max-w-3xl space-y-6 p-6">
        {/* Intent */}
        <section className="rounded-lg border border-zyl-border bg-zyl-surface p-5">
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-widest text-zyl-muted">
            Process Intent
          </h2>
          <div className="flex gap-3">
            <input
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleProcessIntent()}
              placeholder="Describe what you want to build…"
              className="flex-1 rounded-md border border-zyl-border bg-zyl-bg px-3 py-2 text-sm outline-none focus:border-zyl-accent"
            />
            <button
              onClick={handleProcessIntent}
              disabled={busy || !prompt.trim()}
              className="rounded-md bg-zyl-accent px-4 py-2 text-sm font-semibold text-white hover:opacity-90 disabled:opacity-40"
            >
              {busy ? "…" : "Run"}
            </button>
          </div>
          {intentResult && (
            <div className="mt-4 rounded-md bg-zyl-bg p-3 font-mono text-xs">
              <p className="text-white">{intentResult.summary}</p>
              {intentResult.artifacts.length > 0 && (
                <ul className="mt-2 space-y-1 text-zyl-muted">
                  {intentResult.artifacts.map((a, i) => (
                    <li key={i}>
                      [{a.kind}] {a.label}: {a.content}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </section>

        {/* Verify */}
        <section className="rounded-lg border border-zyl-border bg-zyl-surface p-5">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-sm font-semibold uppercase tracking-widest text-zyl-muted">
              Verify Logic
            </h2>
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
              <p
                className={`text-sm font-semibold ${verification.passed ? "text-green-400" : "text-red-400"}`}
              >
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
          <div className="rounded-md border border-red-900 bg-red-950/50 p-3 font-mono text-xs text-red-300">
            {error}
          </div>
        )}
      </main>
    </div>
  );
}
