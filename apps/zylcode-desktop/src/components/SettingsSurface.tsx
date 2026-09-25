import React, { useEffect, useMemo, useState } from "react";
import { useSettings, DEFAULT_SETTINGS, type ZylSettings } from "../lib/settings";
import { AUTONOMY_MODES } from "../lib/mission";
import { fetchVersion, type VersionState } from "../lib/version";
import { getDiagnostics, clearDiagnostics, IS_DESKTOP } from "../lib/runtime";
import { StatusBadge } from "./CapabilityStatus";
import ProviderSettings from "./ProviderSettings";
import McpInspector from "./McpInspector";
import { ThemeSelector } from "./ui";

type SectionId =
  | "general"
  | "agent"
  | "providers"
  | "extensions"
  | "appearance"
  | "hotkeys"
  | "diagnostics"
  | "about";

const SECTIONS: { id: SectionId; label: string; icon: string }[] = [
  { id: "general", label: "General", icon: "⚙" },
  { id: "agent", label: "Agent & Sessions", icon: "🤖" },
  { id: "providers", label: "Model Providers", icon: "⚡" },
  { id: "extensions", label: "Extensions & MCP", icon: "🧩" },
  { id: "appearance", label: "Appearance", icon: "🎨" },
  { id: "hotkeys", label: "Hotkeys", icon: "⌨" },
  { id: "diagnostics", label: "Diagnostics", icon: "📈" },
  { id: "about", label: "About", icon: "ℹ" },
];

interface SettingsSurfaceProps {
  open: boolean;
  initialSection?: SectionId;
  onClose: () => void;
}

function Row(props: {
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-6 px-4 py-3 border-b border-border last:border-0">
      <div className="min-w-0">
        <p className="text-sm text-text-primary">{props.title}</p>
        <p className="text-xs text-text-muted mt-0.5">{props.description}</p>
      </div>
      <div className="shrink-0">{props.children}</div>
    </div>
  );
}

function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={`w-10 h-6 rounded-full transition-colors relative ${
        checked ? "bg-primary" : "bg-surface-active"
      }`}
    >
      <span
        className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow transition-all ${
          checked ? "left-[1.125rem]" : "left-0.5"
        }`}
      />
    </button>
  );
}

function Segmented<T extends string | number>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
  label: string;
}) {
  return (
    <div className="inline-flex rounded-md border border-border overflow-hidden" role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button
          key={String(o.value)}
          role="radio"
          aria-checked={value === o.value}
          onClick={() => onChange(o.value)}
          className={`px-3 py-1 text-xs transition-colors ${
            value === o.value
              ? "bg-primary/15 text-primary font-medium"
              : "text-text-muted hover:text-text-primary hover:bg-surface"
          }`}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

/**
 * The unified settings surface. Every control maps to a real persisted
 * setting, a real backend view, or an honest statement that a capability
 * is not commissioned. Nothing here is decorative.
 */
export const SettingsSurface: React.FC<SettingsSurfaceProps> = ({
  open,
  initialSection,
  onClose,
}) => {
  const [section, setSection] = useState<SectionId>(initialSection ?? "general");
  const { settings, update, reset } = useSettings();
  const [version, setVersion] = useState<VersionState>({ kind: "loading" });
  const [diagCount, setDiagCount] = useState(0);

  useEffect(() => {
    if (open) setSection(initialSection ?? "general");
  }, [open, initialSection]);

  useEffect(() => {
    if (!open) return;
    fetchVersion().then(setVersion);
    const pull = () => setDiagCount(getDiagnostics().length);
    pull();
    const t = setInterval(pull, 2000);
    return () => clearInterval(t);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  const autonomyOptions = useMemo(
    () =>
      AUTONOMY_MODES.map((m) => ({
        value: m.mode,
        label: m.label,
        status: m.status,
      })),
    [],
  );

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[90] bg-black/60 backdrop-blur-sm flex items-center justify-center p-4"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="w-full max-w-4xl h-[min(680px,90vh)] bg-surface border border-border rounded-xl shadow-2xl flex overflow-hidden"
        role="dialog"
        aria-label="Settings"
      >
        {/* Section rail */}
        <div className="w-56 shrink-0 border-r border-border bg-surface/60 py-3 overflow-y-auto">
          <p className="px-4 pb-2 text-base font-semibold text-text-primary">Settings</p>
          {SECTIONS.map((s) => (
            <button
              key={s.id}
              onClick={() => setSection(s.id)}
              className={`w-full text-left px-4 py-2 text-sm flex items-center gap-2.5 transition-colors ${
                section === s.id
                  ? "bg-primary/10 text-primary font-medium"
                  : "text-text-secondary hover:text-text-primary hover:bg-surface"
              }`}
            >
              <span className="w-4 text-center">{s.icon}</span>
              {s.label}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0 flex flex-col">
          <div className="flex items-center justify-between px-5 py-3 border-b border-border shrink-0">
            <p className="text-sm font-medium text-text-primary">
              {SECTIONS.find((s) => s.id === section)?.label}
            </p>
            <button
              onClick={onClose}
              aria-label="Close settings"
              className="w-7 h-7 rounded hover:bg-surface-hover flex items-center justify-center text-text-muted hover:text-text-primary"
            >
              ✕
            </button>
          </div>

          <div className="flex-1 min-h-0 overflow-y-auto">
            {section === "general" && (
              <div className="mx-4 my-4 rounded-lg border border-border bg-background/40 divide-y divide-border">
                <Row
                  title="Default mission mode"
                  description="The agent composer's preselected mode for new missions."
                >
                  <Segmented
                    label="Default mission mode"
                    value={settings.defaultMissionMode}
                    options={[
                      { value: "build" as const, label: "⚡ Build" },
                      { value: "plan" as const, label: "🗺 Plan" },
                    ]}
                    onChange={(v) => update("defaultMissionMode", v)}
                  />
                </Row>
                <Row title="Editor tab width" description="Indent width used by the editor gutter.">
                  <Segmented
                    label="Editor tab width"
                    value={settings.editorTabSize}
                    options={[
                      { value: 2 as const, label: "2" },
                      { value: 4 as const, label: "4" },
                    ]}
                    onChange={(v) => update("editorTabSize", v)}
                  />
                </Row>
                <Row
                  title="Capture diagnostics"
                  description="Record frontend diagnostics (backend failures, IPC issues) for the Diagnostics section."
                >
                  <Toggle
                    label="Capture diagnostics"
                    checked={settings.captureDiagnostics}
                    onChange={(v) => update("captureDiagnostics", v)}
                  />
                </Row>
                <div className="px-4 py-3">
                  <button
                    onClick={() => reset()}
                    className="text-xs text-red-400 hover:text-red-300"
                  >
                    Reset all settings to defaults
                  </button>
                </div>
              </div>
            )}

            {section === "agent" && (
              <div className="mx-4 my-4 space-y-4">
                <div className="rounded-lg border border-border bg-background/40 divide-y divide-border">
                  <Row
                    title="Default autonomy"
                    description="Autonomy preselected for new missions. Modes never override the permission engine."
                  >
                    <select
                      value={settings.defaultAutonomy}
                      onChange={(e) =>
                        update("defaultAutonomy", e.target.value as ZylSettings["defaultAutonomy"])
                      }
                      className="bg-surface border border-border rounded px-2 py-1.5 text-xs text-text-primary focus:outline-none focus:border-primary/50"
                    >
                      {autonomyOptions.map((o) => (
                        <option key={o.value} value={o.value}>
                          {o.label}
                          {o.status !== "AVAILABLE" ? ` (${o.status.toLowerCase()})` : ""}
                        </option>
                      ))}
                    </select>
                  </Row>
                  <Row
                    title="Tool call summary"
                    description="Consecutive tool calls are summarized into one row during a mission."
                  >
                    <Toggle
                      label="Tool call summary"
                      checked={settings.toolCallSummary}
                      onChange={(v) => update("toolCallSummary", v)}
                    />
                  </Row>
                  <Row
                    title="Auto-fold finished work"
                    description="When a mission turn ends, completed tool sections fold away, leaving the summary."
                  >
                    <Toggle
                      label="Auto-fold finished work"
                      checked={settings.autoFoldMessages}
                      onChange={(v) => update("autoFoldMessages", v)}
                    />
                  </Row>
                </div>
                <p className="text-[11px] text-text-muted px-1">
                  Queued missions never run without an explicit human action — the queue drain in
                  the Tasks panel is the approval mechanism.
                </p>
              </div>
            )}

            {section === "providers" && (
              <div className="p-4">
                {IS_DESKTOP ? (
                  <ProviderSettings />
                ) : (
                  <div className="space-y-2">
                    <StatusBadge status="LIMITED" size="xs" />
                    <p className="text-xs text-text-muted">
                      Provider chain configuration runs on the desktop engine (Tauri). The browser
                      preview reads the same defaults but cannot modify the engine's provider chain.
                    </p>
                  </div>
                )}
              </div>
            )}

            {section === "extensions" && (
              <div className="p-4 space-y-3">
                {IS_DESKTOP ? (
                  <McpInspector />
                ) : (
                  <div className="space-y-2">
                    <StatusBadge status="LIMITED" size="xs" />
                    <p className="text-xs text-text-muted">
                      MCP bridge activity is visible on the desktop engine. The browser preview has
                      no engine process to inspect.
                    </p>
                  </div>
                )}
                <p className="text-[11px] text-text-muted">
                  Capability packs live in the Forge surface (taxonomy and trust labels; the
                  marketplace install pipeline is not commissioned yet).
                </p>
              </div>
            )}

            {section === "appearance" && (
              <div className="mx-4 my-4 space-y-4">
                <div className="rounded-lg border border-border bg-background/40 divide-y divide-border">
                  <Row title="Theme" description="All eight themes, persisted.">
                    <ThemeSelector />
                  </Row>
                  <Row
                    title="Reduced motion"
                    description="Minimize non-essential animation across the shell."
                  >
                    <Toggle
                      label="Reduced motion"
                      checked={settings.reducedMotion}
                      onChange={(v) => update("reducedMotion", v)}
                    />
                  </Row>
                  <Row
                    title="Compact density"
                    description="Tighter spacing in panels and lists."
                  >
                    <Toggle
                      label="Compact density"
                      checked={settings.compactDensity}
                      onChange={(v) => update("compactDensity", v)}
                    />
                  </Row>
                  <Row
                    title="Editor line numbers"
                    description="Show the numbered gutter in the editor."
                  >
                    <Toggle
                      label="Editor line numbers"
                      checked={settings.editorLineNumbers}
                      onChange={(v) => update("editorLineNumbers", v)}
                    />
                  </Row>
                  <Row
                    title="Editor word wrap"
                    description="Wrap long lines instead of horizontal scrolling."
                  >
                    <Toggle
                      label="Editor word wrap"
                      checked={settings.editorWordWrap}
                      onChange={(v) => update("editorWordWrap", v)}
                    />
                  </Row>
                </div>
              </div>
            )}

            {section === "hotkeys" && (
              <div className="mx-4 my-4 rounded-lg border border-border bg-background/40 divide-y divide-border">
                {[
                  ["Ctrl+Shift+P", "Command palette"],
                  ["Ctrl+`", "Toggle bottom panel"],
                  ["Ctrl+Shift+E", "Explorer"],
                  ["Ctrl+Shift+F", "Search"],
                  ["Ctrl+Shift+G", "Source Control"],
                  ["Ctrl+M", "Missions"],
                  ["Esc", "Close menus, palette, and dialogs"],
                ].map(([key, desc]) => (
                  <Row key={key} title={key} description={desc}>
                    <span className="text-[10px] text-text-muted font-mono">built-in</span>
                  </Row>
                ))}
                <div className="px-4 py-3">
                  <p className="text-[11px] text-text-muted">
                    Custom hotkey rebinding is not commissioned yet — these are the real current
                    bindings.
                  </p>
                </div>
              </div>
            )}

            {section === "diagnostics" && (
              <div className="mx-4 my-4 space-y-4">
                <div className="rounded-lg border border-border bg-background/40 divide-y divide-border">
                  <Row
                    title="Version"
                    description={
                      version.kind === "ready"
                        ? `Engine ${version.data.engine} · ${version.data.branch}@${version.data.head_commit}${version.data.dirty ? " (dirty tree)" : ""}`
                        : version.kind === "loading"
                          ? "reading…"
                          : version.reason
                    }
                  >
                    <span className="text-xs font-mono text-text-secondary">
                      {version.kind === "ready" ? version.data.app_version : "—"}
                    </span>
                  </Row>
                  <Row
                    title="Diagnostic entries"
                    description="Runtime issues captured by the frontend (IPC failures, backend errors)."
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-mono text-text-secondary">{diagCount}</span>
                      <button
                        onClick={() => clearDiagnostics()}
                        className="px-2.5 py-1 rounded text-[11px] border border-border text-text-secondary hover:text-text-primary hover:bg-surface"
                      >
                        Clear
                      </button>
                    </div>
                  </Row>
                </div>
                <p className="text-[11px] text-text-muted px-1">
                  The Diagnostics panel (Dev Tools tab) shows the full entries with timestamps.
                </p>
              </div>
            )}

            {section === "about" && (
              <div className="p-6 flex flex-col items-center text-center gap-3">
                <div
                  className="flex items-center gap-2 rounded-md overflow-hidden px-3 py-2"
                  style={{ background: "rgb(20 20 20 / 0.92)" }}
                >
                  <img src="/branding/lockup.png" alt="ZylCode" className="h-10" draggable={false} />
                </div>
                <p className="text-xs text-text-muted">
                  Formally Verified AI Software Synthesis Engine
                </p>
                <div className="w-full max-w-sm rounded-lg border border-border bg-background/40 divide-y divide-border text-left">
                  <Row
                    title="App version"
                    description="The running frontend's version."
                  >
                    <span className="text-xs font-mono">0.2.0</span>
                  </Row>
                  <Row
                    title="Engine"
                    description={
                      version.kind === "ready"
                        ? `${version.data.branch}@${version.data.head_commit}${version.data.dirty ? " · dirty" : ""}`
                        : "engine version unavailable in this context"
                    }
                  >
                    <span className="text-xs font-mono">
                      {version.kind === "ready" ? version.data.app_version : "—"}
                    </span>
                  </Row>
                </div>
                <p className="text-[11px] text-text-muted max-w-md">
                  Every status shown across this app is computed from real state: the mission queue,
                  the evidence ledger's hash chain, and the live intelligence pipeline. When a
                  capability is not commissioned, the UI says so instead of simulating it.
                </p>
              </div>
            )}
          </div>

          {/* Footer: defaults live here so nothing is hidden */}
          <div className="shrink-0 border-t border-border px-5 py-2 flex items-center justify-between">
            <span className="text-[10px] text-text-muted">
              {settings.version === DEFAULT_SETTINGS.version
                ? "Settings persist locally"
                : "Settings schema updated"}
            </span>
            <button
              onClick={onClose}
              className="px-3 py-1.5 rounded text-xs bg-primary/15 text-primary border border-primary/40 hover:bg-primary/25"
            >
              Done
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SettingsSurface;
