import { Panel } from "../Panel";
import { ProofInspector, McpInspector, ProviderSettings, DiagnosticsPanel } from "../../components";
import { useTheme } from "../ui";
import { IS_DESKTOP, DESKTOP_REQUIRED_MESSAGE } from "../../lib/runtime";

type DockModule = "agent" | "preview" | "proof" | "mcp" | "providers" | "diagnostics";

interface AgentDockProps {
  activeModule: DockModule;
  onModuleChange: (module: DockModule) => void;
  activeMission: any;
  deltas: any[];
  isStreaming: boolean;
}

const DOCK_MODULES: { id: DockModule; label: string }[] = [
  { id: "agent", label: "Agent" },
  { id: "preview", label: "Preview" },
  { id: "proof", label: "Proof" },
  { id: "mcp", label: "MCP" },
  { id: "providers", label: "Providers" },
  { id: "diagnostics", label: "Diagnostics" },
];

export function AgentDock({
  activeModule,
  onModuleChange,
  activeMission,
  deltas,
  isStreaming,
}: AgentDockProps) {
  const { currentTheme } = useTheme();

  return (
    <aside className="hidden lg:flex w-[360px] shrink-0 border-l border-border bg-surface/50 flex-col">
      <div className="flex items-center gap-1 px-2 py-2 border-b border-border overflow-x-auto">
        {DOCK_MODULES.map((m) => (
          <button
            key={m.id}
            onClick={() => onModuleChange(m.id)}
            className={`px-3 py-1.5 rounded text-sm whitespace-nowrap transition-colors ${
              activeModule === m.id
                ? "bg-primary/10 text-primary font-medium"
                : "text-text-muted hover:text-text-primary hover:bg-surface"
            }`}
          >
            {m.label}
          </button>
        ))}
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto p-3 space-y-3">
        {activeModule === "agent" && (
          <>
            {activeMission ? (
              <ProofInspector mission={activeMission} />
            ) : (
              <Panel title="AGENT">
                <p className="text-xs text-text-muted">
                  No active mission. Start a mission to see the agent stream.
                </p>
              </Panel>
            )}

            <Panel title="STREAM">
              {deltas.length === 0 ? (
                <p className="text-xs text-text-muted">
                  Idle. Run a mission to see the agent stream.
                </p>
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

        {activeModule === "proof" && <ProofInspector mission={activeMission} />}

        {activeModule === "preview" && (
          <Panel title="PREVIEW">
            <p className="text-xs text-text-muted">
              Artifact preview opens with mission artifacts; the Run tab handles run targets.
            </p>
          </Panel>
        )}

        {activeModule === "mcp" && (
          <Panel title="MCP ACTIVITY">
            {IS_DESKTOP ? <McpInspector /> : <DesktopRequired />}
          </Panel>
        )}

        {activeModule === "providers" && (
          <Panel title="PROVIDERS — FALLBACK CHAIN">
            {IS_DESKTOP ? <ProviderSettings /> : <DesktopRequired />}
          </Panel>
        )}

        {activeModule === "diagnostics" && <DiagnosticsPanel />}
      </div>
    </aside>
  );
}

function DesktopRequired() {
  return (
    <div className="flex flex-col items-center justify-center gap-2 py-8 text-center">
      <p className="text-sm text-text-muted max-w-xs">{DESKTOP_REQUIRED_MESSAGE}</p>
      <p className="text-[11px] text-text-muted/70">
        Technical details are available in Diagnostics.
      </p>
    </div>
  );
}