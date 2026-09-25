import { useCallback, useEffect, useRef, useState } from "react";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import {
  enqueueMission,
  listMissions,
  runNextMission,
  clearMissions,
  type Mission,
  type MissionMode,
} from "../lib/missions";
import { fetchEvidence, type EvidenceState } from "../lib/surfaces";
const STATE_COLOR: Record<Mission["state"], string> = {
  queued: "text-text-muted",
  running: "text-primary animate-pulse",
  done: "text-emerald-400",
  failed: "text-red-400",
};

const MODE_STYLE: Record<MissionMode, string> = {
  build: "bg-primary/15 text-primary border-primary/40",
  plan: "bg-amber-500/15 text-amber-400 border-amber-500/40",
};

function ago(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "";
  const s = Math.max(0, Math.round((Date.now() - t) / 1000));
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  return `${Math.floor(s / 3600)}h`;
}

/**
 * Mission Console — the live heart of the app: compose a task in Build or
 * Plan mode, watch the queue drain by actually running the pipeline, and
 * read the evidence ledger as the activity feed. Everything on screen is
 * backed by the real backend; nothing is simulated.
 */
export const PulsePanel: React.FC = () => {
  const [task, setTask] = useState("");
  const [mode, setMode] = useState<MissionMode>("build");
  const [missions, setMissions] = useState<Mission[]>([]);
  const [evidence, setEvidence] = useState<EvidenceState | null>(null);
  const [draining, setDraining] = useState(false);
  const [connected, setConnected] = useState<CapabilityStatus>("BLOCKED");
  const [note, setNote] = useState("connecting…");
  const mounted = useRef(true);

  const refresh = useCallback(async () => {
    const [list, ev] = await Promise.all([listMissions(), fetchEvidence()]);
    if (!mounted.current) return;
    setMissions(list);
    setEvidence(ev);
    if (ev.kind === "ready") {
      setConnected("AVAILABLE");
      setNote(
        `${ev.entry_count} ledger entries · chain ${ev.chain_intact ? "intact" : "BROKEN"}`,
      );
    } else if (ev.kind === "empty") {
      setConnected("LIMITED");
      setNote("ledger empty — run a build mission to record evidence");
    } else if (ev.kind === "unavailable") {
      setConnected("BLOCKED");
      setNote(ev.reason);
    } else {
      setConnected("LIMITED");
      setNote("loading ledger…");
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    const t = setInterval(() => void refresh(), 4000);
    return () => {
      mounted.current = false;
      clearInterval(t);
    };
  }, [refresh]);

  const submit = useCallback(async () => {
    const t = task.trim();
    if (!t || draining) return;
    const m = await enqueueMission(t, mode);
    if (m) {
      setTask("");
      void refresh();
    }
  }, [task, mode, draining, refresh]);

  const drain = useCallback(async () => {
    if (draining) return;
    setDraining(true);
    try {
      // Drain the whole queue, mission by mission. Each run-next call
      // blocks server-side until that mission actually finishes.
      for (;;) {
        const list = await listMissions();
        if (!list.some((m) => m.state === "queued")) break;
        await runNextMission();
        void refresh();
      }
    } finally {
      setDraining(false);
      void refresh();
    }
  }, [draining, refresh]);

  const queued = missions.filter((m) => m.state === "queued").length;
  const active = missions.find((m) => m.state === "running");

  return (
    <div className="flex flex-col gap-3 h-full min-h-0">
      {/* Composer */}
      <div className="rounded-lg border border-border bg-surface/60 p-3">
        <div className="flex items-center gap-2 mb-2">
          <button
            onClick={() => setMode("build")}
            className={`px-3 py-1 rounded text-xs font-medium border transition-colors ${
              mode === "build" ? MODE_STYLE.build : "border-border text-text-muted hover:text-text-primary"
            }`}
          >
            ⚡ Build
          </button>
          <button
            onClick={() => setMode("plan")}
            className={`px-3 py-1 rounded text-xs font-medium border transition-colors ${
              mode === "plan" ? MODE_STYLE.plan : "border-border text-text-muted hover:text-text-primary"
            }`}
          >
            🗺 Plan
          </button>
          <span className="text-[10px] text-text-muted font-mono ml-auto">
            {mode === "build"
              ? "runs the real pipeline · verified by the test suite"
              : "design only · nothing executes"}
          </span>
        </div>
        <div className="flex gap-2">
          <input
            value={task}
            onChange={(e) => setTask(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && void submit()}
            placeholder={mode === "build" ? "Describe the change to build…" : "Describe what to plan…"}
            className="flex-1 bg-background border border-border rounded px-3 py-2 text-sm outline-none focus:border-primary/60 placeholder:text-text-muted"
          />
          <button
            onClick={() => void submit()}
            disabled={!task.trim() || draining}
            className="px-4 rounded text-sm font-medium bg-primary text-background disabled:opacity-40 hover:opacity-90"
          >
            Queue
          </button>
        </div>
        <div className="flex items-center gap-2 mt-3">
          <StatusBadge status={connected} />
          <span className="text-xs text-text-muted font-mono truncate">{note}</span>
          <div className="ml-auto flex gap-2">
            {queued > 0 && (
              <button
                onClick={() => void drain()}
                disabled={draining}
                className="text-xs px-2 py-1 rounded border border-primary/50 text-primary hover:bg-primary/10 disabled:opacity-40"
              >
                {draining ? "draining…" : `Run ${queued} queued`}
              </button>
            )}
            <button
              onClick={() => void clearMissions().then(refresh)}
              className="text-xs px-2 py-1 rounded border border-border text-text-muted hover:text-text-primary"
            >
              Clear
            </button>
          </div>
        </div>
      </div>

      {/* Running banner */}
      {active && (
        <div className="rounded-lg border border-primary/40 bg-primary/5 px-3 py-2 flex items-center gap-2">
          <span className={`h-2 w-2 rounded-full ${draining ? "bg-primary animate-ping" : "bg-primary/50"}`} />
          <span className="text-xs text-primary font-mono">running:</span>
          <span className="text-xs text-text-primary truncate">{active.task}</span>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-3 flex-1 min-h-0 overflow-hidden">
        {/* Mission queue */}
        <div className="rounded-lg border border-border bg-surface/40 overflow-y-auto">
          <div className="px-3 py-2 border-b border-border text-[10px] font-mono tracking-widest text-text-muted">
            MISSION QUEUE {missions.length > 0 && `· ${missions.length}`}
          </div>
          {missions.length === 0 ? (
            <p className="p-3 text-xs text-text-muted">
              Empty. Queue a task above — Build missions execute for real and land their evidence in the ledger.
            </p>
          ) : (
            <ul className="divide-y divide-border/60">
              {missions.map((m) => (
                <li key={m.id} className="px-3 py-2.5">
                  <div className="flex items-center gap-2">
                    <span className={`text-[10px] font-mono uppercase ${STATE_COLOR[m.state]}`}>
                      {m.state === "running" && "◍ "}
                      {m.state}
                    </span>
                    <span className={`text-[9px] px-1.5 py-0.5 rounded border font-mono uppercase ${MODE_STYLE[m.mode]}`}>
                      {m.mode}
                    </span>
                    <span className="ml-auto text-[10px] text-text-muted font-mono">{ago(m.updatedAt)}</span>
                  </div>
                  <p className="text-xs text-text-primary mt-1 break-words">{m.task}</p>
                  {m.summary && (
                    <p className={`text-[10px] mt-1 font-mono break-words ${m.state === "failed" ? "text-red-400/80" : "text-text-muted"}`}>
                      {m.summary}
                    </p>
                  )}
                  {m.ledgerSession && (
                    <p className="text-[9px] text-text-muted font-mono mt-0.5">ledger: {m.ledgerSession.slice(0, 8)}</p>
                  )}
                </li>
              ))}
            </ul>
          )}
        </div>

        {/* Evidence activity feed */}
        <div className="rounded-lg border border-border bg-surface/40 overflow-y-auto">
          <div className="px-3 py-2 border-b border-border text-[10px] font-mono tracking-widest text-text-muted">
            EVIDENCE ACTIVITY {evidence?.kind === "ready" && evidence.chain_intact ? "· ✓ chain intact" : ""}
          </div>
          {evidence?.kind === "ready" ? (
            <ul className="divide-y divide-border/60">
              {evidence.entries.slice(0, 30).map((e) => (
                <li key={e.id} className="px-3 py-2">
                  <div className="flex items-center gap-2">
                    <span className="text-[10px] font-mono text-primary">{e.action_id}</span>
                    <span className="ml-auto text-[9px] text-text-muted font-mono">{ago(e.timestamp)}</span>
                  </div>
                  <p className="text-[10px] text-text-muted font-mono mt-0.5">
                    {e.state} · session {e.session_id.slice(0, 8)}
                  </p>
                </li>
              ))}
            </ul>
          ) : (
            <p className="p-3 text-xs text-text-muted">
              {evidence?.kind === "unavailable"
                ? evidence.reason
                : evidence?.kind === "empty"
                  ? evidence.reason
                  : "No evidence yet — every build mission appends its verified outcome here, hash-chained."}
            </p>
          )}
        </div>
      </div>
    </div>
  );
};

export default PulsePanel;
