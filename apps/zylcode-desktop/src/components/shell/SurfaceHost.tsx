import { ArtifactViewer } from "../ArtifactViewer";
import { EvidenceCenter } from "../EvidenceCenter";
import { MissionComposer } from "../MissionComposer";
import { RepoIntelPanel } from "../RepoIntelPanel";
import { RuntimeLab } from "../RuntimeLab";
import Forge from "../Forge";
import { VerificationRungBadge } from "../VerificationRungBadge";
import { Panel } from "../Panel";
import { StatusBadge } from "../CapabilityStatus";
import { NOT_CAPTURED, type MissionRecord } from "../../lib/mission";
import { IS_DESKTOP } from "../../lib/runtime";

type SurfaceType =
  | "home"
  | "overview"
  | "intel"
  | "code"
  | "design"
  | "missions"
  | "runtime"
  | "artifacts"
  | "evidence"
  | "extensions"
  | "forge";

interface SurfaceHostProps {
  surface: SurfaceType;
  activeMission: MissionRecord | null;
  missions: MissionRecord[];
  verification: any;
  busy: boolean;
  isStreaming: boolean;
  onMissionStart: (goal: string, autonomy: string, model: string | null) => void;
  onVerifyMission: () => void;
  onOpenProject: () => void;
  activeTab: string | null;
  setActiveTab: React.Dispatch<React.SetStateAction<string | null>>;
  files: any;
  diffMode: boolean;
  setDiffMode: React.Dispatch<React.SetStateAction<boolean>>;
  saveStatus: any;
  saveFile: any;
  applyPatch: any;
  closeTab: any;
  deltas: any[];
}

export function SurfaceHost({
  surface,
  activeMission,
  missions,
  verification,
  busy,
  isStreaming,
  onMissionStart,
  onVerifyMission,
  onOpenProject,
  activeTab,
  setActiveTab,
  files,
  diffMode,
  setDiffMode,
  saveStatus,
  saveFile,
  applyPatch,
  closeTab,
  deltas,
}: SurfaceHostProps) {
  const renderSurface = () => {
    switch (surface) {
      case "home":
        return (
          <div className="p-4">
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
                {missions.slice(-3).reverse().length === 0 ? (
                  <p className="text-sm text-text-muted">No missions yet.</p>
                ) : (
                  <div className="space-y-1.5">
                    {missions
                      .slice(-3)
                      .reverse()
                      .map((m) => (
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
          </div>
        );

      case "forge":
        return <div className="p-4"><Forge /></div>;

      case "intel":
        return (
          <div className="grid grid-cols-1 xl:grid-cols-2 gap-3 p-1">
            <RepoIntelPanel initialTask="agent loop and evidence ledger" />
            <div className="space-y-3">
              <Panel title="WHAT AM I LOOKING AT">
                <div className="space-y-1.5 text-xs text-text-muted">
                  <p>
                    This panel queries the <span className="text-text">real</span> Repository
                    Intelligence pipeline: repository scan, symbol index, package manifests,
                    dependency graph, git history — served through the persisted,
                    content-hash-validated index (no per-keystroke re-indexing).
                  </p>
                  <p>
                    Browser preview reaches it via <span className="font-mono">zylcode serve-intel</span>
                    {" "}(HTTP); the desktop app calls the engine in-process. Ranking and provenance
                    are the retriever's own output — nothing here is mocked.
                  </p>
                </div>
              </Panel>
              <Panel title="CAPABILITIES">
                <div className="flex flex-wrap gap-x-4 gap-y-2 text-xs">
                  <span className="flex items-center gap-1.5">Missions <StatusBadge status="LIMITED" size="xs" /></span>
                  <span className="flex items-center gap-1.5">Repository Intelligence <StatusBadge status="AVAILABLE" size="xs" /></span>
                  <span className="flex items-center gap-1.5">Forge <StatusBadge status="LIMITED" size="xs" /></span>
                  <span className="flex items-center gap-1.5">Design <StatusBadge status="COMING SOON" size="xs" /></span>
                </div>
              </Panel>
            </div>
          </div>
        );

      case "overview":
        return (
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
                  <span className="font-mono">BROWSER_PREVIEW</span> (Browser preview)
                </p>
                <p className="text-text-muted">
                  Browser preview: the desktop engine is not connected. Backend panels show controlled states instead of errors.
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
        );

      case "missions":
        return (
          <div className="grid grid-cols-1 xl:grid-cols-3 gap-3">
            <Panel title="MISSION COMPOSER" className="xl:col-span-2">
              <MissionComposer onMissionStart={onMissionStart} busy={busy} streaming={isStreaming} />
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
                          onClick={() => {}}
                          className={`w-full text-left rounded border px-2 py-1.5 text-xs transition-colors ${
                            m.id === "active" ? "border-primary/50 bg-primary/5" : "border-border hover:border-primary/30"
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
                  onClick={onVerifyMission}
                  disabled={busy}
                  className="w-full bg-secondary text-secondary-foreground px-3 py-2 rounded-md text-sm font-medium hover:bg-secondary-hover disabled:opacity-50"
                >
                  Verify active mission
                </button>
                {verification && (
                  <div className="mt-2 space-y-1">
                    <VerificationRungBadge rung={verification.rung} />
                    {verification.checks.map((c: any) => (
                      <p key={c.name} className="text-[11px]">
                        {c.passed ? "✓" : "✗"} {c.name}: {c.message}
                      </p>
                    ))}
                  </div>
                )}
              </Panel>
            </div>
          </div>
        );

      case "runtime":
        return <RuntimeLab />;

      case "evidence":
        return <EvidenceCenter missions={missions} />;

      case "code":
        return (
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
        );

      case "artifacts":
        return (
          <Panel title="ARTIFACTS">
            {missions.flatMap((m) => m.artifacts).length === 0 ? (
              <p className="text-sm text-text-muted">
                No artifacts yet — mission artifacts will appear here.{" "}
                <span className="font-mono text-[10px]">{NOT_CAPTURED}</span>
              </p>
            ) : (
              <div className="space-y-1.5 text-xs">
                {missions.flatMap((m) => m.artifacts).map((a, i) => (
                  <div key={i} className="flex items-center justify-between rounded border border-border px-2 py-1.5">
                    <span>{a.label}</span>
                    <span className="font-mono text-[10px] text-text-muted">{a.kind}</span>
                  </div>
                ))}
              </div>
            )}
          </Panel>
        );

      case "design":
      case "extensions":
        return (
          <Panel title={surface === "design" ? "DESIGN STUDIO" : "EXTENSIONS"}>
            <div className="flex flex-col items-center gap-2 py-10 text-center">
              <StatusBadge status="COMING SOON" />
              <p className="text-sm text-text-muted max-w-sm">
                {surface === "design"
                  ? "The Design workspace (canvas, tokens, screens) is defined in Product Architecture v3 and awaits implementation."
                  : "Extensions integrate with Forge capability packs. The Forge shell is available under Forge."}
              </p>
            </div>
          </Panel>
        );

      default:
        return (
          <Panel title="SURFACE">
            <p className="text-sm text-text-muted">Surface not implemented: {surface}</p>
          </Panel>
        );
    }
  };

  return <div className="flex-1 min-h-0 p-3">{renderSurface()}</div>;
}