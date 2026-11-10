import React, { useState, useEffect } from "react";
import { safeInvoke } from "../lib/runtime";
import { ENVIRONMENT } from "../lib/events";

type ServiceId = "github" | "gmail" | "slack" | "linear" | "gdrive" | "jira" | "notion" | "floot";

type ServiceStatus =
  | { connected: true; connectedAt: string | null }
  | { connected: false; reason: string };

type RawService = {
  service: ServiceId;
  connected: boolean;
  status: string;
  connected_at: string | null;
};

type IntegrationsData = {
  services: RawService[];
  composio_reachable: boolean;
  floot_registered: boolean;
};

type IntegrationsSnapshot =
  | { ok: true; data: IntegrationsData }
  | { ok: false; reason: string };

function isOk(snap: IntegrationsSnapshot): snap is { ok: true; data: IntegrationsData } {
  return snap.ok === true;
}

const SERVICE_CONFIG: Record<ServiceId, { label: string; icon: string }> = {
  github: { label: "GitHub", icon: "💻" },
  gmail: { label: "Gmail", icon: "📧" },
  slack: { label: "Slack", icon: "💬" },
  linear: { label: "Linear", icon: "📋" },
  gdrive: { label: "Google Drive", icon: "☁️" },
  jira: { label: "Jira", icon: "🗂️" },
  notion: { label: "Notion", icon: "📝" },
  floot: { label: "Floot MCP", icon: "🛰️" },
};

const STATUS_LABEL: Record<string, string> = {
  not_setup: "Not connected",
  connected: "Connected",
  token_expired: "Token expired — reauth required",
};

function statusFromSnapshot(svc: RawService, isFloot: boolean): ServiceStatus {
  if (isFloot) {
    if (svc.connected) return { connected: true, connectedAt: svc.connected_at };
    return { connected: false, reason: "Floot MCP not registered — add it from MCP Bridges" };
  }
  if (svc.connected) return { connected: true, connectedAt: svc.connected_at };
  return { connected: false, reason: STATUS_LABEL[svc.status] ?? svc.status };
}

function ServiceCard({ svc }: { svc: RawService }) {
  const cfg = SERVICE_CONFIG[svc.service] ?? { label: svc.service, icon: "🔌" };
  const status = statusFromSnapshot(svc, svc.service === "floot");

  return (
    <div
      className={`rounded-lg border ${status.connected
          ? "border-emerald-900/40 bg-emerald-950/20"
          : "border-border bg-background/40"
        } p-3 flex items-center justify-between gap-3`}
    >
      <div className="flex items-center gap-3 min-w-0">
        <span className="text-lg shrink-0">{cfg.icon}</span>
        <div className="min-w-0">
          <p className="text-sm font-medium text-text-primary truncate">{cfg.label}</p>
          <p className="text-xs text-text-muted truncate">
            {status.connected
              ? (status.connectedAt ?? "Connected")
              : status.reason}
          </p>
        </div>
      </div>
      <div className="shrink-0">
        {status.connected ? (
          <span className="inline-flex items-center gap-1 rounded-full bg-emerald-900/30 border border-emerald-900/40 px-2 py-0.5 text-[11px] text-emerald-300">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
            Connected
          </span>
        ) : (
          <button
            onClick={() => {
              alert(`Connect ${cfg.label}: not wired yet. Backend status: ${status.reason}`);
            }}
            className="rounded border border-border px-2.5 py-1 text-[11px] text-text-secondary hover:bg-surface hover:text-text-primary transition-colors"
          >
            Connect
          </button>
        )}
      </div>
    </div>
  );
}

export function ConnectCards() {
  const [snapshot, setSnapshot] = useState<IntegrationsSnapshot>({ ok: false, reason: "loading" });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      setError(null);
      try {
        const raw = await safeInvoke<IntegrationsData>(ENVIRONMENT, "get_integrations_status");
        if (cancelled) return;
        if (raw.ok) {
          setSnapshot({ ok: true, data: raw.data });
        } else {
          setSnapshot({ ok: false, reason: raw.reason });
        }
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  if (!isOk(snapshot)) {
    if (snapshot.reason === "loading") {
      return (
        <div className="rounded-lg border border-border bg-background/40 p-4">
          <p className="text-xs text-text-muted">Loading integrations…</p>
        </div>
      );
    }
    if (error) {
      return (
        <div className="rounded-lg border border-border bg-background/40 p-4">
          <p className="text-xs text-error">Error loading integrations: {error}</p>
        </div>
      );
    }
    return (
      <div className="rounded-lg border border-border bg-background/40 p-4">
        <p className="text-xs text-text-muted">Integrations backend unavailable: {snapshot.reason}</p>
      </div>
    );
  }

  const data = snapshot.data;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-text-primary">Connected Services</h3>
        {!data.composio_reachable && (
          <span className="text-[11px] text-amber-600 bg-amber-950/30 rounded px-2 py-0.5 border border-amber-900/40">
            Composio backend not reachable
          </span>
        )}
      </div>

      <div className="space-y-2">
        {data.services.map((svc) => (
          <ServiceCard key={svc.service} svc={svc} />
        ))}
      </div>

      <div className="rounded-lg border border-border bg-background/40 p-3">
        <h4 className="text-xs font-medium text-text-muted uppercase tracking-wider mb-2">Floot MCP</h4>
        <p className="text-xs text-text-muted">
          {data.floot_registered
            ? "Floot MCP bridge is registered. Tools discovered through Floot pass through the permission gate and evidence ledger."
            : "Floot MCP is not registered yet. Add it from the MCP Bridges panel (Extensions & MCP)."}
        </p>
      </div>
    </div>
  );
}
