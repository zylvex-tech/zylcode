import React, { useEffect, useState } from "react";
import { McpInspector, ProviderSettings, DiagnosticsPanel } from "../../components";
import { IS_DESKTOP, DESKTOP_REQUIRED_MESSAGE, getDiagnostics, type DiagnosticEntry } from "../../lib/runtime";
import { fetchEvidence, type EvidenceState } from "../../lib/surfaces";
import { listMissions } from "../../lib/missions";
import TerminalPanel from "../TerminalPanel";
import ToolsCataloguePanel from "../ToolsCataloguePanel";
import { fetchProviders, type ProvidersState } from "../../lib/service";

export type BottomTab =
  | "terminal"
  | "output"
  | "problems"
  | "debug-console"
  | "ports"
  | "tests"
  | "evidence"
  | "dev-tools";

interface BottomPanelProps {
  isOpen: boolean;
  onToggle: () => void;
  activeTab: BottomTab;
  onTabChange: (tab: BottomTab) => void;
}

const BOTTOM_TABS: { id: BottomTab; label: string; icon: React.ReactNode }[] = [
  {
    id: "terminal",
    label: "Terminal",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <polyline points="4 17 10 11 4 5"></polyline>
        <line x1="12" y1="19" x2="20" y2="11"></line>
      </svg>
    ),
  },
  {
    id: "output",
    label: "Output",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <polyline points="22 18 12 12 2 18"></polyline>
        <polyline points="22 6 12 12 2 6"></polyline>
      </svg>
    ),
  },
  {
    id: "problems",
    label: "Problems",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <circle cx="12" cy="12" r="10"></circle>
        <line x1="12" y1="8" x2="12" y2="12"></line>
        <line x1="12" y1="16" x2="12.01" y2="16"></line>
      </svg>
    ),
  },
  {
    id: "debug-console",
    label: "Debug Console",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
      </svg>
    ),
  },
  {
    id: "ports",
    label: "Ports",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
        <polyline points="15 3 21 3 21 9"></polyline>
        <line x1="10" y1="14" x2="21" y2="3"></line>
      </svg>
    ),
  },
  {
    id: "tests",
    label: "Tests",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <path d="M9 18V5l12-2v13"></path>
        <circle cx="6" cy="18" r="6"></circle>
      </svg>
    ),
  },
  {
    id: "evidence",
    label: "Evidence",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <path d="M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z"></path>
        <path d="M12 6v6l4 2"></path>
      </svg>
    ),
  },
  {
    id: "dev-tools",
    label: "Dev Tools",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
        <path d="M8 21h8"></path>
        <path d="M12 17v4"></path>
      </svg>
    ),
  },
];

// ---------------------------------------------------------------------------
// Output: the app's own diagnostics channel (real entries recorded at runtime)
// ---------------------------------------------------------------------------

function OutputView() {
  const [entries, setEntries] = useState<DiagnosticEntry[]>([]);
  useEffect(() => {
    const pull = () => setEntries([...getDiagnostics()].reverse());
    pull();
    const t = setInterval(pull, 2000);
    return () => clearInterval(t);
  }, []);
  if (entries.length === 0) {
    return (
      <div className="h-full p-3 overflow-y-auto font-mono text-xs text-text-muted">
        <p>
          Channel: <span className="text-primary">ZylCode</span> — diagnostics recorded by the
          frontend at runtime (backend failures, IPC issues). No output yet.
        </p>
      </div>
    );
  }
  return (
    <div className="h-full p-3 overflow-y-auto font-mono text-xs space-y-1">
      {entries.map((e, i) => (
        <p key={i} className="break-words">
          <span className="text-text-muted/60">{new Date(e.timestamp).toLocaleTimeString()} </span>
          <span className="text-amber-400">[{e.source}]</span> <span className="text-code-text">{e.message}</span>
        </p>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Problems: real workspace lint signals (untracked/modified work + honest note)
// ---------------------------------------------------------------------------

interface Problem {
  severity: "error" | "warning" | "info";
  message: string;
  source: string;
}

function useProblems(): { problems: Problem[]; unavailable: string | null } {
  const [problems, setProblems] = useState<Problem[]>([]);
  const [unavailable, setUnavailable] = useState<string | null>(null);
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [git, missions] = await Promise.all([
          import("../../lib/gitStatus").then((m) => m.fetchGitStatus()),
          listMissions(),
        ]);
        if (cancelled) return;
        const found: Problem[] = [];
        if (git.kind === "ready") {
          for (const e of git.data.entries) {
            if (e.status === "??") {
              found.push({
                severity: "info",
                message: `Untracked file — not part of any commit or verification run`,
                source: e.path,
              });
            }
          }
        } else if (git.kind === "unavailable") {
          setUnavailable(git.reason);
          return;
        }
        for (const m of missions) {
          if (m.state === "failed") {
            found.push({
              severity: "warning",
              message: `Mission failed: ${m.summary ?? m.task}`,
              source: `mission ${m.id.slice(0, 8)}`,
            });
          }
        }
        setProblems(found);
      } catch (e) {
        if (!cancelled) setUnavailable(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);
  return { problems, unavailable };
}

function ProblemsView() {
  const { problems, unavailable } = useProblems();
  if (unavailable) {
    return <div className="h-full p-3 text-xs text-text-muted">{unavailable}</div>;
  }
  if (problems.length === 0) {
    return (
      <div className="h-full p-3 text-xs text-text-muted">
        No problems detected. (This panel reports real workspace signals — untracked files and
        failed missions. Compiler diagnostics appear as the toolchain surfaces them.)
      </div>
    );
  }
  return (
    <div className="h-full p-3 overflow-y-auto space-y-1.5 text-xs">
      {problems.map((p, i) => (
        <div key={i} className="flex items-start gap-2">
          <span
            className={
              p.severity === "warning"
                ? "text-amber-400"
                : p.severity === "error"
                  ? "text-red-400"
                  : "text-sky-400"
            }
          >
            {p.severity === "warning" ? "▲" : p.severity === "error" ? "✕" : "ⓘ"}
          </span>
          <span className="text-text-secondary">{p.message}</span>
          <span className="ml-auto text-text-muted font-mono text-[10px] shrink-0">{p.source}</span>
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Debug Console: honest disabled state (no debugger attached)
// ---------------------------------------------------------------------------

function DebugConsoleView() {
  return (
    <div className="h-full p-3 text-xs text-text-muted space-y-1.5">
      <p className="text-text-secondary">Debug Console — not available.</p>
      <p>
        No debugger integration is commissioned in this build. The Terminal tab runs real shell
        commands; missions and their verification runs are visible in the Evidence ledger.
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Ports: the ports this app genuinely serves
// ---------------------------------------------------------------------------

interface PortRow {
  port: number;
  label: string;
  url: string;
}

function PortsView() {
  const [rows] = useState<PortRow[]>(() => {
    const intel = Number(new URLSearchParams(window.location.search).get("intelPort") ?? 17630);
    return [
      { port: intel, label: "zylcode serve-intel (repo intelligence API)", url: `http://localhost:${intel}` },
      { port: Number(window.location.port) || 1420, label: "This app (Vite dev server)", url: window.location.origin },
    ];
  });
  return (
    <div className="h-full p-3 space-y-1.5 text-xs">
      <p className="text-text-muted text-[11px]">
        Ports actually served by this workspace. Nothing here is invented.
      </p>
      {rows.map((r) => (
        <div key={r.port} className="flex items-center gap-2">
          <span className="w-14 font-mono text-primary">{r.port}</span>
          <span className="text-text-secondary">{r.label}</span>
          <a href={r.url} target="_blank" rel="noreferrer" className="ml-auto text-[10px] text-text-muted hover:text-primary font-mono">
            {r.url}
          </a>
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Tests: real mission verification runs from the ledger, honestly empty
// ---------------------------------------------------------------------------

interface TestRunRow {
  session: string;
  when: string;
  passed: boolean;
  checks: string;
  detail: string;
}

function useTestRuns(): { rows: TestRunRow[]; state: "loading" | "empty" | "ready" | "unavailable"; reason?: string } {
  const [evidence, setEvidence] = useState<EvidenceState | null>(null);
  useEffect(() => {
    let cancelled = false;
    const pull = () => {
      fetchEvidence().then((next) => {
        if (!cancelled) setEvidence(next);
      });
    };
    pull();
    const t = setInterval(pull, 6000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, []);
  if (!evidence || evidence.kind === "loading") return { rows: [], state: "loading" };
  if (evidence.kind === "unavailable") return { rows: [], state: "unavailable", reason: evidence.reason };
  if (evidence.kind === "empty") return { rows: [], state: "empty" };
  const rows: TestRunRow[] = [];
  const bySession = new Map<string, typeof evidence.entries>();
  for (const e of evidence.entries) {
    const list = bySession.get(e.session_id) ?? [];
    list.push(e);
    bySession.set(e.session_id, list);
  }
  for (const [session, list] of bySession) {
    const started = list.find((e) => e.state === "Started");
    const selection = list.find((e) => e.action_id.includes("selection"));
    const candidates = list.filter((e) => e.action_id.includes("candidate"));
    if (!started && !selection) continue;
    const payload = selection?.payload as { outcomes?: { candidate_index: number; passed: boolean; passing_checks: number }[]; reason?: string } | undefined;
    const outcomes = payload?.outcomes ?? [];
    const anyPassed = outcomes.some((o) => o.passed);
    const totalChecks = outcomes.length > 0 ? Math.max(...outcomes.map((o) => o.passing_checks)) : null;
    rows.push({
      session: session.slice(0, 8),
      when: started?.timestamp ?? selection?.timestamp ?? list[0].timestamp,
      passed: Boolean(anyPassed),
      checks: totalChecks !== null ? `${totalChecks} checks` : "outcome recorded",
      detail: payload?.reason ?? `${candidates.length} candidate run${candidates.length === 1 ? "" : "s"}`,
    });
  }
  return { rows, state: "ready" };
}

function TestsView() {
  const { rows, state, reason } = useTestRuns();
  if (state === "loading") return <div className="h-full p-3 text-xs text-text-muted">Reading the evidence ledger…</div>;
  if (state === "unavailable") return <div className="h-full p-3 text-xs text-text-muted">{reason}</div>;
  if (state === "empty" || rows.length === 0) {
    return (
      <div className="h-full p-3 text-xs text-text-muted">
        No verification runs recorded yet. Run a Build mission — its real test-suite execution
        lands here with the recorded outcome.
      </div>
    );
  }
  return (
    <div className="h-full p-3 overflow-y-auto space-y-1.5 text-xs">
      {rows.map((r) => (
        <div key={r.session} className="flex items-center gap-2">
          <span className={r.passed ? "text-emerald-400" : "text-red-400"}>{r.passed ? "✓" : "✕"}</span>
          <span className="font-mono text-text-secondary">{r.session}</span>
          <span className="text-text-muted">{r.checks}</span>
          <span className="text-text-muted/70 truncate">{r.detail}</span>
          <span className="ml-auto text-[10px] text-text-muted shrink-0">{new Date(r.when).toLocaleString()}</span>
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Evidence (compact ledger view) + Dev Tools
// ---------------------------------------------------------------------------

function EvidenceView() {
  const [evidence, setEvidence] = useState<EvidenceState | null>(null);
  useEffect(() => {
    let cancelled = false;
    fetchEvidence().then((next) => {
      if (!cancelled) setEvidence(next);
    });
    return () => {
      cancelled = true;
    };
  }, []);
  if (!evidence || evidence.kind === "loading")
    return <div className="h-full p-3 text-xs text-text-muted">loading…</div>;
  if (evidence.kind === "empty")
    return <div className="h-full p-3 text-xs text-text-muted">{evidence.reason}</div>;
  if (evidence.kind === "unavailable")
    return <div className="h-full p-3 text-xs text-text-muted">{evidence.reason}</div>;
  return (
    <div className="h-full p-3 overflow-y-auto space-y-1 text-xs">
      <p className="text-[11px] mb-2">
        <span className={evidence.chain_intact ? "text-emerald-400" : "text-red-400"}>
          {evidence.chain_intact ? "✓ chain intact" : "✕ CHAIN BROKEN"}
        </span>{" "}
        <span className="text-text-muted">
          — {evidence.entry_count} entries across {evidence.session_count} sessions
        </span>
      </p>
      {evidence.entries.map((e) => (
        <p key={e.id} className="font-mono break-words">
          <span className="text-text-muted/60">{new Date(e.timestamp).toLocaleTimeString()} </span>
          <span className="text-primary">{e.action_id}</span>{" "}
          <span className="text-text-muted">{e.state}</span>
        </p>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Providers (browser-capable): real fallback chain from the service; the
// desktop additionally gets the editable settings.
// ---------------------------------------------------------------------------

function ProvidersServiceView() {
  const [state, setState] = useState<ProvidersState>({ kind: "loading" });
  useEffect(() => {
    let cancelled = false;
    const load = () => {
      fetchProviders().then((next) => {
        if (!cancelled) setState(next);
      });
    };
    load();
    const t = setInterval(load, 15000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, []);
  if (state.kind === "loading") return <p className="p-3 text-xs text-text-muted">Reading the provider chain…</p>;
  if (state.kind === "unavailable") return <p className="p-3 text-xs text-text-muted">{state.reason}</p>;
  return (
    <div className="p-3 space-y-1.5 text-xs">
      <p className="text-[11px] text-text-muted mb-2">
        Live fallback chain (primary <span className="font-mono text-text-secondary">{state.primary_model}</span>,
        fallback <span className="font-mono text-text-secondary">{state.fallback_model}</span>). Secrets are never
        exposed; editing happens in Settings → Model Providers on the desktop.
      </p>
      {state.chain.map((p) => (
        <div key={`${p.kind}-${p.fallback_order}`} className="flex items-center gap-2">
          <span className="font-mono text-text-secondary w-24 shrink-0">{p.kind}</span>
          <span className="truncate text-text-muted">{p.model}</span>
          {p.requires_api_key && (
            <span className="text-[10px] text-amber-400" title="This provider requires an API key (configured on the desktop)">
              key required
            </span>
          )}
          <span className={`ml-auto text-[10px] ${p.enabled ? "text-emerald-400" : "text-text-muted"}`}>
            {p.enabled ? "enabled" : "disabled"}
          </span>
        </div>
      ))}
    </div>
  );
}

function DevToolsView() {
  return (
    <div className="h-full p-3 overflow-y-auto space-y-3">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
        <div className="border border-border rounded-lg p-3">
          <h4 className="text-sm font-medium mb-2">MCP Tool Catalogue</h4>
          <ToolsCataloguePanel />
          {IS_DESKTOP && (
            <div className="mt-3 border-t border-border pt-2">
              <h4 className="text-sm font-medium mb-2">MCP Bridge Activity</h4>
              <McpInspector />
            </div>
          )}
        </div>
        <div className="border border-border rounded-lg p-3">
          <h4 className="text-sm font-medium mb-2">Providers</h4>
          {IS_DESKTOP ? <ProviderSettings /> : <ProvidersServiceView />}
        </div>
        <div className="border border-border rounded-lg p-3 lg:col-span-2">
          <h4 className="text-sm font-medium mb-2">Diagnostics</h4>
          <DiagnosticsPanel />
        </div>
      </div>
    </div>
  );
}

function DesktopRequired() {
  return (
    <div className="flex flex-col items-center justify-center gap-2 py-4 text-center text-sm">
      <p className="text-text-muted max-w-xs">{DESKTOP_REQUIRED_MESSAGE}</p>
      <p className="text-[11px] text-text-muted/70">Technical details are available in Diagnostics.</p>
    </div>
  );
}

// ---------------------------------------------------------------------------

export function BottomPanel({ isOpen, onToggle, activeTab, onTabChange }: BottomPanelProps) {
  const [height, setHeight] = useState(240);
  const resizeRef = React.useRef<{ startY: number; startH: number } | null>(null);

  const onPointerDown = (e: React.PointerEvent) => {
    resizeRef.current = { startY: e.clientY, startH: height };
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e: React.PointerEvent) => {
    if (!resizeRef.current) return;
    const delta = resizeRef.current.startY - e.clientY;
    setHeight(Math.min(Math.max(120, resizeRef.current.startH + delta), Math.floor(window.innerHeight * 0.7)));
  };
  const onPointerUp = (e: React.PointerEvent) => {
    resizeRef.current = null;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
  };

  const renderTabContent = () => {
    switch (activeTab) {
      case "terminal":
        return <TerminalPanel />;
      case "output":
        return <OutputView />;
      case "problems":
        return <ProblemsView />;
      case "debug-console":
        return <DebugConsoleView />;
      case "ports":
        return <PortsView />;
      case "tests":
        return <TestsView />;
      case "evidence":
        return <EvidenceView />;
      case "dev-tools":
        return <DevToolsView />;
      default:
        return <TerminalPanel />;
    }
  };

  return (
    <React.Fragment>
      {isOpen && (
        <div
          className="h-1 shrink-0 bg-transparent hover:bg-primary/30 transition-colors"
          style={{ cursor: "row-resize" }}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          role="separator"
          aria-orientation="horizontal"
          aria-label="Resize bottom panel"
        />
      )}
      {isOpen && (
        <div className="h-[var(--panel-h)] min-h-0 shrink-0 border-t border-border bg-surface flex flex-col" style={{ height, "--panel-h": `${height}px` } as React.CSSProperties}>
          <div className="flex items-center justify-between px-2 py-1 border-b border-border shrink-0">
            <div className="flex items-center gap-0.5 overflow-x-auto">
              {BOTTOM_TABS.map((tab) => (
                <button
                  key={tab.id}
                  onClick={() => onTabChange(tab.id)}
                  className={`px-2.5 py-1 rounded text-xs whitespace-nowrap transition-colors flex items-center gap-1.5 ${
                    activeTab === tab.id
                      ? "bg-primary/10 text-primary font-medium"
                      : "text-text-muted hover:text-text-primary hover:bg-surface"
                  }`}
                >
                  <span>{tab.icon}</span>
                  <span>{tab.label}</span>
                </button>
              ))}
            </div>
            <button
              onClick={onToggle}
              className="p-1 text-text-muted hover:text-text-primary transition-colors"
              aria-label="Close panel"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </button>
          </div>
          <div className="flex-1 min-h-0 overflow-hidden">
            {renderTabContent()}
          </div>
        </div>
      )}
    </React.Fragment>
  );
}
