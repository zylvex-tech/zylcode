import { useEffect, useState } from "react";
import { safeInvoke, unwrapInvoke } from "../lib/runtime";
import { ENVIRONMENT, safeListen } from "../lib/events";

type ProviderKind =
  "anthropic" | "open_router" | "open_ai" | "deep_seek" | "ollama" | "custom" | "synthetic_offline";

type ProviderConfig = {
  kind: ProviderKind;
  model: string;
  endpoint: string;
  timeout_ms: number;
  enabled: boolean;
  fallback_order: number;
  requires_api_key: boolean;
};

/**
 * Where a provider's credential comes from.
 *
 * This is a *source*, never a value. `hint` is four trailing characters of a
 * key held by the OS credential store, so two stored keys can be told apart
 * without either being rendered.
 */
type KeyStatus = {
  kind: ProviderKind;
  provider: string | null;
  requires_key: boolean;
  configured: boolean;
  source: "credential_store" | "environment" | "config" | "none" | "not_required";
  env_var: string | null;
  hint: string | null;
  store_error: string | null;
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
  open_ai: "OpenAI",
  open_router: "OpenRouter API",
  deep_seek: "DeepSeek",
  ollama: "Ollama Local",
  custom: "Custom (OpenAI-compatible)",
  synthetic_offline: "Synthetic Offline",
};

const KIND_DOT: Record<ProviderKind, string> = {
  anthropic: "bg-violet-500",
  open_ai: "bg-teal-500",
  open_router: "bg-sky-500",
  deep_seek: "bg-indigo-500",
  ollama: "bg-emerald-500",
  custom: "bg-orange-500",
  synthetic_offline: "bg-zinc-500",
};

/** Endpoint placeholder per provider: the value dispatch would use by default. */
const ENDPOINT_PLACEHOLDER: Record<ProviderKind, string> = {
  anthropic: "https://api.anthropic.com",
  open_ai: "https://api.openai.com/v1",
  open_router: "https://openrouter.ai/api/v1",
  deep_seek: "https://api.deepseek.com/v1",
  ollama: "http://localhost:11434",
  custom: "required — e.g. http://gateway.internal/v1",
  synthetic_offline: "no endpoint",
};

/**
 * Result of one real probe (`test_provider_connection`).
 *
 * `status` is the classification the backend produced; it is the only thing
 * this panel is allowed to call "working". A configured provider with a key is
 * not "ready" — only `connected` is.
 */
type Probe = {
  provider: string;
  model: string;
  endpoint: string;
  key_source: string;
  status: ProbeStatus;
  http_status?: number | null;
  latency_ms: number;
  response_chars: number;
  detail: string;
};

type ProbeStatus =
  | "connected"
  | "offline_provider"
  | "configuration_incomplete"
  | "authentication_failed"
  | "invalid_endpoint"
  | "model_unavailable"
  | "provider_rejected"
  | "network_error";

const PROBE_LABEL: Record<ProbeStatus, string> = {
  connected: "Connected",
  offline_provider: "Offline provider (no request sent)",
  configuration_incomplete: "Configuration incomplete",
  authentication_failed: "Authentication failed",
  invalid_endpoint: "Invalid endpoint",
  model_unavailable: "Model unavailable",
  provider_rejected: "Provider rejected request",
  network_error: "Network error",
};

const PROBE_STYLE: Record<ProbeStatus, string> = {
  connected: "border-emerald-800 bg-emerald-950/40 text-emerald-300",
  offline_provider: "border-zinc-700 bg-zinc-900 text-zinc-300",
  configuration_incomplete: "border-amber-900 bg-amber-950/40 text-amber-300",
  authentication_failed: "border-red-900 bg-red-950/40 text-red-300",
  invalid_endpoint: "border-red-900 bg-red-950/40 text-red-300",
  model_unavailable: "border-red-900 bg-red-950/40 text-red-300",
  provider_rejected: "border-red-900 bg-red-950/40 text-red-300",
  network_error: "border-red-900 bg-red-950/40 text-red-300",
};

const KEY_SOURCE_LABEL: Record<string, string> = {
  config: "saved key",
  credential_store: "OS credential store",
  environment: "environment variable",
  not_required: "no key required",
  none: "no key found",
};

function keySourceLabel(status: KeyStatus | undefined, probeKeySource: string | undefined) {
  if (probeKeySource && probeKeySource.startsWith("env:")) return probeKeySource;
  if (status) {
    const base = KEY_SOURCE_LABEL[status.source] ?? status.source;
    return status.source === "environment" && status.env_var ? `${base} (${status.env_var})` : base;
  }
  return KEY_SOURCE_LABEL[probeKeySource ?? ""] ?? probeKeySource ?? "unknown";
}

type Edit = { endpoint: string; timeout_ms: string; model: string };

export default function ProviderSettings() {
  const [configs, setConfigs] = useState<ProviderConfig[]>([]);
  const [keys, setKeys] = useState<KeyStatus[]>([]);
  const [failovers, setFailovers] = useState<Failover[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [keyBusy, setKeyBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [probing, setProbing] = useState<string | null>(null);
  const [probes, setProbes] = useState<Record<string, Probe>>({});
  const [edit, setEdit] = useState<Record<string, Edit>>({});
  const [keyDraft, setKeyDraft] = useState<Record<string, string>>({});

  function applyConfigs(res: ProviderConfig[]) {
    res.sort((a, b) => a.fallback_order - b.fallback_order);
    setConfigs(res);
    const draft: Record<string, Edit> = {};
    for (const c of res)
      draft[c.kind] = { endpoint: c.endpoint, timeout_ms: String(c.timeout_ms), model: c.model };
    setEdit(draft);
  }

  function applyKeys(res: KeyStatus[]) {
    setKeys(res);
  }

  async function refresh() {
    try {
      const res = unwrapInvoke(
        await safeInvoke<ProviderConfig[]>(ENVIRONMENT, "get_provider_configs"),
      );
      applyConfigs(res);
      try {
        applyKeys(unwrapInvoke(await safeInvoke<KeyStatus[]>(ENVIRONMENT, "provider_key_status")));
      } catch (keyErr) {
        // The chain is still usable when credential status is unavailable; say
        // so instead of blanking the panel.
        setErr(String(keyErr));
      }
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
        const fn = await safeListen<Failover>("telemetry:provider_failover", (failover) => {
          setFailovers((prev) => [failover, ...prev].slice(0, 20));
        });
        unlisten = fn;
      } catch {
        // No listener available in this runtime. Deliberately swallowed: the
        // settings surface still works without the live failover feed.
      }
    })();
    return () => unlisten?.();
  }, []);

  function isDirty(c: ProviderConfig) {
    const e = edit[c.kind];
    if (!e) return false;
    return (
      e.model !== c.model || e.endpoint !== c.endpoint || e.timeout_ms !== String(c.timeout_ms)
    );
  }

  async function handleSet(kind: ProviderKind) {
    const e = edit[kind];
    if (!e) return;
    setBusy(kind);
    setErr(null);
    setNotice(null);
    try {
      const timeout = e.timeout_ms.trim() ? Number(e.timeout_ms) : undefined;
      if (timeout !== undefined && (isNaN(timeout) || timeout < 1000 || timeout > 300000)) {
        throw new Error("timeout_ms must be 1000..300000");
      }
      const next = unwrapInvoke(
        await safeInvoke<ProviderConfig[]>(ENVIRONMENT, "set_provider_config", {
          kind,
          // Sent as strings, not null: an emptied field is how a model or
          // endpoint override is removed. `null` means "leave unchanged" and is
          // used only by the enable toggle.
          endpoint: e.endpoint.trim(),
          timeoutMs: timeout ?? null,
          enabled: null,
          model: e.model.trim(),
        }),
      );
      applyConfigs(next);
      setNotice(
        `${KIND_LABEL[kind]} saved — the next request uses these values, and they survive a restart.`,
      );
    } catch (ex) {
      setErr(String(ex));
    } finally {
      setBusy(null);
    }
  }

  async function toggleEnabled(kind: ProviderKind, enabled: boolean) {
    setBusy(kind);
    setErr(null);
    try {
      const next = unwrapInvoke(
        await safeInvoke<ProviderConfig[]>(ENVIRONMENT, "set_provider_config", {
          kind,
          endpoint: null,
          timeoutMs: null,
          enabled,
          model: null,
        }),
      );
      applyConfigs(next);
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
    setErr(null);
    try {
      const res = unwrapInvoke(
        await safeInvoke<ProviderConfig[]>(ENVIRONMENT, "reorder_provider_chain", { order }),
      );
      applyConfigs(res);
      setNotice("Order saved — it is the order dispatch will attempt.");
    } catch (ex) {
      setErr(String(ex));
    } finally {
      setBusy(null);
    }
  }

  /**
   * Probe the saved configuration: exactly what the next real request will use.
   */
  async function testSaved(kind: ProviderKind) {
    setProbing(kind);
    setErr(null);
    try {
      const probe = unwrapInvoke(
        await safeInvoke<Probe>(ENVIRONMENT, "test_provider_connection", {
          kind,
          endpoint: null,
          model: null,
          timeoutMs: null,
          saved: true,
        }),
      );
      setProbes((p) => ({ ...p, [`${kind}:saved`]: probe }));
    } catch (ex) {
      setErr(String(ex));
    } finally {
      setProbing(null);
    }
  }

  /**
   * Probe the values currently in the form, so a configuration can be checked
   * before it is saved. The result is labelled as unsaved.
   */
  async function testDraft(kind: ProviderKind) {
    const e = edit[kind];
    setProbing(kind);
    setErr(null);
    try {
      const timeout = e?.timeout_ms.trim() ? Number(e.timeout_ms) : null;
      if (timeout !== null && (isNaN(timeout) || timeout < 1000 || timeout > 300000)) {
        throw new Error("timeout_ms must be 1000..300000");
      }
      const probe = unwrapInvoke(
        await safeInvoke<Probe>(ENVIRONMENT, "test_provider_connection", {
          kind,
          endpoint: e?.endpoint.trim() || null,
          model: e?.model.trim() || null,
          timeoutMs: timeout,
          saved: false,
        }),
      );
      setProbes((p) => ({ ...p, [`${kind}:draft`]: probe }));
    } catch (ex) {
      setErr(String(ex));
    } finally {
      setProbing(null);
    }
  }

  async function saveKey(kind: ProviderKind) {
    const value = (keyDraft[kind] ?? "").trim();
    if (!value) return;
    setKeyBusy(kind);
    setErr(null);
    setNotice(null);
    try {
      const res = unwrapInvoke(
        await safeInvoke<KeyStatus[]>(ENVIRONMENT, "set_provider_key", {
          kind,
          apiKey: value,
        }),
      );
      applyKeys(res);
      // The key is held by the OS now; drop the field so it is not re-submitted
      // or left visible in the DOM.
      setKeyDraft((s) => ({ ...s, [kind]: "" }));
      setNotice(`${KIND_LABEL[kind]} key stored in the OS credential store.`);
    } catch (ex) {
      setErr(String(ex));
    } finally {
      setKeyBusy(null);
    }
  }

  async function removeKey(kind: ProviderKind) {
    setKeyBusy(kind);
    setErr(null);
    setNotice(null);
    try {
      const res = unwrapInvoke(
        await safeInvoke<KeyStatus[]>(ENVIRONMENT, "delete_provider_key", { kind }),
      );
      applyKeys(res);
    } catch (ex) {
      setErr(String(ex));
    } finally {
      setKeyBusy(null);
    }
  }

  function keyStatus(kind: ProviderKind) {
    return keys.find((k) => k.kind === kind);
  }

  return (
    <div className="rounded-lg border border-zyl-border bg-zyl-surface p-4 space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-widest text-zyl-muted">
          Provider Settings — Fallback Chain
        </h2>
        <button
          onClick={refresh}
          className="rounded border border-zyl-border px-2 py-1 text-xs hover:bg-zyl-bg"
        >
          Refresh
        </button>
      </div>
      {err && (
        <div className="rounded border border-red-900 bg-red-950/40 p-2 text-xs font-mono text-red-300">
          {err}
        </div>
      )}
      {notice && (
        <div className="rounded border border-emerald-900 bg-emerald-950/30 p-2 text-xs font-mono text-emerald-300">
          {notice}
        </div>
      )}

      <div className="grid gap-3">
        {configs.map((c) => {
          const status = keyStatus(c.kind);
          const draftProbe = probes[`${c.kind}:draft`];
          const savedProbe = probes[`${c.kind}:saved`];
          return (
            <div
              key={c.kind}
              className="rounded border border-zyl-border bg-zyl-bg p-3 flex flex-col gap-2"
            >
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <span
                    className={`inline-block h-2 w-2 rounded-full ${KIND_DOT[c.kind]} ${c.enabled ? "opacity-100" : "opacity-30"}`}
                  />
                  <span className="text-sm font-semibold">{KIND_LABEL[c.kind]}</span>
                  <span className="text-xs font-mono text-zyl-muted">
                    #{c.fallback_order} · {c.kind}
                  </span>
                  {c.requires_api_key && (
                    <span
                      className={`rounded border px-1.5 py-0.5 text-[10px] ${
                        status?.configured
                          ? "border-emerald-900 bg-emerald-950/40 text-emerald-300"
                          : "border-amber-900 bg-amber-950/50 text-amber-300"
                      }`}
                      data-testid={`provider-key-source-${c.kind}`}
                      title={status?.store_error ?? undefined}
                    >
                      {status?.configured
                        ? `key: ${keySourceLabel(status, undefined)}${status.hint ? ` ${status.hint}` : ""}`
                        : "key: not configured"}
                    </span>
                  )}
                  {!c.enabled && (
                    <span className="rounded bg-zinc-800 border border-zinc-700 px-1.5 py-0.5 text-[10px] text-zinc-400">
                      disabled
                    </span>
                  )}
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
                    <input
                      type="checkbox"
                      checked={c.enabled}
                      onChange={(e) => toggleEnabled(c.kind, e.target.checked)}
                      disabled={busy === c.kind}
                    />
                    enabled
                  </label>
                </div>
              </div>

              <div className="grid grid-cols-1 lg:grid-cols-[1.2fr_0.8fr_0.7fr] gap-2">
                <label className="text-xs space-y-1">
                  <span className="text-zyl-muted">Model</span>
                  <input
                    value={edit[c.kind]?.model ?? ""}
                    onChange={(e) =>
                      setEdit((s) => ({ ...s, [c.kind]: { ...s[c.kind], model: e.target.value } }))
                    }
                    placeholder={c.model || "required"}
                    className="w-full rounded border border-zyl-border bg-zyl-surface px-2 py-1.5 font-mono text-xs outline-none focus:border-zyl-accent"
                  />
                </label>
                <label className="text-xs space-y-1">
                  <span className="text-zyl-muted">Endpoint override</span>
                  <input
                    value={edit[c.kind]?.endpoint ?? ""}
                    onChange={(e) =>
                      setEdit((s) => ({
                        ...s,
                        [c.kind]: { ...s[c.kind], endpoint: e.target.value },
                      }))
                    }
                    placeholder={ENDPOINT_PLACEHOLDER[c.kind]}
                    className="w-full rounded border border-zyl-border bg-zyl-surface px-2 py-1.5 font-mono text-xs outline-none focus:border-zyl-accent"
                  />
                </label>
                <label className="text-xs space-y-1">
                  <span className="text-zyl-muted">Timeout ms</span>
                  <input
                    value={edit[c.kind]?.timeout_ms ?? ""}
                    onChange={(e) =>
                      setEdit((s) => ({
                        ...s,
                        [c.kind]: { ...s[c.kind], timeout_ms: e.target.value },
                      }))
                    }
                    placeholder={String(c.timeout_ms)}
                    className="w-full rounded border border-zyl-border bg-zyl-surface px-2 py-1.5 font-mono text-xs outline-none focus:border-zyl-accent"
                  />
                </label>
              </div>

              {c.requires_api_key && (
                <div className="flex flex-wrap items-end gap-2 rounded border border-zyl-border/60 bg-zyl-surface/40 p-2">
                  <label className="flex-1 text-xs space-y-1">
                    <span className="text-zyl-muted">
                      API key — stored in the OS credential store, never in the workspace
                      {status?.env_var ? (
                        <>
                          {" "}
                          · env fallback <span className="font-mono">{status.env_var}</span>
                        </>
                      ) : null}
                    </span>
                    <input
                      type="password"
                      autoComplete="off"
                      spellCheck={false}
                      value={keyDraft[c.kind] ?? ""}
                      onChange={(e) => setKeyDraft((s) => ({ ...s, [c.kind]: e.target.value }))}
                      placeholder={
                        status?.source === "credential_store"
                          ? `replace stored key ${status.hint ?? ""}`.trim()
                          : "paste key"
                      }
                      className="w-full rounded border border-zyl-border bg-zyl-bg px-2 py-1.5 font-mono text-xs outline-none focus:border-zyl-accent"
                    />
                  </label>
                  <button
                    onClick={() => saveKey(c.kind)}
                    disabled={keyBusy === c.kind || !(keyDraft[c.kind] ?? "").trim()}
                    className="rounded border border-zyl-border px-3 py-1.5 text-xs font-semibold hover:bg-zyl-surface disabled:opacity-40"
                  >
                    {keyBusy === c.kind
                      ? "Storing…"
                      : status?.source === "credential_store"
                        ? "Replace key"
                        : "Store key"}
                  </button>
                  <button
                    onClick={() => removeKey(c.kind)}
                    disabled={keyBusy === c.kind || status?.source !== "credential_store"}
                    className="rounded border border-zyl-border px-2 py-1.5 text-xs hover:bg-zyl-surface disabled:opacity-40"
                    title="Remove the key from the OS credential store"
                  >
                    Remove
                  </button>
                </div>
              )}

              <div className="flex flex-wrap items-center justify-end gap-2">
                <button
                  onClick={() => testSaved(c.kind)}
                  disabled={probing !== null}
                  className="rounded border border-zyl-border px-3 py-1.5 text-xs font-semibold hover:bg-zyl-surface disabled:opacity-40"
                  title="Send one real request with the saved configuration — the same values the next agent turn will use"
                >
                  {probing === c.kind ? "Testing…" : "Test saved"}
                </button>
                <button
                  onClick={() => testDraft(c.kind)}
                  disabled={probing !== null}
                  className="rounded border border-zyl-border px-3 py-1.5 text-xs hover:bg-zyl-surface disabled:opacity-40"
                  title="Send one real request with the values in the form, without saving them"
                >
                  {probing === c.kind ? "Testing…" : "Test unsaved values"}
                </button>
                <button
                  onClick={() => handleSet(c.kind)}
                  disabled={busy === c.kind}
                  className="rounded bg-zyl-accent px-3 py-1.5 text-xs font-semibold text-white hover:opacity-90 disabled:opacity-40"
                >
                  {busy === c.kind ? "Saving…" : "Save"}
                </button>
              </div>

              {savedProbe && (
                <div
                  className={`rounded border p-2 text-xs ${PROBE_STYLE[savedProbe.status]}`}
                  data-testid={`provider-probe-saved-${c.kind}`}
                >
                  <div className="flex flex-wrap items-center gap-2 font-semibold">
                    <span>Saved configuration · {PROBE_LABEL[savedProbe.status]}</span>
                    {savedProbe.http_status != null && (
                      <span className="font-mono text-[10px] opacity-80">
                        HTTP {savedProbe.http_status}
                      </span>
                    )}
                    {savedProbe.status === "connected" && (
                      <span className="font-mono text-[10px] opacity-80">
                        {savedProbe.latency_ms}ms · {savedProbe.response_chars} chars
                      </span>
                    )}
                  </div>
                  <div className="mt-1 font-mono text-[10px] opacity-80 break-all">
                    {savedProbe.endpoint}
                  </div>
                  <div className="mt-1 font-mono text-[10px] opacity-80">
                    model {savedProbe.model} · key {keySourceLabel(status, savedProbe.key_source)}
                  </div>
                  {savedProbe.detail && (
                    <div className="mt-1 break-words text-[10px] opacity-90">
                      {savedProbe.detail}
                    </div>
                  )}
                </div>
              )}

              {draftProbe && (
                <div
                  className={`rounded border p-2 text-xs ${PROBE_STYLE[draftProbe.status]}`}
                  data-testid={`provider-probe-draft-${c.kind}`}
                >
                  <div className="flex flex-wrap items-center gap-2 font-semibold">
                    <span>Unsaved values · {PROBE_LABEL[draftProbe.status]}</span>
                    {draftProbe.http_status != null && (
                      <span className="font-mono text-[10px] opacity-80">
                        HTTP {draftProbe.http_status}
                      </span>
                    )}
                    {draftProbe.status === "connected" && (
                      <span className="font-mono text-[10px] opacity-80">
                        {draftProbe.latency_ms}ms · {draftProbe.response_chars} chars
                      </span>
                    )}
                  </div>
                  <div className="mt-1 font-mono text-[10px] opacity-80 break-all">
                    {draftProbe.endpoint}
                  </div>
                  <div className="mt-1 font-mono text-[10px] opacity-80">
                    model {draftProbe.model} · key {keySourceLabel(status, draftProbe.key_source)}
                  </div>
                  {draftProbe.detail && (
                    <div className="mt-1 break-words text-[10px] opacity-90">
                      {draftProbe.detail}
                    </div>
                  )}
                  {isDirty(c) && (
                    <div className="mt-1 text-[10px] font-semibold opacity-90">
                      These values are not saved — the next request still uses the saved
                      configuration until you press Save.
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>

      <div className="rounded border border-zyl-border bg-zyl-bg p-3">
        <div className="text-xs font-semibold uppercase tracking-widest text-zyl-muted mb-2">
          Live failover indicators
        </div>
        {failovers.length === 0 ? (
          <p className="text-xs text-zyl-muted">
            No failovers yet — streaming intents will emit{" "}
            <span className="font-mono">telemetry:provider_failover</span> when fallback occurs.
          </p>
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
        Order defines fallback priority (top = first attempt) and is the order dispatch attempts
        after a restart. Timeout is per provider (1000–300000ms). Disabled providers are skipped;
        adding one does not change what an existing configuration tries first. API keys are kept in
        the OS credential store (never in the workspace or this panel) and are shown only as a
        masked hint and a stored key is used first, with the provider's environment variable as the
        fallback when nothing is stored. <span className="font-semibold">Test saved</span> sends one
        real request with the saved values — the same ones the next agent turn will use;{" "}
        <span className="font-semibold">Test unsaved values</span> checks a draft before saving it.
      </p>
    </div>
  );
}
