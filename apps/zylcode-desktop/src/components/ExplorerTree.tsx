import { useEffect, useMemo, useState } from "react";
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

function renderNode(node: TreeNode, depth: number, out: React.ReactNode[], keyPrefix: string) {
  const pad = { paddingLeft: `${depth * 12 + 4}px` };
  if (node.isDir) {
    out.push(
      <div key={`d:${keyPrefix}`} style={pad} className="text-[11px] text-text-muted font-mono">
        📁 {node.name}
      </div>,
    );
    const children = [...node.children.values()].sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    for (const child of children) renderNode(child, depth + 1, out, `${keyPrefix}/${child.name}`);
  } else {
    out.push(
      <div
        key={`f:${keyPrefix}`}
        style={pad}
        className="text-[11px] font-mono truncate"
        title={`${node.path} (${node.language})`}
      >
        📄 {node.name}
      </div>,
    );
  }
}

/**
 * Live Explorer: the scanner's actual file tree (gitignore-aware), nested
 * client-side from the flat real rows. No fake "sample project" entries.
 */
export function ExplorerTree() {
  const [state, setState] = useState<
    { kind: "loading" } | { kind: "unavailable"; reason: string } | { kind: "ready"; data: FileTreeResult }
  >({ kind: "loading" });

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
          <div className="max-h-[70vh] overflow-y-auto">
            {[...tree.values()]
              .sort((a, b) => (a.isDir !== b.isDir ? (a.isDir ? -1 : 1) : a.name.localeCompare(b.name)))
              .flatMap((node) => {
                const out: React.ReactNode[] = [];
                renderNode(node, 0, out, node.path);
                return out;
              })}
          </div>
        )}
      </div>
    </Panel>
  );
}
