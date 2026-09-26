import { useCallback, useEffect, useMemo, useState } from "react";
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
import { useArtifactStream } from "./lib/useArtifactStream";
import { usePersistentState, useDragResize } from "./lib/layout";
import { StatusBar } from "./components/StatusBar";
import TokenMetricsWidget from "./components/TokenMetricsWidget";
import { ExplorerTree } from "./components/ExplorerTree";
import { ThemeProvider, useTheme, ThemeSelector } from "./components/ui";
import Forge from "./components/Forge";
import CommandPalette from "./components/CommandPalette";
import EditorPane from "./components/EditorPane";
import SettingsSurface from "./components/SettingsSurface";
import { useSettings } from "./lib/settings";
import {
  MenuBar,
  type Menu,
} from "./components/shell/MenuBar";
import { isEmbeddedPreview } from "./components/shell/RightWorkspace";
import {
  ActivityRail,
  ContextSidebar,
  AgentDock,
  BottomPanel,
  SurfaceHost,
  RightWorkspace,
} from "./components/shell";
import type { BottomTab } from "./components/shell/BottomPanel";
import type { ActivityId } from "./components/shell/ActivityRail";

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

type SurfaceType =
  | "home"
  | "overview"
  | "intel"
  | "source-control"
  | "search"
  | "evidence-live"
  | "code"
  | "design"
  | "missions"
  | "runtime"
  | "artifacts"
  | "evidence"
  | "extensions"
  | "forge";

const ACTIVITY_TO_SURFACE: Record<ActivityId, SurfaceType> = {
  explorer: "code",
  search: "search",
  "source-control": "source-control",
  missions: "missions",
  run: "runtime",
  evidence: "evidence-live",
  forge: "forge",
  extensions: "extensions",
  settings: "overview",
  account: "overview",
};

type DockModule =
  | "agent"
  | "pulse"
  | "changes"
  | "files"
  | "preview"
  | "proof"
  | "mcp"
  | "providers"
  | "diagnostics";

// ---------------------------------------------------------------------------
// Main app
// ---------------------------------------------------------------------------

/**
 * Embedded preview target: when this app is iframed by its own Preview pane
 * (`/?embed=1`), render content only — no menu bar, rail, sidebars, or status
 * strip. The previous behavior rendered the full shell recursively, which is
 * how a second menu system appeared inside the right workspace.
 */
function EmbeddedPreviewApp() {
  return (
    <div className="h-screen bg-background text-text-primary overflow-auto">
      <div className="p-4 space-y-3">
        <div className="flex items-center gap-2">
          <img src="/branding/emblem.png" alt="" className="h-5 w-5" draggable={false} />
          <span className="text-sm font-semibold tracking-wide">ZylCode — live project preview</span>
        </div>
        <p className="text-xs text-text-muted max-w-md">
          This pane mirrors the running workspace front end. A production project preview
          (the built output of the project under development) is served once a project's
          preview target is commissioned in Project System — the shell around this pane
          remains fully interactive while this frame reflects the live state.
        </p>
        <div className="grid grid-cols-2 gap-2 max-w-md text-xs">
          <div className="border border-border rounded-lg p-2">
            <p className="text-[10px] text-text-muted">ENGINE</p>
            <p className="font-mono text-text-secondary">serve-intel · 17630</p>
          </div>
          <div className="border border-border rounded-lg p-2">
            <p className="text-[10px] text-text-muted">FRONTEND</p>
            <p className="font-mono text-text-secondary">vite · 1420</p>
          </div>
        </div>
      </div>
    </div>
  );
}

function AppContent() {
  const [activeActivity, setActiveActivity] = useState<ActivityId>("explorer");
  const [activeSurface, setActiveSurface] = useState<SurfaceType>("home");
  const [activeDockModule, setActiveDockModule] = useState<DockModule>("agent");
  const [bottomPanelOpen, setBottomPanelOpen] = usePersistentState("bottomOpen", true);
  const [activeBottomTab, setActiveBottomTab] = useState<BottomTab>("terminal");

  // Editor state: real workspace files open in the central pane.
  const [editorTabs, setEditorTabs] = usePersistentState<string[]>("editorTabs", []);
  const [editorActive, setEditorActive] = usePersistentState<string | null>("editorActive", null);

  // Resizable sidebars (persisted).
  const [sidebarWidth, setSidebarWidth] = usePersistentState("sidebarWidth", 260);
  const [agentDockOpen, setAgentDockOpen] = usePersistentState("agentDockOpen", true);
  const [aboutOpen, setAboutOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsSection, setSettingsSection] = useState<"general" | "agent" | "providers" | "extensions" | "appearance" | "hotkeys" | "diagnostics" | "about" | undefined>(undefined);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const { settings } = useSettings();

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
    setActiveSurface(ACTIVITY_TO_SURFACE[activity] || "code");
  }, []);

  // --- Editor tab management -------------------------------------------------

  const openFile = useCallback((path: string) => {
    setEditorTabs((prev) => (prev.includes(path) ? prev : [...prev, path]));
    setEditorActive(path);
  }, [setEditorTabs, setEditorActive]);

  const closeEditorTab = useCallback(
    (path: string) => {
      setEditorTabs((prev) => {
        const next = prev.filter((p) => p !== path);
        if (editorActive === path) {
          setEditorActive(next[next.length - 1] ?? null);
        }
        return next;
      });
    },
    [editorActive, setEditorTabs, setEditorActive],
  );

  // --- Sidebar resize ---------------------------------------------------------

  const sidebarResize = useDragResize((delta) => {
    setSidebarWidth((w) => Math.min(Math.max(200, w + delta), 480));
  });

  // --- Menus -------------------------------------------------------------------

  const menus: Menu[] = useMemo(
    () => [
      {
        id: "code",
        label: "Code",
        items: [
          { id: "cmd-palette", label: "Command Palette…", shortcut: "Ctrl+Shift+P", run: () => setPaletteOpen(true) },
          { id: "settings", label: "Settings…", shortcut: "Ctrl+,", run: () => { setSettingsSection("general"); setSettingsOpen(true); } },
          { id: "about", label: "About ZylCode", separatorBefore: true, run: () => { setSettingsSection("about"); setSettingsOpen(true); } },
        ],
      },
      {
        id: "build",
        label: "Build",
        items: [
          {
            id: "build-verify",
            label: "Verify Project (test suite)…",
            shortcut: "Ctrl+Shift+B",
            disabled: !IS_DESKTOP,
            disabledReason: IS_DESKTOP
              ? undefined
              : "Verification runs on the desktop engine; browser preview has no engine process. Use a Build mission instead (AI Agent tab).",
            run: () => { setActiveActivity("missions"); setActiveSurface("missions"); },
          },
          {
            id: "build-clean",
            label: "Rebuild Workspace",
            disabled: true,
            disabledReason: "No build orchestrator is commissioned yet; the Rust workspace builds via `cargo build` in the Terminal.",
          },
        ],
      },
      {
        id: "run",
        label: "Run",
        items: [
          { id: "run-open", label: "Open Run Surface", shortcut: "Ctrl+R", run: () => { setActiveActivity("run"); setActiveSurface("runtime"); } },
          { id: "run-terminal", label: "Open Terminal", shortcut: "Ctrl+`", run: () => { setBottomPanelOpen(true); setActiveBottomTab("terminal"); } },
          { id: "run-tests", label: "Show Test Results", run: () => { setBottomPanelOpen(true); setActiveBottomTab("tests"); } },
        ],
      },
      {
        id: "deploy",
        label: "Deploy",
        items: [
          {
            id: "deploy-app",
            label: "Deploy…",
            disabled: true,
            disabledReason: "No deployment target is commissioned (no remote/CI pipeline configured in this build).",
          },
        ],
      },
      {
        id: "ai",
        label: "AI",
        items: [
          { id: "ai-agent", label: "New Mission (Build)…", shortcut: "Ctrl+M", run: () => { setActiveActivity("missions"); setActiveSurface("missions"); } },
          { id: "ai-queue", label: "Mission Queue", run: () => { setActiveActivity("missions"); setActiveSurface("missions"); } },
          { id: "ai-evidence", label: "Evidence Ledger", run: () => { setActiveActivity("evidence"); setActiveSurface("evidence-live"); } },
          { id: "ai-forge", label: "Capability Forge", run: () => { setActiveActivity("forge"); setActiveSurface("forge"); } },
        ],
      },
      {
        id: "view",
        label: "View",
        items: [
          { id: "view-explorer", label: "Explorer", shortcut: "Ctrl+Shift+E", run: () => handleActivityChange("explorer") },
          { id: "view-search", label: "Search", shortcut: "Ctrl+Shift+F", run: () => handleActivityChange("search") },
          { id: "view-scm", label: "Source Control", shortcut: "Ctrl+Shift+G", run: () => handleActivityChange("source-control") },
          { id: "view-terminal", label: "Toggle Terminal Panel", shortcut: "Ctrl+`", run: () => setBottomPanelOpen((v) => !v) },
          { id: "view-problems", label: "Problems", run: () => { setBottomPanelOpen(true); setActiveBottomTab("problems"); } },
          { id: "view-output", label: "Output", run: () => { setBottomPanelOpen(true); setActiveBottomTab("output"); } },
          { id: "view-ports", label: "Ports", run: () => { setBottomPanelOpen(true); setActiveBottomTab("ports"); } },
        ],
      },
      {
        id: "window",
        label: "Window",
        items: [
          { id: "win-editor", label: "Editor Focus", run: () => { setActiveActivity("explorer"); setActiveSurface("code"); } },
          { id: "win-preview", label: "Show Preview", run: () => { setAgentDockOpen(true); } },
          {
            id: "win-theme",
            label: `Theme: ${currentTheme}`,
            run: () => {
              // Cycle to the next theme for quick visual verification.
              const order = ["midnight-pro", "arctic-light", "github-dark", "vscode-classic", "solarized-dark", "dracula", "nord", "monokai-pro"];
              const idx = order.indexOf(currentTheme);
              setTheme(order[(idx + 1) % order.length] as typeof currentTheme);
            },
          },
        ],
      },
      {
        id: "help",
        label: "Help",
        items: [
          { id: "help-about", label: "About", run: () => setAboutOpen(true) },
          {
            id: "help-cli",
            label: "CLI Reference",
            disabled: true,
            disabledReason: "Documentation site not commissioned; use `zylcode --help` in the Terminal.",
          },
        ],
      },
    ],
    [currentTheme, handleActivityChange, setTheme, setBottomPanelOpen, setAgentDockOpen],
  );

  // Global shortcuts.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (mod && e.shiftKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        setPaletteOpen((v) => !v);
      } else if (mod && e.key === "`") {
        e.preventDefault();
        setBottomPanelOpen((v) => !v);
      } else if (mod && e.key === ",") {
        e.preventDefault();
        setSettingsSection("general");
        setSettingsOpen(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setBottomPanelOpen]);

  return (
    <div className="h-screen bg-background text-text-primary flex flex-col overflow-hidden">
      {/* TOP: menu bar (brand inside the bar) */}
      <MenuBar
        menus={menus}
        brand={
          <button
            type="button"
            onClick={() => setActiveSurface("home")}
            className="flex items-center gap-2 mr-2 rounded focus:outline-none focus-visible:ring-2 focus-visible:ring-primary"
            aria-label="Go to Mission Home"
            title="Mission Home"
          >
            <div
              className="flex items-center gap-1.5 shrink-0 rounded-md overflow-hidden"
              style={{ background: "rgb(20 20 20 / 0.92)" }}
            >
              <img
                src="/branding/emblem.png"
                alt="ZylCode emblem"
                className="h-6 w-6"
                draggable={false}
              />
              <img
                src="/branding/wordmark.png"
                alt="ZylCode"
                className="h-3.5 hidden md:block"
                draggable={false}
              />
            </div>
            <span className="hidden lg:inline text-[10px] font-mono text-text-muted">v0.3.0-convergence</span>
          </button>
        }
        right={
          <div className="flex items-center gap-2">
            {activeMission && (
              <span className="hidden sm:flex items-center gap-1.5 text-xs font-mono text-text-muted">
                {activeMission.state}
              </span>
            )}
            <TokenMetricsWidget refreshKey={metricsKey} />
            <ThemeSelector />
          </div>
        }
      />

      {/* MIDDLE: rail | sidebar | center | agent dock / right workspace */}
      <div className="flex flex-1 min-h-0">
        <ActivityRail
          activeActivity={activeActivity}
          onActivityChange={handleActivityChange}
          onBottomUtilityClick={(id) => {
            if (id === "settings") {
              setActiveActivity("settings");
              setActiveSurface("overview");
            }
          }}
        />

        {/* Resizable context sidebar */}
        <div
          className="relative shrink-0 border-r border-border bg-surface/50 flex min-h-0"
          style={{ width: sidebarWidth }}
        >
          <div className="flex-1 min-w-0 overflow-y-auto">
            <ContextSidebar activeActivity={activeActivity}>
              {activeActivity === "explorer" ? (
                <ExplorerTree onOpenFile={openFile} activePath={editorActive} />
              ) : undefined}
            </ContextSidebar>
          </div>
          <div
            className="w-1 hover:bg-primary/30 transition-colors"
            style={{ cursor: "col-resize" }}
            onPointerDown={sidebarResize.onPointerDown}
            onPointerMove={sidebarResize.onPointerMove}
            onPointerUp={sidebarResize.onPointerUp}
            role="separator"
            aria-orientation="vertical"
            aria-label="Resize sidebar"
          />
        </div>

        {/* CENTER: tabbed editor or surface host */}
        <main className="flex-1 flex flex-col min-h-0">
          {activeSurface === "code" ? (
            <EditorPane
              openFiles={editorTabs}
              activePath={editorActive}
              onSelectTab={setEditorActive}
              onCloseTab={closeEditorTab}
              serviceNote="Open a file from the Explorer — content comes from the real workspace through the engine API."
            />
          ) : (
            <div className="flex-1 min-h-0 overflow-y-auto">
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
                  setActiveActivity("explorer");
                  setActiveSurface("code");
                }}
                onOpenMissions={() => {
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
            </div>
          )}

          {/* Bottom workspace */}
          <BottomPanel
            isOpen={bottomPanelOpen}
            onToggle={() => setBottomPanelOpen((v) => !v)}
            activeTab={activeBottomTab}
            onTabChange={setActiveBottomTab}
          />
        </main>

        {/* RIGHT: preview + agent workspace (large screens) */}
        <RightWorkspace />

        {/* Legacy agent dock hidden on xl screens where RightWorkspace shows; kept for smaller */}
        <div className="hidden lg:flex xl:hidden flex-col">
          <AgentDock
            activeModule={activeDockModule}
            onModuleChange={setActiveDockModule}
            activeMission={activeMission}
            deltas={deltas}
            isStreaming={isStreaming}
          />
        </div>
      </div>

      {/* Status strip */}
      <StatusBar theme={currentTheme} onThemeChange={setTheme} mcpBridgeCount={0} />

      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} menus={menus} />
      <SettingsSurface
        open={settingsOpen || aboutOpen}
        initialSection={aboutOpen ? "about" : settingsSection}
        onClose={() => {
          setSettingsOpen(false);
          setAboutOpen(false);
        }}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// App root
// ---------------------------------------------------------------------------

export default function App() {
  // Embedded context (`/?embed=1`, e.g. the Preview pane iframing this app):
  // content-only render, no second shell.
  if (isEmbeddedPreview()) {
    return <EmbeddedPreviewApp />;
  }
  return (
    <ThemeProvider>
      <AppContent />
    </ThemeProvider>
  );
}
