import React, { useCallback, useEffect, useRef, useState } from "react";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import { ChangesPanel } from "./ChangesPanel";
import {
  enqueueMission,
  listMissions,
  runNextMission,
  type Mission,
  type MissionMode,
} from "../lib/missions";
import { fetchEvidence, type EvidenceState } from "../lib/surfaces";
import { useSettings } from "../lib/settings";

type AgentTab = "agent" | "tasks" | "changes" | "evidence";

const TABS: { id: AgentTab; label: string }[] = [
  { id: "agent", label: "AI Agent" },
  { id: "tasks", label: "Tasks" },
  { id: "changes", label: "Changes" },
  { id: "evidence", label: "Evidence" },
];

const STATE_COLOR: Record<Mission["state"], string> = {
  queued: "text-text-muted",
  running: "text-primary animate-pulse",
  done: "text-emerald-400",
  failed: "text-red-400",
};

function ago(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "";
  const s = Math.max(0, Math.round((Date.now() - t) / 1000));
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  return `${Math.floor(s / 3600)}h`;
}

interface AgentWorkspaceProps {
  activeTab: AgentTab;
  onTabChange: (tab: AgentTab) => void;
  /** Dock width; the workspace renders a stacked variant when narrow. */
  narrow?: boolean;
}

/**
 * Lower-right agent workspace: AI Agent (compose + run), Tasks (the real
 * mission queue with review/approval), Changes (working tree), Evidence
 * (hash-chained ledger). Every action maps to a real backend call; nothing
 * is simulated and nothing auto-approves — queued work runs only when a
 * human clicks Run.
 */
export const AgentWorkspace: React.FC<AgentWorkspaceProps> = ({
  activeTab,
  onTabChange,
  narrow,
}) => {
  const [task, setTask] = useState("");
  const { settings } = useSettings();
  const [mode, setMode] = useState<MissionMode>(settings.defaultMissionMode);
  // Follow a changed default when the composer is idle and untouched.
  useEffect(() => {
    setMode(settings.defaultMissionMode);
  }, [settings.defaultMissionMode]);
  const [missions, setMissions] = useState<Mission[]>([]);
  const [evidence, setEvidence] = useState<EvidenceState | null>(null);
  const [connected, setConnected] = useState<CapabilityStatus>("BLOCKED");
  const [note, setNote] = useState("connecting…");
  const [busy, setBusy] = useState(false);
  const mounted = useRef(true);

  const refresh = useCallback(async () => {
    const [list, ev] = await Promise.all([listMissions(), fetchEvidence()]);
    if (!mounted.current) return;
    setMissions(list);
    setEvidence(ev);
    if (ev.kind === "ready") {
      setConnected("AVAILABLE");
      setNote(`${ev.entry_count} entries · chain ${ev.chain_intact ? "intact" : "BROKEN"}`);
    } else if (ev.kind === "empty") {
      setConnected("LIMITED");
      setNote("ledger empty — run a build mission");
    } else {
      setConnected("BLOCKED");
      setNote(ev.kind === "unavailable" ? ev.reason : "loading…");
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    const t = setInterval(() => void refresh(), 5000);
    return () => {
      mounted.current = false;
      clearInterval(t);
    };
  }, [refresh]);

  const submit = async () => {
    const trimmed = task.trim();
    if (!trimmed || busy) return;
    setBusy(true);
    try {
      await enqueueMission(trimmed, mode);
      setTask("");
      await refresh();
    } finally {
      setBusy(false);
    }
  };

  const runNext = async () => {
    if (busy) return;
    const queued = missions.filter((m) => m.state === "queued");
    if (queued.length === 0) return;
    const label = queued[0].task;
    if (
      !window.confirm(
        `Run queued mission now?\n\n“${label}”\n\nBuild missions execute the real verification pipeline (the test suite runs in an isolated worktree; this takes several minutes).`,
      )
    )
      return;
    setBusy(true);
    try {
      await runNextMission();
      await refresh();
    } finally {
      setBusy(false);
    }
  };

  const queuedCount = missions.filter((m) => m.state === "queued").length;
  const running = missions.find((m) => m.state === "running") ?? null;
  const missionTimestamp = (m: Mission) => m.updatedAt;

  return (
    <div className="flex-1 min-h-0 flex flex-col" data-testid="agent-workspace">
      {/* Tabs */}
      <div className="flex items-center border-b border-border shrink-0 overflow-x-auto">
        {TABS.map((t) => (
          <button
            key={t.id}
            onClick={() => onTabChange(t.id)}
            className={`px-3 py-1.5 text-xs whitespace-nowrap border-b-2 -mb-px transition-colors ${
              activeTab === t.id
                ? "border-primary text-text-primary"
                : "border-transparent text-text-muted hover:text-text-primary"
            }`}
          >
            {t.label}
          </button>
        ))}
        <div className="ml-auto pr-2 flex items-center gap-2">
          <StatusBadge status={connected} size="xs" />
        </div>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto p-2.5 space-y-2.5">
        {activeTab === "agent" && (
          <>
            <textarea
              value={task}
              onChange={(e) => setTask(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) void submit();
              }}
              placeholder={
                narrow
                  ? "Describe a task… (Ctrl+Enter to queue)"
                  : "Describe a task for the agent… (Ctrl+Enter to queue)"
              }
              rows={2}
              className="w-full bg-background border border-border rounded px-2 py-1.5 text-xs text-text-primary placeholder:text-text-muted focus:outline-none focus:border-primary/50 resize-y"
            />
            <div className="flex items-center gap-1.5">
              {(["build", "plan"] as MissionMode[]).map((m) => (
                <button
                  key={m}
                  onClick={() => setMode(m)}
                  className={`px-2.5 py-1 rounded text-[11px] border transition-colors ${
                    mode === m
                      ? "border-primary/60 bg-primary/15 text-primary font-medium"
                      : "border-border text-text-muted hover:text-text-primary"
                  }`}
                  title={
                    m === "build"
                      ? "Runs the real Best-of-N verification pipeline (test suite in an isolated worktree)"
                      : "Read-only: produces a real plan from the intelligence pipeline; no code executes"
                  }
                >
                  {m === "build" ? "⚡ Build" : "🗺 Plan"}
                </button>
              ))}
              <button
                onClick={() => void submit()}
                disabled={!task.trim() || busy}
                className="ml-auto px-3 py-1 rounded text-[11px] bg-primary/15 text-primary border border-primary/40 hover:bg-primary/25 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                {busy ? "…" : "Queue"}
              </button>
            </div>
            {running && (
              <div className="border border-primary/40 bg-primary/5 rounded px-2.5 py-1.5 text-xs">
                <span className="text-primary animate-pulse font-medium">running:</span>{" "}
                <span className="text-text-secondary">{running.task}</span>
              </div>
            )}
            <p className="text-[10px] text-text-muted">
              Queued work never runs by itself — execution happens only when you press
              “Run queued”. Build missions verify candidates by running the real test suite.
            </p>
          </>
        )}

        {activeTab === "tasks" && (
          <>
            <div className="flex items-center justify-between">
              <span className="text-[10px] uppercase tracking-wider text-text-muted">
                Mission queue · {missions.length}
              </span>
              {queuedCount > 0 && (
                <button
                  onClick={() => void runNext()}
                  disabled={busy || Boolean(running)}
                  className="px-2.5 py-1 rounded text-[11px] bg-primary/15 text-primary border border-primary/40 hover:bg-primary/25 disabled:opacity-40 disabled:cursor-not-allowed"
                  title="Drain the next queued mission through the real pipeline"
                >
                  {running ? "Run in progress…" : `Run ${queuedCount} queued`}
                </button>
              )}
            </div>
            {missions.length === 0 && (
              <p className="text-xs text-text-muted py-3 text-center">
                No missions yet — compose one in the AI Agent tab.
              </p>
            )}
            {missions.map((m) => (
              <div key={m.id} className="border border-border rounded px-2.5 py-1.5 space-y-1">
                <div className="flex items-center gap-2">
                  <span className={`text-[11px] font-medium ${STATE_COLOR[m.state]}`}>
                    {m.state === "running" ? "◍" : m.state === "done" ? "●" : m.state === "failed" ? "✕" : "○"}{" "}
                    {m.state}
                  </span>
                  <span className="text-[10px] text-text-muted font-mono uppercase">{m.mode}</span>
                  <span className="ml-auto text-[10px] text-text-muted">{ago(missionTimestamp(m))}</span>
                </div>
                <p className="text-xs text-text-primary">{m.task}</p>
                {m.summary && <p className="text-[11px] text-text-muted">{m.summary}</p>}
                {m.ledgerSession && (
                  <p className="text-[10px] text-text-muted font-mono">
                    ledger: {m.ledgerSession.slice(0, 8)}
                  </p>
                )}
              </div>
            ))}
          </>
        )}

        {activeTab === "changes" && (
          <div className="flex-1 min-h-0 flex flex-col -m-2.5">
            <ChangesPanel />
          </div>
        )}

        {activeTab === "evidence" && (
          <div className="space-y-2">
            {evidence?.kind === "ready" && (
              <>
                <div className="flex items-center gap-2 text-[11px]">
                  <span className={evidence.chain_intact ? "text-emerald-400" : "text-red-400"}>
                    {evidence.chain_intact ? "✓ hash chain intact" : "✕ CHAIN BROKEN"}
                  </span>
                  <span className="text-text-muted">
                    {evidence.entry_count} entries · {evidence.session_count} session
                    {evidence.session_count === 1 ? "" : "s"}
                  </span>
                </div>
                {evidence.entries.map((e) => (
                  <div key={e.id} className="border border-border rounded px-2.5 py-1.5">
                    <div className="flex items-center gap-2">
                      <span className="text-[11px] font-mono text-text-secondary">{e.action_id}</span>
                      <span className="ml-auto text-[10px] text-text-muted">{e.state}</span>
                    </div>
                    <p className="text-[10px] text-text-muted font-mono">
                      {e.session_id.slice(0, 8)} · {new Date(e.timestamp).toLocaleTimeString()}
                    </p>
                  </div>
                ))}
              </>
            )}
            {evidence?.kind === "empty" && (
              <p className="text-xs text-text-muted py-3 text-center">{evidence.reason}</p>
            )}
            {evidence?.kind === "unavailable" && (
              <p className="text-xs text-text-muted py-3 text-center">{evidence.reason}</p>
            )}
            {evidence?.kind === "loading" && (
              <p className="text-xs text-text-muted py-3 text-center">loading…</p>
            )}
          </div>
        )}
      </div>

      <div className="shrink-0 border-t border-border px-2.5 py-1">
        <span className="text-[10px] text-text-muted truncate block" title={note}>
          {note}
        </span>
      </div>
    </div>
  );
};

export default AgentWorkspace;
