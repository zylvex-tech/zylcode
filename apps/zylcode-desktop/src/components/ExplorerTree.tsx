import React, { useEffect, useMemo, useState } from "react";
import { Panel } from "./Panel";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import { fetchFileTree, type FileTreeResult } from "../lib/surfaces";

type TreeNode = {
  name: string;
  path: string;
  isDir: boolean;
  language?: string;
  children: Map<string, TreeNode>;
};

function insertPath(rootMap: Map<string, TreeNode>, path: string, language: string) {
  const parts = path.split(/[\\/]/).filter(Boolean);
  let map = rootMap;
  let acc = "";
  for (let i = 0; i < parts.length; i++) {
    const name = parts[i];
    acc = acc ? `${acc}/${name}` : name;
    const isDir = i < parts.length - 1;
    let node = map.get(name);
    if (!node) {
      node = { name, path: acc, isDir, language, children: new Map() };
      map.set(name, node);
    }
    if (!isDir) node.language = language;
    map = node.children;
  }
}

interface ExplorerTreeProps {
  /** Called when a real file row is clicked; opens the file in the editor. */
  onOpenFile?: (path: string) => void;
  /** Files currently open in the editor, for the active/dirty marker. */
  activePath?: string | null;
  compact?: boolean;
}

function sortChildren(map: Map<string, TreeNode>): TreeNode[] {
  return [...map.values()].sort((a, b) => {
    if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
}

/**
 * Live Explorer: the scanner's actual file tree (gitignore-aware), nested
 * client-side from the flat real rows. Directories collapse; files open in
 * the central editor. No fake "sample project" entries.
 */
export function ExplorerTree({ onOpenFile, activePath, compact }: ExplorerTreeProps) {
  const [state, setState] = useState<
    { kind: "loading" } | { kind: "unavailable"; reason: string } | { kind: "ready"; data: FileTreeResult }
  >({ kind: "loading" });
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set(["apps", "crates", "src"]));

  useEffect(() => {
    let cancelled = false;
    fetchFileTree().then((next) => {
      if (!cancelled && next.kind !== "loading") setState(next);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const headerStatus: CapabilityStatus =
    state.kind === "ready" ? "AVAILABLE" : state.kind === "loading" ? "LIMITED" : "BLOCKED";

  const tree = useMemo(() => {
    if (state.kind !== "ready") return null;
    const map = new Map<string, TreeNode>();
    for (const row of state.data.rows) insertPath(map, row.path, row.language);
    return map;
  }, [state]);

  const toggle = (path: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const renderNode = (node: TreeNode, depth: number, out: React.ReactNode[], keyPrefix: string) => {
    const pad = { paddingLeft: `${depth * 11 + 4}px` };
    if (node.isDir) {
      const open = expanded.has(node.path);
      out.push(
        <button
          key={`d:${keyPrefix}`}
          style={pad}
          onClick={() => toggle(node.path)}
          className="w-full text-left text-[11px] font-mono text-text-secondary hover:bg-surface-hover rounded flex items-center gap-1 py-0.5"
        >
          <span className="text-[9px] w-2 text-text-muted">{open ? "▾" : "▸"}</span>
          <span>{open ? "📂" : "📁"}</span>
          <span className="truncate">{node.name}</span>
        </button>,
      );
      if (open) {
        for (const child of sortChildren(node.children)) {
          renderNode(child, depth + 1, out, `${keyPrefix}/${child.name}`);
        }
      }
    } else {
      const active = activePath === node.path;
      out.push(
        <button
          key={`f:${keyPrefix}`}
          style={pad}
          onClick={() => onOpenFile?.(node.path)}
          className={`w-full text-left text-[11px] font-mono truncate rounded flex items-center gap-1 py-0.5 ${
            active ? "bg-primary/15 text-primary" : "text-text-muted hover:text-text-primary hover:bg-surface-hover"
          }`}
          title={`${node.path} (${node.language})`}
        >
          <span className="w-2" />
          <span>📄</span>
          <span className="truncate">{node.name}</span>
        </button>,
      );
    }
  };

  const rows = tree ? [...tree.values()].flatMap((node) => {
    const out: React.ReactNode[] = [];
    renderNode(node, 0, out, node.path);
    return out;
  }) : [];

  return (
    <Panel title="EXPLORER">
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <StatusBadge status={headerStatus} size="xs" />
          {state.kind === "ready" && (
            <span className="text-[10px] font-mono text-text-muted">
              {state.data.total_scanned} files
              {state.data.truncated ? " (truncated)" : ""}
            </span>
          )}
        </div>

        {state.kind === "unavailable" && <p className="text-xs text-text-muted">{state.reason}</p>}
        {state.kind === "loading" && <p className="text-xs text-text-muted">Scanning the repository…</p>}

        {state.kind === "ready" && tree && (
          <div className={compact ? "flex-1 min-h-0 overflow-y-auto" : "max-h-[70vh] overflow-y-auto"}>
            {rows}
          </div>
        )}
      </div>
    </Panel>
  );
}
