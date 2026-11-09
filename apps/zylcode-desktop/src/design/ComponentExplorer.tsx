// ---------------------------------------------------------------------------
// Component explorer (design/ComponentExplorer.tsx)
//
// The list of components the indexer found in `src/`, grouped by what they are:
// the design system in `components/ui`, the application's own components, the
// shell's screens and the page. Selecting one drives the canvas and inspector.
// ---------------------------------------------------------------------------

import { useMemo, useState } from "react";
import { componentsOfKind, searchComponents } from "./catalog";
import type { ComponentEntry, ComponentKind, DesignIndex } from "./types";

type Filter = ComponentKind | "all";

const FILTERS: Array<{ id: Filter; label: string }> = [
  { id: "all", label: "all" },
  { id: "design-system", label: "design system" },
  { id: "component", label: "components" },
  { id: "screen", label: "screens" },
  { id: "page", label: "pages" },
];

interface ComponentExplorerProps {
  index: DesignIndex;
  selectedId: string | null;
  onSelect: (id: string) => void;
}

function KindTag({ kind }: { kind: ComponentKind }) {
  const tone =
    kind === "design-system"
      ? "text-primary border-primary/40"
      : kind === "screen" || kind === "page"
        ? "text-text-muted border-border"
        : "text-text-secondary border-border";
  return (
    <span
      className={`shrink-0 rounded border px-1 py-[1px] font-mono text-[9px] uppercase ${tone}`}
    >
      {kind}
    </span>
  );
}

export function ComponentExplorer({ index, selectedId, onSelect }: ComponentExplorerProps) {
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");

  const rows = useMemo(
    () => searchComponents({ ...index, components: componentsOfKind(index, filter) }, query),
    [index, filter, query],
  );

  const counts = useMemo(() => {
    const map = new Map<Filter, number>();
    for (const f of FILTERS) map.set(f.id, componentsOfKind(index, f.id).length);
    return map;
  }, [index]);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap items-center gap-2 border-b border-border px-3 py-2">
        <div className="flex flex-wrap gap-1">
          {FILTERS.map((f) => (
            <button
              key={f.id}
              type="button"
              onClick={() => setFilter(f.id)}
              className={`rounded border px-2 py-0.5 text-[10px] font-mono ${
                filter === f.id ? "border-primary text-primary" : "border-border text-text-muted"
              }`}
            >
              {f.label} <span className="text-text-muted/70">{counts.get(f.id) ?? 0}</span>
            </button>
          ))}
        </div>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="filter components, classes, tokens…"
          aria-label="Filter components"
          className="ml-auto w-56 rounded border border-border bg-background px-2 py-1 text-[11px] text-text-primary"
        />
      </div>

      <div className="min-h-0 flex-1 overflow-auto" data-testid="component-list">
        <table className="w-full text-left text-[11px]">
          <thead className="sticky top-0 bg-surface text-[10px] uppercase tracking-wider text-text-muted">
            <tr>
              <th className="px-3 py-1.5 font-medium">component</th>
              <th className="px-2 py-1.5 font-medium">source</th>
              <th className="px-2 py-1.5 text-right font-medium">props</th>
              <th className="px-2 py-1.5 text-right font-medium">tokens</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((component: ComponentEntry) => (
              <tr
                key={component.id}
                onClick={() => onSelect(component.id)}
                className={`cursor-pointer border-t border-border/50 ${
                  selectedId === component.id ? "bg-primary/10" : "hover:bg-surface/60"
                }`}
              >
                <td className="px-3 py-1">
                  <span className="flex items-center gap-1.5">
                    <KindTag kind={component.kind} />
                    <span className="truncate text-text-primary">{component.name}</span>
                  </span>
                </td>
                <td className="max-w-[14rem] truncate px-2 py-1 font-mono text-[10px] text-text-muted">
                  {component.file.replace("apps/zylcode-desktop/", "")}:{component.line}
                </td>
                <td className="px-2 py-1 text-right font-mono text-text-muted">
                  {component.propsUnknown ? "?" : component.props.length}
                </td>
                <td className="px-2 py-1 text-right font-mono text-text-muted">
                  {new Set(component.tokens).size}
                </td>
              </tr>
            ))}
            {rows.length === 0 && (
              <tr>
                <td colSpan={4} className="px-3 py-6 text-center text-text-muted">
                  nothing indexed matches “{query}”
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
