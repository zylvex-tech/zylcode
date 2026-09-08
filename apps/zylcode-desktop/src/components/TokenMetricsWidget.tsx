import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type TokenSnapshot = {
  input_tokens: number;
  output_tokens: number;
  verification_saved_tokens: number;
  fallback_count: number;
};

export default function TokenMetricsWidget({ refreshKey }: { refreshKey?: number }) {
  const [snap, setSnap] = useState<TokenSnapshot | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<TokenSnapshot>("token_metrics")
      .then((v) => {
        if (!cancelled) setSnap(v);
      })
      .catch((e) => {
        if (!cancelled) setErr(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  if (err) {
    return <span className="text-xs text-red-400" title={err}>metrics: error</span>;
  }
  if (!snap) {
    return <span className="text-xs text-zyl-muted">metrics…</span>;
  }

  const hit = snap.verification_saved_tokens > 0;
  const total = snap.input_tokens + snap.output_tokens;

  // Cost estimate using Anthropic pricing ($3/$15 per 1M tokens)
  const inputCost = (snap.input_tokens / 1_000_000) * 3.0;
  const outputCost = (snap.output_tokens / 1_000_000) * 15.0;
  const totalCost = inputCost + outputCost;
  const savedCost = (snap.verification_saved_tokens / 1_000_000) * 3.0;

  const formatCost = (v: number) =>
    v < 0.01 ? "<$0.01" : `$${v.toFixed(2)}`;

  return (
    <div className="flex flex-wrap items-center gap-2 text-xs font-mono">
      <span className="rounded bg-zyl-bg px-2 py-0.5 border border-zyl-border" title="input / output tokens">
        tok {snap.input_tokens}/{snap.output_tokens}
        <span className="text-zyl-muted"> Σ{total}</span>
      </span>
      <span
        className={`rounded px-2 py-0.5 border ${hit ? "bg-emerald-950/60 border-emerald-800 text-emerald-300" : "bg-zyl-bg border-zyl-border text-zyl-muted"}`}
        title="speculative cache: tokens saved by avoiding redundant LLM roundtrips"
      >
        cache {hit ? "HIT" : "—"} +{snap.verification_saved_tokens} saved
      </span>
      <span
        className="rounded bg-zyl-bg px-2 py-0.5 border border-zyl-border text-zyl-muted"
        title="estimated cost (Anthropic pricing: $3/$15 per 1M in/out)"
      >
        cost {formatCost(totalCost)}
        {savedCost > 0 && (
          <span className="text-emerald-400 ml-1">saved {formatCost(savedCost)}</span>
        )}
      </span>
      {snap.fallback_count > 0 && (
        <span className="rounded bg-amber-950/50 border border-amber-900 text-amber-300 px-2 py-0.5" title="fallback provider activations">
          fallback ×{snap.fallback_count}
        </span>
      )}
      {/* context trim indicator — derived from speculative savings vs total */}
      <span className="text-zyl-muted" title="adaptive context window (8192 tokens, memchr trimmed)">
        win 8192
      </span>
    </div>
  );
}
