import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { useStreamSubscription, ENVIRONMENT, IS_DESKTOP } from "./lib/events";
import {
  DESKTOP_REQUIRED_MESSAGE,
  safeInvoke,
  getDiagnostics,
  clearDiagnostics,
} from "./lib/runtime";
import {
  AUTONOMY_MODES,
  EVIDENCE_CATEGORIES,
  NOT_CAPTURED,
  newMissionId,
  type MissionRecord,
} from "./lib/mission";
import { StatusBadge, type CapabilityStatus } from "./components/CapabilityStatus";
import TokenMetricsWidget from "./components/TokenMetricsWidget";
import { useArtifactStream } from "./lib/useArtifactStream";
import { StatusBar } from "./components/StatusBar";
import VerificationRungBadge from "./components/VerificationRungBadge";
import { ThemeProvider, useTheme, ThemeSelector } from "./components/ui";
import { Panel, SectionTitle } from "./components/Panel";
import Forge from "./components/Forge";
import { ActivityRail, ContextSidebar, AgentDock, BottomPanel, SurfaceHost } from "./components/shell";

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
// Navigation model
// ---------------------------------------------------------------------------

type ActivityId =
  | "explorer"
  | "search"
  | "source-control"
  | "missions"
  | "run"
  | "evidence"
  | "forge"
  | "extensions"
  | "settings"
  | "account";

type SurfaceType =
  | "home"
  | "overview"
  | "intel"
  | "source-control"
  | "code"
  | "design"
  | "missions"
  | "runtime"
  | "artifacts"
  | "evidence"
  | "extensions"
  | "forge";

type DockModule = "agent" | "preview" | "proof" | "mcp" | "providers" | "diagnostics";

type BottomTab = "terminal" | "output" | "problems" | "tests" | "evidence" | "dev-tools";

const ACTIVITY_TO_SURFACE: Record<ActivityId, SurfaceType> = {
  explorer: "intel",
  search: "overview",
  "source-control": "source-control",
  missions: "missions",
  run: "runtime",
  evidence: "evidence",
  forge: "forge",
  extensions: "extensions",
  settings: "overview",
  account: "overview",
};

const BOTTOM_UTILITIES: { id: string; label: string }[] = [
  { id: "settings", label: "Settings" },
  { id: "account", label: "Account" },
];

// ---------------------------------------------------------------------------
// Main app
// ---------------------------------------------------------------------------

function AppContent() {
  const [activeActivity, setActiveActivity] = useState<ActivityId>("explorer");
  const [activeSurface, setActiveSurface] = useState<SurfaceType>("overview");
  const [activeDockModule, setActiveDockModule] = useState<DockModule>("agent");
  const [bottomPanelOpen, setBottomPanelOpen] = useState(false);
  const [activeBottomTab, setActiveBottomTab] = useState<BottomTab>("terminal");

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
      setActiveActivity("missions");
      setActiveSurface("missions");
      setBusy(true);
      setError(null);
      setVerification(null);

      const result = await safeInvoke<IntentResult>(ENVIRONMENT, "process_intent_stream", {
        prompt: goal,
        model,
      });
      if (!result.ok && IS_DESKTOP) {
        setError(result.reason);
      } else if (!result.ok) {
        setMissions((prev) =>
          prev.map((m) =>
            m.id === id ? { ...m, state: "BLOCKED", finalOutcome: "Desktop runtime required" } : m,
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

  const handleActivityChange = useCallback((activity: ActivityId) => {
    setActiveActivity(activity);
    setActiveSurface(ACTIVITY_TO_SURFACE[activity] || "overview");
  }, []);

  const handleBottomUtilityClick = useCallback((id: string) => {
    if (id === "settings") {
      setActiveActivity("explorer");
      setActiveSurface("overview");
    }
  }, []);

  return (
    <div className="min-h-screen bg-background text-text-primary flex flex-col">
      {/* Header */}
      <header className="border-b border-border bg-surface/80 backdrop-blur-sm px-4 py-2.5 flex items-center justify-between gap-4 sticky top-0 z-40">
        <div className="flex items-center gap-3 min-w-0">
          <h1 className="text-base font-bold tracking-tight truncate">
            <span className="bg-gradient-to-r from-primary to-accent bg-clip-text text-transparent">
              ZylCode
            </span>
            <span className="font-mono text-xs font-normal text-text-muted ml-2">v0.3.0-convergence</span>
          </h1>
          {activeSurface !== "home" && (
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
        {/* LEFT: Activity Rail */}
        <ActivityRail
          activeActivity={activeActivity}
          onActivityChange={handleActivityChange}
          onBottomUtilityClick={handleBottomUtilityClick}
        />

        {/* CONTEXT SIDEBAR */}
        <ContextSidebar activeActivity={activeActivity} />

        {/* CENTER: Surface Host */}
        <main className="flex-1 flex flex-col min-h-0 overflow-y-auto">
          <SurfaceHost
            surface={activeSurface}
            activeMission={activeMission}
            missions={missions}
            verification={verification}
            busy={busy}
            isStreaming={isStreaming}
            onMissionStart={startMission}
            onVerifyMission={verifyMission}
            onOpenProject={() => {
              setActiveActivity("missions");
              setActiveSurface("missions");
            }}
            activeTab={activeTab}
            setActiveTab={setActiveTab}
            files={files}
            diffMode={diffMode}
            setDiffMode={setDiffMode}
            saveStatus={saveStatus}
            saveFile={saveFile}
            applyPatch={applyPatch}
            closeTab={closeTab}
            deltas={deltas}
          />
        </main>

        {/* RIGHT: Agent Dock */}
        <AgentDock
          activeModule={activeDockModule}
          onModuleChange={setActiveDockModule}
          activeMission={activeMission}
          deltas={deltas}
          isStreaming={isStreaming}
        />
      </div>

      {/* BOTTOM PANEL */}
      <BottomPanel
        isOpen={bottomPanelOpen}
        onToggle={() => setBottomPanelOpen((v) => !v)}
        activeTab={activeBottomTab}
        onTabChange={setActiveBottomTab}
      />

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