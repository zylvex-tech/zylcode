import React, { useMemo, useState } from "react";
import { StatusBadge, type CapabilityStatus } from "../CapabilityStatus";
import AgentWorkspace from "../AgentWorkspace";
import { isEmbeddedPreview } from "../../lib/runtime";

type RightTab = "preview" | "design" | "responsive";

const WIDTHS: { id: string; label: string; px: number | null }[] = [
  { id: "full", label: "100%", px: null },
  { id: "desktop", label: "1280", px: 1280 },
  { id: "tablet", label: "768", px: 768 },
  { id: "mobile", label: "375", px: 375 },
];

/**
 * Right workspace: Preview (the actual running project in an iframe),
 * Design (points at the Design surface, which is implemented), Responsive
 * (real width-constrained preview presets). Below it, the agent workspace
 * (AI Agent / Tasks / Changes / Evidence) backed by the real queue+ledger.
 */
export const RightWorkspace: React.FC = () => {
  const [tab, setTab] = useState<RightTab>("preview");
  const [previewOpen, setPreviewOpen] = useState(true);
  const [agentTab, setAgentTab] = useState<"agent" | "tasks" | "changes" | "evidence">("agent");
  const [width, setWidth] = useState<string>("full");

  const embedded = useMemo(() => isEmbeddedPreview(), []);
  const previewSrc = useMemo(() => {
    const url = new URL(window.location.origin);
    url.searchParams.set("embed", "1");
    return url.toString();
  }, []);

  const previewStatus: CapabilityStatus = embedded ? "LIMITED" : "AVAILABLE";
  const previewLabel =
    "Workspace mirror — the project's own preview target is not commissioned yet";

  return (
    <aside className="hidden xl:flex w-[380px] 2xl:w-[440px] shrink-0 border-l border-border bg-surface/40 flex-col min-h-0">
      {/* Preview / Design / Responsive tabs */}
      <div className="flex items-center border-b border-border shrink-0">
        {(
          [
            { id: "preview", label: "Preview" },
            { id: "design", label: "Design" },
            { id: "responsive", label: "Responsive" },
          ] as { id: RightTab; label: string }[]
        ).map((t) => (
          <button
            key={t.id}
            onClick={() => {
              setTab(t.id);
              if (t.id !== "design") setPreviewOpen(true);
            }}
            className={`px-3 py-1.5 text-xs border-b-2 -mb-px transition-colors ${
              tab === t.id
                ? "border-primary text-text-primary"
                : "border-transparent text-text-muted hover:text-text-primary"
            }`}
          >
            {t.label}
          </button>
        ))}
        <div className="ml-auto flex items-center gap-1 pr-2">
          {tab === "responsive" && (
            <div className="flex items-center gap-0.5">
              {WIDTHS.map((w) => (
                <button
                  key={w.id}
                  onClick={() => setWidth(w.id)}
                  className={`px-1.5 py-0.5 rounded text-[10px] font-mono ${
                    width === w.id
                      ? "bg-primary/15 text-primary"
                      : "text-text-muted hover:text-text-primary"
                  }`}
                >
                  {w.label}
                </button>
              ))}
            </div>
          )}
          <button
            onClick={() => setPreviewOpen((v) => !v)}
            className="px-1.5 py-0.5 text-[10px] text-text-muted hover:text-text-primary"
            aria-label={previewOpen ? "Collapse preview" : "Expand preview"}
          >
            {previewOpen ? "▾" : "▸"}
          </button>
        </div>
      </div>

      {tab === "design" ? (
        <div className="flex-1 min-h-0 p-3 overflow-y-auto">
          <StatusBadge status="LIMITED" size="xs" />
          <p className="text-xs text-text-muted mt-2">
            Design Studio is a working workspace on the Design surface — create projects, edit text
            and shapes on the canvas, save, reopen and export standalone HTML. It is not duplicated
            in this pane.
          </p>
          <p className="text-[11px] text-text-muted/70 mt-2">
            Not implemented: visual diff, design↔code generation, agent repair, Figma import.
          </p>
        </div>
      ) : (
        previewOpen && (
          <div className="h-[42%] min-h-[160px] shrink-0 border-b border-border flex flex-col bg-background">
            {embedded ? (
              <div className="flex-1 flex items-center justify-center p-3">
                <p className="text-xs text-text-muted text-center">
                  Already inside the embedded preview context.
                </p>
              </div>
            ) : (
              <div className="flex-1 min-h-0 flex justify-center overflow-hidden">
                <iframe
                  key={width}
                  src={previewSrc}
                  title="Project preview"
                  className="h-full border-0 bg-white"
                  style={
                    WIDTHS.find((w) => w.id === width)?.px
                      ? { width: WIDTHS.find((w) => w.id === width)!.px! }
                      : { width: "100%" }
                  }
                  sandbox="allow-scripts allow-forms allow-same-origin"
                />
              </div>
            )}
            <div className="shrink-0 px-2 py-1 border-t border-border flex items-center gap-2">
              <StatusBadge status={previewStatus} size="xs" />
              <span className="text-[10px] text-text-muted truncate" title={previewLabel}>
                {tab === "responsive"
                  ? `Responsive · ${WIDTHS.find((w) => w.id === width)?.label}`
                  : "Workspace mirror"}
              </span>
            </div>
          </div>
        )
      )}

      {/* Agent workspace */}
      <div className="flex-1 min-h-0 flex flex-col">
        <AgentWorkspace activeTab={agentTab} onTabChange={setAgentTab} narrow />
      </div>
    </aside>
  );
};

export default RightWorkspace;
