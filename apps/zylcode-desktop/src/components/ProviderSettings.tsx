import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type ProviderKind = "anthropic" | "open_router" | "ollama" | "synthetic_offline";

type ProviderConfig = {
  kind: ProviderKind;
  model: string;
  endpoint: string;
  timeout_ms: number;
  enabled: boolean;
  fallback_order: number;
  requires_api_key: boolean;
};

type Failover = {
  timestamp: string;
  from_provider: string;
  to_provider: string;
  reason: string;
  attempt_number: number;
};

const KIND_LABEL: Record<ProviderKind, string> = {
  anthropic: "Anthropic Native",
  open_router: "OpenRouter API",
  ollama: "Ollama Local",
  synthetic_offline: "Synthetic Offline",
};

const KIND_DOT: Record<ProviderKind, string> = {
  anthropic: "bg-violet-500",
  open_router: "bg-sky-500",
  ollama: "bg-emerald-500",
  synthetic_offline: "bg-zinc-500",
};

export default function ProviderSettings() {
  const [configs, setConfigs] = useState<ProviderConfig[]>([]);
  const [failovers, setFailovers] = useState<Failover[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [edit, setEdit] = useState<Record<string, { endpoint: string; timeout_ms: string; model: string }>>({});

  async function refresh() {
    try {
      const res = await invoke<ProviderConfig[]>("get_provider_configs");
      res.sort((a, b) => a.fallback_order - b.fallback_order);
      setConfigs(res);
      const draft: Record<string, { endpoint: string; timeout_ms: string; model: string }> = {};
      for (const c of res) draft[c.kind] = { endpoint: c.endpoint, timeout_ms: String(c.timeout_ms), model: c.model };
      setEdit(draft);
      setErr(null);
    } catch (e) {
      setErr(String(e));
    }
  }

  useEffect(() => {
    refresh();
    let unlisten: (() => void) | undefined;
    (async () => {
      try {
        const fn = await listen<Failover>("telemetry:provider_failover", (ev) => {
          setFailovers((prev) => [ev.payload, ...prev].slice(0, 20));
        });
        unlisten = fn;
      } catch {}
    })();
    return () => unlisten?.();
  }, []);

  async function handleSet(kind: ProviderKind) {
    const e = edit[kind];
    if (!e) return;
    setBusy(kind);
    setErr(null);
    try {
      const timeout = e.timeout_ms.trim() ? Number(e.timeout_ms) : undefined;
      if (timeout !== undefined && (isNaN(timeout) || timeout < 1000 || timeout > 300000)) {
        throw new Error("timeout_ms must be 1000..300000");
      }
      const next = await invoke<ProviderConfig[]>("set_provider_config", {
        kind,
        endpoint: e.endpoint.trim() || null,
        timeoutMs: timeout ?? null,
        enabled: null,
        model: e.model.trim() || null,
      });
      next.sort((a, b) => a.fallback_order - b.fallback_order);
      setConfigs(next);
    } catch (ex) {
      setErr(String(ex));
    } finally {
      setBusy(null);
    }
  }

  async function toggleEnabled(kind: ProviderKind, enabled: boolean) {
    setBusy(kind);
    try {
      const next = await invoke<ProviderConfig[]>("set_provider_config", {
        kind,
        endpoint: null,
        timeoutMs: null,
        enabled,
        model: null,
      });
      next.sort((a, b) => a.fallback_order - b.fallback_order);
      setConfigs(next);
    } catch (ex) {
      setErr(String(ex));
    } finally {
      setBusy(null);
    }
  }

  async function move(kind: ProviderKind, dir: -1 | 1) {
    const sorted = [...configs].sort((a, b) => a.fallback_order - b.fallback_order);
    const idx = sorted.findIndex((c) => c.kind === kind);
    const nxt = idx + dir;
    if (nxt < 0 || nxt >= sorted.length) return;
    const order = sorted.map((c) => c.kind);
    [order[idx], order[nxt]] = [order[nxt], order[idx]];
    setBusy("reorder");
    try {
      const res = await invoke<ProviderConfig[]>("reorder_provider_chain", { order });
      res.sort((a, b) => a.fallback_order - b.fallback_order);
      setConfigs(res);
    } catch (ex) {
      setErr(String(ex));
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="rounded-lg border border-zyl-border bg-zyl-surface p-4 space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-widest text-zyl-muted">Provider Settings — Fallback Chain</h2>
        <button onClick={refresh} className="rounded border border-zyl-border px-2 py-1 text-xs hover:bg-zyl-bg">
          Refresh
        </button>
      </div>
      {err && <div className="rounded border border-red-900 bg-red-950/40 p-2 text-xs font-mono text-red-300">{err}</div>}

      <div className="grid gap-3">
        {configs.map((c) => (
          <div key={c.kind} className="rounded border border-zyl-border bg-zyl-bg p-3 flex flex-col gap-2">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2">
                <span className={`inline-block h-2 w-2 rounded-full ${KIND_DOT[c.kind]} ${c.enabled ? "opacity-100" : "opacity-30"}`} />
                <span className="text-sm font-semibold">{KIND_LABEL[c.kind]}</span>
                <span className="text-xs font-mono text-zyl-muted">#{c.fallback_order} · {c.kind}</span>
                {c.requires_api_key && <span className="rounded bg-amber-950/50 border border-amber-900 px-1.5 py-0.5 text-[10px] text-amber-300">API key</span>}
                {!c.enabled && <span className="rounded bg-zinc-800 border border-zinc-700 px-1.5 py-0.5 text-[10px] text-zinc-400">disabled</span>}
              </div>
              <div className="flex items-center gap-1">
                <button
                  disabled={busy !== null}
                  onClick={() => move(c.kind, -1)}
                  className="rounded border border-zyl-border px-2 py-1 text-xs hover:bg-zyl-surface disabled:opacity-40"
                  title="Move up (higher priority)"
                >
                  ↑
                </button>
                <button
                  disabled={busy !== null}
                  onClick={() => move(c.kind, 1)}
                  className="rounded border border-zyl-border px-2 py-1 text-xs hover:bg-zyl-surface disabled:opacity-40"
                  title="Move down (lower priority)"
                >
                  ↓
                </button>
                <label className="ml-2 flex items-center gap-1 text-xs">
                  <input type="checkbox" checked={c.enabled} onChange={(e) => toggleEnabled(c.kind, e.target.checked)} disabled={busy === c.kind} />
                  enabled
                </label>
              </div>
            </div>

            <div className="grid grid-cols-1 lg:grid-cols-[1.2fr_0.8fr_0.7fr] gap-2">
              <label className="text-xs space-y-1">
                <span className="text-zyl-muted">Model</span>
                <input
                  value={edit[c.kind]?.model ?? ""}
                  onChange={(e) => setEdit((s) => ({ ...s, [c.kind]: { ...s[c.kind], model: e.target.value } }))}
                  placeholder={c.model}
                  className="w-full rounded border border-zyl-border bg-zyl-surface px-2 py-1.5 font-mono text-xs outline-none focus:border-zyl-accent"
                />
              </label>
              <label className="text-xs space-y-1">
                <span className="text-zyl-muted">Endpoint override</span>
                <input
                  value={edit[c.kind]?.endpoint ?? ""}
                  onChange={(e) => setEdit((s) => ({ ...s, [c.kind]: { ...s[c.kind], endpoint: e.target.value } }))}
                  placeholder={c.kind === "ollama" ? "http://localhost:11434" : "default"}
                  className="w-full rounded border border-zyl-border bg-zyl-surface px-2 py-1.5 font-mono text-xs outline-none focus:border-zyl-accent"
                />
              </label>
              <label className="text-xs space-y-1">
                <span className="text-zyl-muted">Timeout ms</span>
                <input
                  value={edit[c.kind]?.timeout_ms ?? ""}
                  onChange={(e) => setEdit((s) => ({ ...s, [c.kind]: { ...s[c.kind], timeout_ms: e.target.value } }))}
                  placeholder={String(c.timeout_ms)}
                  className="w-full rounded border border-zyl-border bg-zyl-surface px-2 py-1.5 font-mono text-xs outline-none focus:border-zyl-accent"
                />
              </label>
            </div>

            <div className="flex justify-end">
              <button
                onClick={() => handleSet(c.kind)}
                disabled={busy === c.kind}
                className="rounded bg-zyl-accent px-3 py-1.5 text-xs font-semibold text-white hover:opacity-90 disabled:opacity-40"
              >
                {busy === c.kind ? "Saving…" : "Save"}
              </button>
            </div>
          </div>
        ))}
      </div>

      <div className="rounded border border-zyl-border bg-zyl-bg p-3">
        <div className="text-xs font-semibold uppercase tracking-widest text-zyl-muted mb-2">Live failover indicators</div>
        {failovers.length === 0 ? (
          <p className="text-xs text-zyl-muted">No failovers yet — streaming intents will emit <span className="font-mono">telemetry:provider_failover</span> when fallback occurs.</p>
        ) : (
          <ul className="space-y-1 max-h-40 overflow-auto pr-1">
            {failovers.map((f, i) => (
              <li key={i} className="flex items-center gap-2 text-xs font-mono">
                <span className="text-amber-300">↻ #{f.attempt_number}</span>
                <span className="text-zyl-muted">{f.from_provider}</span>
                <span>→</span>
                <span className="text-sky-300">{f.to_provider}</span>
                <span className="text-zyl-muted truncate">{f.reason}</span>
                <span className="ml-auto text-[10px] text-zyl-muted">{f.timestamp}</span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <p className="text-[11px] leading-relaxed text-zyl-muted">
        Order defines fallback priority (top = first attempt). Timeout is per-request (1000–300000ms). Ollama default host is <span className="font-mono">http://localhost:11434</span>. Synthetic offline requires no key and is last-resort.
      </p>
    </div>
  );
}
