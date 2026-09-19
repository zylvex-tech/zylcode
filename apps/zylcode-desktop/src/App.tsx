import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { useStreamSubscription, ENVIRONMENT, IS_DESKTOP } from "./lib/events";
import {
  DESKTOP_REQUIRED_MESSAGE,
  ENVIRONMENT_UNKNOWN_MESSAGE,
  detectEnvironment,
  safeInvoke,
  getDiagnostics,
  clearDiagnostics,
  ENVIRONMENT_LABEL,
} from "./lib/runtime";
import {
  AUTONOMY_MODES,
  EVIDENCE_CATEGORIES,
  NOT_CAPTURED,
  newMissionId,
  type MissionRecord,
} from "./lib/mission";
import { StatusBadge, RungBadge, type CapabilityStatus } from "./components/CapabilityStatus";
import TokenMetricsWidget from "./components/TokenMetricsWidget";
import McpInspector from "./components/McpInspector";
import ArtifactViewer from "./components/ArtifactViewer";
import ProviderSettings from "./components/ProviderSettings";
import { useArtifactStream } from "./lib/useArtifactStream";
import { StatusBar } from "./components/StatusBar";
import VerificationRungBadge from "./components/VerificationRungBadge";
import { ThemeProvider, useTheme, ThemeSelector } from "./components/ui";
import { Panel, SectionTitle } from "./components/Panel";
import Forge from "./components/Forge";

// ---------------------------------------------------------------------------
// Backend result shapes (match src-tauri commands)
// ---------------------------------------------------------------------------

type IntentResult = {
  summary: string;
  artifacts: { kind: string; label: string; content: string }[];
  success: boolean;
};

type VerificationReport = {
  passed: boolean;
  rung: number;
  checks: { name: string; passed: boolean; message: string }[];
  duration_ms: number;
};

// ---------------------------------------------------------------------------
// Navigation model (v3 §3)
// ---------------------------------------------------------------------------

type AppArea =
  | "HOME"
  | "PROJECTS"
  | "MISSIONS"
  | "DESIGN"
  | "RUNTIME"
  | "EVIDENCE"
  | "FORGE";

type ProjectTab =
  | "OVERVIEW"
  | "CODE"
  | "DESIGN"
  | "MISSIONS"
  | "RUNTIME"
  | "ARTIFACTS"
  | "EVIDENCE"
  | "EXTENSIONS";

type RightDockModule = "AGENT" | "PREVIEW" | "BROWSER" | "TERMINAL" | "INSPECTOR" | "PROOF";

const APP_AREAS: { area: AppArea; label: string; status: CapabilityStatus }[] = [
  { area: "HOME", label: "Home", status: "AVAILABLE" },
  { area: "PROJECTS", label: "Projects", status: "AVAILABLE" },
  { area: "MISSIONS", label: "Missions", status: "LIMITED" },
  { area: "DESIGN", label: "Design", status: "COMING SOON" },
  { area: "RUNTIME", label: "Runtime", status: "LIMITED" },
  { area: "EVIDENCE", label: "Evidence", status: "LIMITED" },
  { area: "FORGE", label: "Forge", status: "LIMITED" },
];

const PROJECT_TABS: { tab: ProjectTab; label: string; status: CapabilityStatus }[] = [
  { tab: "OVERVIEW", label: "Overview", status: "AVAILABLE" },
  { tab: "CODE", label: "Code", status: "LIMITED" },
  { tab: "DESIGN", label: "Design", status: "COMING SOON" },
  { tab: "MISSIONS", label: "Missions", status: "LIMITED" },
  { tab: "RUNTIME", label: "Runtime", status: "LIMITED" },
  { tab: "ARTIFACTS", label: "Artifacts", status: "LIMITED" },
  { tab: "EVIDENCE", label: "Evidence", status: "LIMITED" },
  { tab: "EXTENSIONS", label: "Extensions", status: "COMING SOON" },
];

const RIGHT_DOCK_MODULES: { id: RightDockModule; label: string }[] = [
  { id: "AGENT", label: "Agent" },
  { id: "PREVIEW", label: "Preview" },
  { id: "BROWSER", label: "Browser" },
  { id: "TERMINAL", label: "Terminal" },
  { id: "INSPECTOR", label: "Inspector" },
  { id: "PROOF", label: "Proof" },
];

// ---------------------------------------------------------------------------
// Small building blocks
// ---------------------------------------------------------------------------

/** Controlled backend-dependent panel state (v3 §11) — never a raw exception. */
function DesktopRequired() {
  const env = detectEnvironment();
  const message =
    env === "UNKNOWN" ? ENVIRONMENT_UNKNOWN_MESSAGE : DESKTOP_REQUIRED_MESSAGE;
  return (
    <div className="flex flex-col items-center justify-center gap-2 py-8 text-center">
      <StatusBadge status="LIMITED" />
      <p className="text-sm text-text-muted max-w-xs">{message}</p>
      <p className="text-[11px] text-text-muted/70">
        Technical details are available in Developer Tools → Diagnostics.
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Mission Composer (v3 §6–§8)
// ---------------------------------------------------------------------------

function MissionComposer({
  onMissionStart,
  busy,
  streaming,
}: {
  onMissionStart: (goal: string, autonomy: string, model: string | null) => void;
  busy: boolean;
  streaming: boolean;
}) {
  const [goal, setGoal] = useState("");
  const [autonomy, setAutonomy] = useState("GUIDED");
  const [model, setModel] = useState("");

  const autonomyInfo = AUTONOMY_MODES.find((m) => m.mode === autonomy) ?? AUTONOMY_MODES[1];

  return (
    <div className="space-y-3">
      <textarea
        value={goal}
        onChange={(e) => setGoal(e.target.value)}
        rows={3}
        placeholder="Describe the mission — what should change, and what proves it worked?"
        className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary focus:ring-2 focus:ring-primary/20 transition-all resize-none"
      />

      <div className="flex flex-wrap items-center gap-1.5 text-[11px] text-text-muted">
        <span className="rounded border border-border px-1.5 py-0.5" title="Attach context files — desktop runtime">
          + Context
        </span>
        <span className="rounded border border-border px-1.5 py-0.5" title="Reference project files">
          @ Files
        </span>
        <span className="rounded border border-border px-1.5 py-0.5" title="Recent missions">
          # History
        </span>
        <span className="rounded border border-border px-1.5 py-0.5" title="Commands — limited set available">
          / Commands
        </span>
        <span className="rounded border border-border px-1.5 py-0.5 opacity-60" title="Skills — coming soon">
          $ Skills <StatusBadge status="COMING SOON" size="xs" />
        </span>
        <span className="rounded border border-border px-1.5 py-0.5" title="Capabilities from the registry">
          ⚡ Capabilities
        </span>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
        <label className="text-xs text-text-muted space-y-1 block">
          <span>Autonomy</span>
          <select
            value={autonomy}
            onChange={(e) => setAutonomy(e.target.value)}
            className="w-full rounded-md border border-border bg-background px-2 py-1.5 text-sm outline-none focus:border-primary"
          >
            {AUTONOMY_MODES.map((m) => (
              <option key={m.mode} value={m.mode} disabled={m.status === "COMING SOON"}>
                {m.label} — {m.status}
              </option>
            ))}
          </select>
        </label>
        <label className="text-xs text-text-muted space-y-1 block">
          <span>Model override (optional)</span>
          <input
            value={model}
            onChange={(e) => setModel(e.target.value)}
            placeholder="provider/model"
            className="w-full rounded-md border border-border bg-background px-2 py-1.5 text-sm font-mono outline-none focus:border-primary"
          />
        </label>
      </div>

      <div className="flex items-center justify-between gap-2">
        <p className="text-[11px] text-text-muted">{autonomyInfo.description}</p>
        <button
          onClick={() => onMissionStart(goal, autonomy, model.trim() || null)}
          disabled={busy || !goal.trim() || autonomyInfo.status === "COMING SOON"}
          className="bg-primary text-primary-foreground px-4 py-2 rounded-md text-sm font-medium hover:bg-primary-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {streaming ? (
            <span className="flex items-center gap-2">
              <svg className="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              Running…
            </span>
          ) : (
            "RUN MISSION"
          )}
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Evidence Center (v3 §12) — real evidence only; missing → NOT CAPTURED
// ---------------------------------------------------------------------------

function EvidenceCenter({ missions }: { missions: MissionRecord[] }) {
  if (missions.length === 0) {
    return (
      <Panel title="EVIDENCE CENTER">
        <p className="text-sm text-text-muted">
          No missions yet — run a mission to generate verifiable evidence.
        </p>
      </Panel>
    );
  }
  return (
    <div className="space-y-3">
      {missions.map((m) => (
        <Panel key={m.id} title={`EVIDENCE — ${m.title}`}>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 text-xs">
            {EVIDENCE_CATEGORIES.map((cat) => {
              const has =
                (cat === "Tool actions" && m.toolCalls.length > 0) ||
                (cat === "Artifacts" && m.artifacts.length > 0) ||
                (cat === "Verification" && m.verification !== null);
              return (
                <div key={cat} className="rounded border border-border px-2 py-1.5 flex items-center justify-between gap-2">
                  <span className="truncate">{cat}</span>
                  {has ? (
                    <StatusBadge status="AVAILABLE" size="xs" />
                  ) : (
                    <span className="font-mono text-[10px] text-text-muted shrink-0">{NOT_CAPTURED}</span>
                  )}
                </div>
              );
            })}
          </div>
        </Panel>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Proof Inspector (v3 §13) — visualization of currently available evidence
// ---------------------------------------------------------------------------

function ProofInspector({ mission }: { mission: MissionRecord | null }) {
  if (!mission) {
    return (
      <Panel title="PROOF INSPECTOR">
        <p className="text-sm text-text-muted">Select or run a mission to inspect its proof chain.</p>
      </Panel>
    );
  }
  const chain: { step: string; value: string }[] = [
    { step: "GOAL", value: mission.goal || NOT_CAPTURED },
    {
      step: "CLAIMS",
      value:
        mission.acceptanceCriteria.length > 0
          ? mission.acceptanceCriteria.join("; ")
          : NOT_CAPTURED,
    },
    {
      step: "ACTIONS",
      value: mission.toolCalls.length > 0 ? `${mission.toolCalls.length} tool call(s)` : NOT_CAPTURED,
    },
    {
      step: "STATE CHANGES",
      value: mission.artifacts.length > 0 ? `${mission.artifacts.length} artifact(s)` : NOT_CAPTURED,
    },
    {
      step: "TESTS",
      value: mission.verification ? `${mission.verification.checks.length} check(s)` : NOT_CAPTURED,
    },
    {
      step: "ARTIFACTS",
      value:
        mission.artifacts.length > 0
          ? mission.artifacts.map((a) => a.label).join(", ")
          : NOT_CAPTURED,
    },
    {
      step: "EVIDENCE",
      value: mission.deltas.length > 0 ? `${mission.deltas.length} stream record(s)` : NOT_CAPTURED,
    },
    { step: "OUTCOME", value: mission.finalOutcome ?? NOT_CAPTURED },
  ];
  return (
    <Panel
      title="PROOF INSPECTOR"
      right={
        <RungBadge rung={mission.verification ? `R${mission.verification.rung}` : "unverified"} />
      }
    >
      <div className="space-y-1.5">
        {chain.map((c) => (
          <div key={c.step} className="flex items-start gap-2 text-xs">
            <span className="font-mono text-[10px] text-text-muted w-28 shrink-0">{c.step}</span>
            <span className="break-words min-w-0">{c.value}</span>
          </div>
        ))}
      </div>
    </Panel>
  );
}

// ---------------------------------------------------------------------------
// Runtime Lab (v3 §10) — truthful target statuses
// ---------------------------------------------------------------------------

function RuntimeLab() {
  const targets: { name: string; status: CapabilityStatus; note: string }[] = [
    { name: "WEB", status: "AVAILABLE", note: "Vite dev/preview of the current project" },
    {
      name: "DESKTOP",
      status: IS_DESKTOP ? "AVAILABLE" : "LIMITED",
      note: IS_DESKTOP ? "Tauri shell active" : "Requires the Tauri desktop shell",
    },
    { name: "TERMINAL", status: "LIMITED", note: "zylcode CLI (repo-context, build)" },
    { name: "ANDROID", status: "NOT INSTALLED", note: "Android execution not commissioned" },
    { name: "IOS", status: "NOT INSTALLED", note: "iOS execution not commissioned" },
    { name: "CONTAINER", status: "NOT INSTALLED", note: "Container runtime not commissioned" },
    { name: "REMOTE", status: "NOT INSTALLED", note: "Remote targets not commissioned" },
    {
      name: "LOGS",
      status: IS_DESKTOP ? "AVAILABLE" : "LIMITED",
      note: IS_DESKTOP ? "Live streams active" : "Streams require desktop runtime",
    },
  ];
  return (
    <div className="grid grid-cols-2 md:grid-cols-4 gap-2">
      {targets.map((t) => (
        <Panel key={t.name}>
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className="font-mono text-xs font-semibold">{t.name}</span>
              <StatusBadge status={t.status} size="xs" />
            </div>
            <p className="text-[11px] text-text-muted">{t.note}</p>
          </div>
        </Panel>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Diagnostics (raw errors live here, not in normal UI)
// ---------------------------------------------------------------------------

function DiagnosticsPanel() {
  const [, force] = useState(0);
  const entries = getDiagnostics();
  return (
    <Panel
      title="DIAGNOSTICS"
      right={
        <button
          onClick={() => {
            clearDiagnostics();
            force((n) => n + 1);
          }}
          className="text-[11px] text-text-muted hover:text-text-primary underline"
        >
          Clear
        </button>
      }
    >
      {entries.length === 0 ? (
        <p className="text-xs text-text-muted">No diagnostics recorded.</p>
      ) : (
        <div className="space-y-1 max-h-48 overflow-y-auto">
          {entries.map((d, i) => (
            <div key={i} className="font-mono text-[10px] text-text-muted break-all">
              [{d.timestamp}] {d.source}: {d.message}
            </div>
          ))}
        </div>
      )}
    </Panel>
  );
}

// ---------------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------------

function Home({
  onOpenProject,
  missions,
}: {
  onOpenProject: () => void;
  missions: MissionRecord[];
}) {
  const recent = missions.slice(-3).reverse();
  return (
    <div className="space-y-4">
      <Panel title="PROJECTS">
        <div className="flex items-center justify-between gap-3">
          <div>
            <p className="text-sm font-medium">zylcode</p>
            <p className="text-[11px] text-text-muted font-mono">C:\Projects\zylcode · main</p>
          </div>
          <button
            onClick={onOpenProject}
            className="bg-primary text-primary-foreground px-3 py-1.5 rounded-md text-sm font-medium hover:bg-primary-hover"
          >
            Open project
          </button>
        </div>
      </Panel>
      <Panel title="RECENT MISSIONS">
        {recent.length === 0 ? (
          <p className="text-sm text-text-muted">No missions yet.</p>
        ) : (
          <div className="space-y-1.5">
            {recent.map((m) => (
              <div key={m.id} className="flex items-center justify-between gap-2 text-sm">
                <span className="truncate">{m.title}</span>
                <span className="font-mono text-[10px] text-text-muted shrink-0">{m.state}</span>
              </div>
            ))}
          </div>
        )}
      </Panel>
      <Panel title="CAPABILITY SPOTLIGHT">
        <div className="flex flex-wrap gap-x-4 gap-y-2">
          <span className="flex items-center gap-1.5 text-xs text-text-muted">
            <StatusBadge status="AVAILABLE" size="xs" /> Missions (intent pipeline)
          </span>
          <span className="flex items-center gap-1.5 text-xs text-text-muted">
            <StatusBadge status="AVAILABLE" size="xs" /> Repository Intelligence
          </span>
          <span className="flex items-center gap-1.5 text-xs text-text-muted">
            <StatusBadge status="LIMITED" size="xs" /> Runtime (web/desktop)
          </span>
          <span className="flex items-center gap-1.5 text-xs text-text-muted">
            <StatusBadge status="COMING SOON" size="xs" /> Design Studio
          </span>
          <span className="flex items-center gap-1.5 text-xs text-text-muted">
            <StatusBadge status="COMING SOON" size="xs" /> Multi-agent board
          </span>
        </div>
      </Panel>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main app
// ---------------------------------------------------------------------------

function AppContent() {
  const [area, setArea] = useState<AppArea>("HOME");
  const [projectTab, setProjectTab] = useState<ProjectTab>("OVERVIEW");
  const [dockModule, setDockModule] = useState<RightDockModule>("AGENT");
  const [devDrawerOpen, setDevDrawerOpen] = useState(false);

  const [missions, setMissions] = useState<MissionRecord[]>([]);
  const [activeMissionId, setActiveMissionId] = useState<string | null>(null);
  const [verification, setVerification] = useState<VerificationReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [metricsKey, setMetricsKey] = useState(0);
  const { currentTheme, setTheme } = useTheme();

  const { deltas, done, mcpCalls, isStreaming } = useStreamSubscription(true);

  const {
    files,
    activeTab,
    setActiveTab,
    diffMode,
    setDiffMode,
    saveStatus,
    saveFile,
    applyPatch,
    closeTab,
  } = useArtifactStream(deltas);

  const activeMission = missions.find((m) => m.id === activeMissionId) ?? null;

  useEffect(() => {
    if (done) setMetricsKey((k) => k + 1);
  }, [done]);

  // Fold real stream state into the active mission record.
  useEffect(() => {
    if (!activeMissionId) return;
    setMissions((prev) =>
      prev.map((m) =>
        m.id === activeMissionId
          ? {
              ...m,
              state: done ? (verification ? (verification.passed ? "COMPLETE" : "FAILED") : "VERIFYING") : "RUNNING",
              deltas: deltas.map((d) => ({ index: d.index, delta: d.delta, phase: d.phase })),
              toolCalls: mcpCalls,
              artifacts: done?.artifacts ?? m.artifacts,
              finalOutcome: done?.summary ?? m.finalOutcome,
              updatedAt: new Date().toISOString(),
            }
          : m,
      ),
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeMissionId, deltas, mcpCalls, done]);

  const startMission = useCallback(
    async (goal: string, autonomy: string, model: string | null) => {
      const id = newMissionId();
      const now = new Date().toISOString();
      const mission: MissionRecord = {
        id,
        title: goal.length > 42 ? goal.slice(0, 42) + "…" : goal,
        goal,
        state: "RUNNING",
        autonomy: autonomy as MissionRecord["autonomy"],
        model,
        createdAt: now,
        updatedAt: now,
        acceptanceCriteria: [],
        deltas: [],
        toolCalls: [],
        artifacts: [],
        verification: null,
        finalOutcome: null,
      };
      setMissions((prev) => [...prev, mission]);
      setActiveMissionId(id);
      setArea("MISSIONS");
      setProjectTab("MISSIONS");
      setBusy(true);
      setError(null);
      setVerification(null);

      // Real pipeline: process_intent_stream via the safe bridge. In browser
      // preview this returns a controlled "desktop runtime required" result
      // instead of a raw exception.
      const result = await safeInvoke<IntentResult>(ENVIRONMENT, "process_intent_stream", {
        prompt: goal,
        model,
      });
      if (!result.ok && IS_DESKTOP) {
        setError(result.reason);
      } else if (!result.ok) {
        // Browser preview: controlled state, not an error banner.
        setMissions((prev) =>
          prev.map((m) =>
            m.id === id
              ? { ...m, state: "BLOCKED", finalOutcome: "Desktop runtime required" }
              : m,
          ),
        );
      }
      setBusy(false);
    },
    [],
  );

  const verifyMission = useCallback(async () => {
    if (!activeMissionId) return;
    setBusy(true);
    setError(null);
    const result = await safeInvoke<VerificationReport>(ENVIRONMENT, "verify_logic");
    if (result.ok) {
      setVerification(result.data);
      const report = result.data;
      setMissions((prev) =>
        prev.map((m) =>
          m.id === activeMissionId
            ? { ...m, verification: report, state: report.passed ? "COMPLETE" : "FAILED" }
            : m,
        ),
      );
    } else if (!IS_DESKTOP) {
      setMissions((prev) =>
        prev.map((m) => (m.id === activeMissionId ? { ...m, state: "BLOCKED" } : m)),
      );
    } else {
      setError(result.reason);
    }
    setBusy(false);
  }, [activeMissionId]);

  const inProject = area !== "HOME" && area !== "FORGE";

  return (
    <div className="min-h-screen bg-background text-text-primary flex flex-col">
      {/* Header: project/branch, mission status, evidence */}
      <header className="border-b border-border bg-surface/80 backdrop-blur-sm px-4 py-2.5 flex items-center justify-between gap-4 sticky top-0 z-40">
        <div className="flex items-center gap-3 min-w-0">
          <h1 className="text-base font-bold tracking-tight truncate">
            <span className="bg-gradient-to-r from-primary to-accent bg-clip-text text-transparent">
              ZylCode
            </span>
            <span className="font-mono text-xs font-normal text-text-muted ml-2">v0.3.0-convergence</span>
          </h1>
          {inProject && (
            <span className="hidden md:inline text-xs text-text-muted font-mono truncate">
              zylcode · main
            </span>
          )}
        </div>
        <div className="flex items-center gap-3">
          {activeMission && (
            <span className="hidden sm:flex items-center gap-1.5 text-xs">
              <span className="font-mono text-[10px] text-text-muted">{activeMission.state}</span>
            </span>
          )}
          <TokenMetricsWidget refreshKey={metricsKey} />
          <ThemeSelector />
        </div>
      </header>

      <div className="flex flex-1 min-h-0">
        {/* LEFT: navigation */}
        <nav className="w-44 shrink-0 border-r border-border bg-surface/50 flex flex-col py-3">
          <div className="px-3 space-y-0.5">
            {APP_AREAS.map(({ area: a, label, status }) => (
              <button
                key={a}
                onClick={() => setArea(a)}
                className={`w-full flex items-center justify-between px-2.5 py-1.5 rounded-md text-sm transition-colors ${
                  area === a
                    ? "bg-primary/10 text-primary font-medium"
                    : "text-text-muted hover:text-text-primary hover:bg-surface"
                }`}
              >
                <span>{label}</span>
                {status !== "AVAILABLE" && <StatusBadge status={status} size="xs" />}
              </button>
            ))}
          </div>
          <div className="mt-auto px-3">
            <button
              onClick={() => setDevDrawerOpen((v) => !v)}
              className="w-full text-left px-2.5 py-1.5 rounded-md text-xs text-text-muted hover:text-text-primary hover:bg-surface"
            >
              Developer Tools {devDrawerOpen ? "▾" : "▸"}
            </button>
          </div>
        </nav>

        {/* CENTER: work surface */}
        <main className="flex-1 flex flex-col min-h-0 overflow-y-auto">
          {area === "HOME" && (
            <div className="p-4">
              <Home
                missions={missions}
                onOpenProject={() => {
                  setArea("MISSIONS");
                  setProjectTab("MISSIONS");
                }}
              />
            </div>
          )}

          {area === "FORGE" && (
            <div className="p-4">
              <Forge />
            </div>
          )}

          {inProject && (
            <div className="flex flex-col min-h-0 flex-1">
              {/* Project tabs */}
              <div className="flex items-center gap-1 px-3 py-2 border-b border-border overflow-x-auto">
                {PROJECT_TABS.map(({ tab, label, status }) => (
                  <button
                    key={tab}
                    onClick={() => setProjectTab(tab)}
                    className={`px-2.5 py-1 rounded-md text-xs whitespace-nowrap transition-colors ${
                      projectTab === tab
                        ? "bg-primary/10 text-primary font-medium"
                        : "text-text-muted hover:text-text-primary"
                    }`}
                  >
                    {label}
                    {status !== "AVAILABLE" && (
                      <span className="ml-1 inline-block align-middle">
                        <StatusBadge status={status} size="xs" />
                      </span>
                    )}
                  </button>
                ))}
              </div>

              <div className="flex-1 min-h-0 p-3">
                {projectTab === "OVERVIEW" && (
                  <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
                    <Panel title="PROJECT">
                      <div className="space-y-1 text-xs">
                        <p><span className="text-text-muted">Name:</span> zylcode</p>
                        <p><span className="text-text-muted">Repository:</span> <span className="font-mono">github.com/zylvex-tech/zylcode</span></p>
                        <p><span className="text-text-muted">Branch:</span> <span className="font-mono">main</span></p>
                        <p>
                          <span className="text-text-muted">Runtime state:</span>{" "}
                          {IS_DESKTOP ? "Desktop runtime active" : "Browser preview (desktop engine idle)"}
                        </p>
                        <p><span className="text-text-muted">Current mission:</span> {activeMission ? activeMission.title : "none"}</p>
                        <p>
                          <span className="text-text-muted">Evidence status:</span>{" "}
                          {missions.some((m) => m.verification) ? "verification present" : NOT_CAPTURED}
                        </p>
                      </div>
                    </Panel>
                    <Panel title="ENVIRONMENT">
                      <div className="space-y-1 text-xs">
                        <p>
                          <span className="text-text-muted">Detected:</span>{" "}
                          <span className="font-mono">{ENVIRONMENT}</span> ({ENVIRONMENT_LABEL[ENVIRONMENT]})
                        </p>
                        <p className="text-text-muted">
                          {IS_DESKTOP
                            ? "Full engine available: intents, tools, providers, verification."
                            : "Browser preview: the desktop engine is not connected. Backend panels show controlled states instead of errors."}
                        </p>
                      </div>
                    </Panel>
                    <Panel title="CAPABILITIES" className="lg:col-span-2">
                      <div className="flex flex-wrap gap-x-4 gap-y-2 text-xs">
                        <span className="flex items-center gap-1.5">Missions <StatusBadge status="LIMITED" size="xs" /></span>
                        <span className="flex items-center gap-1.5">Repository Intelligence <StatusBadge status="AVAILABLE" size="xs" /></span>
                        <span className="flex items-center gap-1.5">Forge <StatusBadge status="LIMITED" size="xs" /></span>
                        <span className="flex items-center gap-1.5">Design <StatusBadge status="COMING SOON" size="xs" /></span>
                        <span className="flex items-center gap-1.5">Multi-agent <StatusBadge status="COMING SOON" size="xs" /></span>
                      </div>
                    </Panel>
                  </div>
                )}

                {projectTab === "MISSIONS" && (
                  <div className="grid grid-cols-1 xl:grid-cols-3 gap-3">
                    <Panel title="MISSION COMPOSER" className="xl:col-span-2">
                      <MissionComposer onMissionStart={startMission} busy={busy} streaming={isStreaming} />
                    </Panel>
                    <div className="space-y-3">
                      <Panel title="MISSIONS">
                        {missions.length === 0 ? (
                          <p className="text-xs text-text-muted">No missions yet.</p>
                        ) : (
                          <div className="space-y-1.5">
                            {missions
                              .slice()
                              .reverse()
                              .map((m) => (
                                <button
                                  key={m.id}
                                  onClick={() => setActiveMissionId(m.id)}
                                  className={`w-full text-left rounded border px-2 py-1.5 text-xs transition-colors ${
                                    m.id === activeMissionId
                                      ? "border-primary/50 bg-primary/5"
                                      : "border-border hover:border-primary/30"
                                  }`}
                                >
                                  <div className="flex items-center justify-between gap-2">
                                    <span className="truncate">{m.title}</span>
                                    <span className="font-mono text-[10px] text-text-muted shrink-0">{m.state}</span>
                                  </div>
                                </button>
                              ))}
                          </div>
                        )}
                      </Panel>
                      <Panel title="VERIFY">
                        <button
                          onClick={verifyMission}
                          disabled={busy || !activeMission}
                          className="w-full bg-secondary text-secondary-foreground px-3 py-2 rounded-md text-sm font-medium hover:bg-secondary-hover disabled:opacity-50"
                        >
                          Verify active mission
                        </button>
                        {verification && (
                          <div className="mt-2 space-y-1">
                            <VerificationRungBadge rung={verification.rung} />
                            {verification.checks.map((c) => (
                              <p key={c.name} className="text-[11px]">
                                {c.passed ? "✓" : "✗"} {c.name}: {c.message}
                              </p>
                            ))}
                          </div>
                        )}
                      </Panel>
                    </div>
                  </div>
                )}

                {projectTab === "RUNTIME" && <RuntimeLab />}

                {projectTab === "EVIDENCE" && <EvidenceCenter missions={missions} />}

                {projectTab === "CODE" && (
                  <div className="space-y-3">
                    <ArtifactViewer
                      files={files}
                      activeTab={activeTab}
                      setActiveTab={setActiveTab}
                      diffMode={diffMode}
                      setDiffMode={setDiffMode}
                      saveStatus={saveStatus}
                      saveFile={saveFile}
                      applyPatch={applyPatch}
                      closeTab={closeTab}
                    />
                    <p className="text-[11px] text-text-muted">
                      Code surface shows artifacts produced by missions. Full repository browsing is PROPOSED.
                    </p>
                  </div>
                )}

                {(projectTab === "DESIGN" || projectTab === "EXTENSIONS") && (
                  <Panel title={projectTab === "DESIGN" ? "DESIGN STUDIO" : "EXTENSIONS"}>
                    <div className="flex flex-col items-center gap-2 py-10 text-center">
                      <StatusBadge status="COMING SOON" />
                      <p className="text-sm text-text-muted max-w-sm">
                        {projectTab === "DESIGN"
                          ? "The Design workspace (canvas, tokens, screens) is defined in Product Architecture v3 and awaits implementation."
                          : "Extensions integrate with Forge capability packs. The Forge shell is available under Forge."}
                      </p>
                    </div>
                  </Panel>
                )}

                {projectTab === "ARTIFACTS" && (
                  <Panel title="ARTIFACTS">
                    {missions.flatMap((m) => m.artifacts).length === 0 ? (
                      <p className="text-sm text-text-muted">
                        No artifacts yet — mission artifacts will appear here.{" "}
                        <span className="font-mono text-[10px]">{NOT_CAPTURED}</span>
                      </p>
                    ) : (
                      <div className="space-y-1.5 text-xs">
                        {missions
                          .flatMap((m) => m.artifacts)
                          .map((a, i) => (
                            <div key={i} className="flex items-center justify-between rounded border border-border px-2 py-1.5">
                              <span>{a.label}</span>
                              <span className="font-mono text-[10px] text-text-muted">{a.kind}</span>
                            </div>
                          ))}
                      </div>
                    )}
                  </Panel>
                )}
              </div>
            </div>
          )}
        </main>

        {/* RIGHT: context dock */}
        <aside className="hidden lg:flex w-[320px] shrink-0 border-l border-border bg-surface/50 flex-col">
          <div className="flex items-center gap-1 px-2 py-2 border-b border-border overflow-x-auto">
            {RIGHT_DOCK_MODULES.map((m) => (
              <button
                key={m.id}
                onClick={() => setDockModule(m.id)}
                className={`px-2 py-1 rounded text-[11px] whitespace-nowrap transition-colors ${
                  dockModule === m.id
                    ? "bg-primary/10 text-primary font-medium"
                    : "text-text-muted hover:text-text-primary"
                }`}
              >
                {m.label}
              </button>
            ))}
          </div>
          <div className="flex-1 min-h-0 overflow-y-auto p-3 space-y-3">
            {dockModule === "AGENT" && (
              <>
                {activeMission ? (
                  <ProofInspector mission={activeMission} />
                ) : (
                  <Panel title="AGENT">
                    <p className="text-xs text-text-muted">No active mission. Compose one under Missions.</p>
                  </Panel>
                )}
                <Panel title="STREAM">
                  {deltas.length === 0 ? (
                    <p className="text-xs text-text-muted">Idle. Run a mission to see the agent stream.</p>
                  ) : (
                    <div className="max-h-56 overflow-y-auto space-y-0.5">
                      {deltas.slice(-40).map((d, i) => (
                        <p key={i} className="font-mono text-[10px] text-text-muted break-words">
                          {d.delta}
                        </p>
                      ))}
                    </div>
                  )}
                </Panel>
              </>
            )}
            {dockModule === "PROOF" && <ProofInspector mission={activeMission} />}
            {dockModule === "PREVIEW" && (
              <Panel title="PREVIEW">
                <p className="text-xs text-text-muted">
                  Artifact preview opens with mission artifacts; the Runtime tab handles run targets.
                </p>
              </Panel>
            )}
            {dockModule === "BROWSER" && (
              <Panel title="BROWSER">
                <div className="flex flex-col items-center gap-2 py-6 text-center">
                  <StatusBadge status="COMING SOON" />
                  <p className="text-xs text-text-muted">Embedded browser control is not commissioned.</p>
                </div>
              </Panel>
            )}
            {dockModule === "TERMINAL" && (
              <Panel title="TERMINAL">
                <div className="flex flex-col items-center gap-2 py-6 text-center">
                  <StatusBadge status="COMING SOON" />
                  <p className="text-xs text-text-muted">
                    Embedded terminal is not commissioned. The zylcode CLI is available in your shell.
                  </p>
                </div>
              </Panel>
            )}
            {dockModule === "INSPECTOR" && (
              <Panel title="INSPECTOR">
                <div className="flex flex-col items-center gap-2 py-6 text-center">
                  <StatusBadge status="COMING SOON" />
                  <p className="text-xs text-text-muted">UI inspector is not commissioned.</p>
                </div>
              </Panel>
            )}
          </div>
        </aside>
      </div>

      {/* Developer Tools drawer (relocated harness surfaces) */}
      <AnimatePresence>
        {devDrawerOpen && (
          <motion.aside
            initial={{ y: "100%" }}
            animate={{ y: 0 }}
            exit={{ y: "100%" }}
            transition={{ type: "tween", duration: 0.2 }}
            className="fixed bottom-0 left-0 right-0 z-50 max-h-[60vh] overflow-y-auto border-t border-border bg-surface shadow-2xl"
          >
            <div className="flex items-center justify-between px-4 py-2 border-b border-border sticky top-0 bg-surface">
              <SectionTitle>DEVELOPER TOOLS</SectionTitle>
              <button
                onClick={() => setDevDrawerOpen(false)}
                className="text-xs text-text-muted hover:text-text-primary"
              >
                close ✕
              </button>
            </div>
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-3 p-3">
              <Panel title="MCP ACTIVITY">
                {IS_DESKTOP ? <McpInspector /> : <DesktopRequired />}
              </Panel>
              <Panel title="PROVIDERS — FALLBACK CHAIN">
                {IS_DESKTOP ? <ProviderSettings /> : <DesktopRequired />}
              </Panel>
              <Panel title="DIAGNOSTICS">
                <DiagnosticsPanel />
              </Panel>
              <Panel title="ENVIRONMENT">
                <p className="text-xs">
                  <span className="text-text-muted">Detected:</span>{" "}
                  <span className="font-mono">{ENVIRONMENT}</span>
                </p>
                <p className="text-[11px] text-text-muted mt-1">
                  Outside the desktop runtime, backend calls are suppressed and surfaced here instead
                  of as UI errors.
                </p>
              </Panel>
            </div>
          </motion.aside>
        )}
      </AnimatePresence>

      {/* Status strip */}
      <StatusBar theme={currentTheme} onThemeChange={setTheme} mcpBridgeCount={0} />
    </div>
  );
}

export default function App() {
  return (
    <ThemeProvider>
      <AppContent />
    </ThemeProvider>
  );
}
