import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { McpToolCall } from "../lib/events";
import { onMcpToolCall, drainBacklog } from "../lib/events";

type ToolDescriptor = {
  id: string;
  transport: string;
  command: string;
  env_keys: string[];
  description?: string | null;
};

function badgeForStatus(s: McpToolCall["status"]) {
  switch (s) {
    case "calling":
      return "bg-sky-950/60 border-sky-800 text-sky-300";
    case "retry":
      return "bg-amber-950/60 border-amber-800 text-amber-300";
    case "complete":
      return "bg-emerald-950/60 border-emerald-800 text-emerald-300";
    case "failed":
      return "bg-red-950/60 border-red-800 text-red-300";
    default:
      return "bg-zyl-bg border-zyl-border text-zyl-muted";
  }
}

function transportLabel(t: string) {
  const c =
    t === "stdio"
      ? "border-violet-800 text-violet-300"
      : t === "sse"
        ? "border-cyan-800 text-cyan-300"
        : "border-fuchsia-800 text-fuchsia-300";
  return `rounded px-1.5 py-0.5 text-[10px] border bg-zyl-bg ${c}`;
}

export default function McpInspector() {
  const [tools, setTools] = useState<ToolDescriptor[]>([]);
  const [calls, setCalls] = useState<McpToolCall[]>([]);
  const [err, setErr] = useState<string | null>(null);

  // initial list + backlog drain
  useEffect(() => {
    invoke<ToolDescriptor[]>("list_tools")
      .then(setTools)
      .catch((e) => setErr(String(e)));
    // also list MCP bridges as fallback
    if (tools.length === 0) {
      invoke<ToolDescriptor[]>("list_mcp_bridges")
        .then((bridges: unknown) => {
          // bridges have different shape; map to descriptor if list_tools empty
          if (Array.isArray(bridges) && (bridges as unknown[]).length > 0 && tools.length === 0) {
            // keep original tools; bridges shown via same feed
          }
        })
        .catch(() => {});
    }
    const pending = drainBacklog<McpToolCall>("zylcode://mcp-tool-call");
    if (pending.length) setCalls((c) => [...c, ...pending]);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onMcpToolCall((payload) => {
      setCalls((c) => [...c.slice(-99), payload]);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-semibold uppercase tracking-widest text-zyl-muted">MCP Activity</h3>
        <span className="text-[10px] font-mono text-zyl-muted">{tools.length} tools</span>
      </div>

      {err && <div className="rounded border border-red-900 bg-red-950/30 p-2 text-xs text-red-300">{err}</div>}

      {/* Tool registry */}
      <div className="space-y-1">
        {tools.length === 0 ? (
          <p className="text-xs text-zyl-muted">No tools registered. Add `mcp.tools.yaml` → hot-reload.</p>
        ) : (
          tools.map((t) => (
            <div key={t.id} className="flex items-center justify-between rounded border border-zyl-border bg-zyl-bg px-2 py-1.5">
              <span className="font-mono text-xs text-white">{t.id}</span>
              <span className={transportLabel(t.transport)}>{t.transport}</span>
            </div>
          ))
        )}
      </div>

      {/* Live feed */}
      <div className="rounded border border-zyl-border bg-zyl-bg p-2">
        <div className="mb-2 text-[10px] font-semibold uppercase tracking-widest text-zyl-muted">Live feed</div>
        {calls.length === 0 ? (
          <p className="text-xs text-zyl-muted">Awaiting tool calls…</p>
        ) : (
          <ul className="space-y-1.5 max-h-48 overflow-auto pr-1">
            {calls.map((c, i) => (
              <li key={i} className="flex items-center justify-between gap-2 rounded bg-zyl-surface px-2 py-1 border border-zyl-border/50">
                <span className="font-mono text-xs text-white truncate">{c.tool}</span>
                <span className={`rounded border px-2 py-0.5 text-[10px] font-semibold uppercase ${badgeForStatus(c.status)}`}>
                  {c.status}
                  {c.attempt ? ` #${c.attempt}` : ""}
                </span>
                <span className="text-[10px] font-mono text-zyl-muted">
                  {c.duration_ms != null ? `${c.duration_ms}ms` : ""}
                  {c.error ? ` · ${c.error.slice(0, 60)}` : ""}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
