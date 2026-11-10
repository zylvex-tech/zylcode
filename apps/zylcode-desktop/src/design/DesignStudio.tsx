// ---------------------------------------------------------------------------
// Design Studio surface (design/DesignStudio.tsx)
//
// The shell surface at `design`. It is a view over the generated index: tokens,
// components, a canvas that mounts the real components, an inspector that reads
// both the source and the rendered result, and an accessibility panel that
// separates static findings from measured contrast.
//
// This module owns layout and selection state only. The indexer produces the
// data, `catalog.ts` answers questions about it, and each panel renders one
// answer.
// ---------------------------------------------------------------------------

import { useEffect, useMemo, useState } from "react";
import { AccessibilityPanel } from "./AccessibilityPanel";
import { ComponentExplorer } from "./ComponentExplorer";
import { getComponent, mountPlan } from "./catalog";
import { DesignCanvas, type CanvasSelection } from "./DesignCanvas";
import indexData from "./generated/design-index.json";
import { Inspector } from "./Inspector";
import { TokenExplorer } from "./TokenExplorer";
import type { DesignIndex } from "./types";

const index = indexData as unknown as DesignIndex;

type StudioTab = "tokens" | "components" | "canvas" | "inspector" | "accessibility";

const TABS: Array<{ id: StudioTab; label: string }> = [
  { id: "tokens", label: "tokens" },
  { id: "components", label: "components" },
  { id: "canvas", label: "canvas" },
  { id: "inspector", label: "inspector" },
  { id: "accessibility", label: "accessibility" },
];

function defaultComponentId(): string | null {
  const preferred = index.components.find((c) => c.id === "components/ui/index#Button");
  if (preferred) return preferred.id;
  const designSystem = index.components.find((c) => c.kind === "design-system");
  return (designSystem ?? index.components[0])?.id ?? null;
}

function initialTheme(): string {
  if (typeof document !== "undefined") {
    const applied = document.documentElement.getAttribute("data-theme");
    if (applied && index.themes.some((t) => t.id === applied)) return applied;
  }
  return index.themes[0]?.id ?? "midnight-pro";
}

export function DesignStudio() {
  const [tab, setTab] = useState<StudioTab>("canvas");
  const [theme, setTheme] = useState(initialTheme);
  const [selectedId, setSelectedId] = useState<string | null>(defaultComponentId);
  const [tokenName, setTokenName] = useState<string | null>(null);
  const [width, setWidth] = useState(1024);
  const [zoom, setZoom] = useState(1);
  const [props, setProps] = useState<Record<string, unknown>>({});
  const [selection, setSelection] = useState<CanvasSelection | null>(null);

  const entry = useMemo(
    () => (selectedId ? (getComponent(index, selectedId) ?? null) : null),
    [selectedId],
  );

  // A new component starts from the props the canvas can supply, not from the
  // previous component's values.
  useEffect(() => {
    setProps(entry ? mountPlan(entry).initialProps : {});
    setSelection(null);
  }, [entry]);

  const plan = entry ? mountPlan(entry) : null;

  const openComponent = (id: string) => {
    setSelectedId(id);
    setTab("canvas");
  };

  const inspector = (
    <Inspector
      index={index}
      entry={entry}
      theme={theme}
      props={props}
      onPropChange={(name, value) => setProps((current) => ({ ...current, [name]: value }))}
      selection={selection}
      onSelectComponent={openComponent}
      defaultTab={tab === "canvas" ? "component" : tab === "inspector" ? "source" : "component"}
    />
  );

  return (
    <div className="flex min-h-0 flex-1 flex-col" data-testid="design-studio">
      <header className="flex flex-wrap items-center gap-2 border-b border-border px-3 py-2">
        <div>
          <p className="font-mono text-[11px] tracking-[0.16em] text-primary uppercase">
            Design Studio
          </p>
          <p className="mt-0.5 text-[10px] text-text-muted">
            {index.stats.components + index.stats.designSystemComponents} components ·{" "}
            {index.stats.tokens} tokens · {index.stats.themes} themes · {index.stats.tokenUsages}{" "}
            token usages · indexed from {index.stats.files} files
          </p>
        </div>

        <div
          className="ml-auto flex flex-wrap items-center gap-1"
          role="tablist"
          aria-label="Design Studio views"
        >
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={tab === t.id}
              onClick={() => setTab(t.id)}
              className={`rounded border px-2 py-0.5 text-[10px] font-mono ${
                tab === t.id ? "border-primary text-primary" : "border-border text-text-muted"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>

        <select
          value={theme}
          onChange={(e) => setTheme(e.target.value)}
          aria-label="Canvas theme"
          className="rounded border border-border bg-background px-1 py-1 text-[11px] text-text-primary"
        >
          {index.themes.map((t) => (
            <option key={t.id} value={t.id}>
              {t.id}
            </option>
          ))}
        </select>
      </header>

      <div className="flex min-h-0 flex-1">
        {tab === "tokens" && (
          <TokenExplorer
            index={index}
            theme={theme}
            onTheme={setTheme}
            selected={tokenName}
            onSelect={setTokenName}
            onOpenComponent={openComponent}
          />
        )}

        {tab === "components" && (
          <>
            <div className="flex min-w-0 flex-1 flex-col">
              <ComponentExplorer index={index} selectedId={selectedId} onSelect={setSelectedId} />
            </div>
            <aside className="hidden w-96 shrink-0 flex-col border-l border-border xl:flex">
              {inspector}
            </aside>
          </>
        )}

        {tab === "canvas" && (
          <>
            <DesignCanvas
              index={index}
              entry={entry}
              theme={theme}
              width={width}
              zoom={zoom}
              props={props}
              selection={selection}
              onWidth={setWidth}
              onZoom={setZoom}
              onSelect={setSelection}
              blockedReason={plan && !plan.mountable ? plan.reason : null}
            />
            <aside className="hidden w-96 shrink-0 flex-col border-l border-border lg:flex">
              {inspector}
            </aside>
          </>
        )}

        {tab === "inspector" && (
          <>
            <div className="hidden w-2/5 min-w-0 flex-col border-r border-border lg:flex">
              <ComponentExplorer index={index} selectedId={selectedId} onSelect={setSelectedId} />
            </div>
            <div className="flex min-w-0 flex-1 flex-col">{inspector}</div>
          </>
        )}

        {tab === "accessibility" && (
          <AccessibilityPanel index={index} entry={entry} theme={theme} />
        )}
      </div>

      <footer className="border-t border-border px-3 py-1.5 text-[10px] text-text-muted">
        Indexed from the application&apos;s source by{" "}
        <span className="font-mono">scripts/design-index.mjs</span>. Selection and prop edits are
        local to this surface; nothing here writes to the repository.
      </footer>
    </div>
  );
}
