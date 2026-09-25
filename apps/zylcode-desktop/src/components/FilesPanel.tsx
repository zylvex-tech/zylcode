import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import { fetchFileTree, type FileTreeResult } from "../lib/surfaces";

interface TreeNode {
  name: string;
  path: string;
  children: TreeNode[];
  isDir: boolean;
}

/** Build a nested tree from flat relative paths. */
function buildTree(rows: { path: string }[]): TreeNode[] {
  const root: TreeNode = { name: "", path: "", children: [], isDir: true };
  for (const { path } of rows) {
    const parts = path.split("/");
    let cur = root;
    for (let i = 0; i < parts.length; i++) {
      const isLeaf = i === parts.length - 1;
      const name = parts[i];
      let next = cur.children.find((c) => c.name === name && c.isDir === !isLeaf);
      if (!next) {
        next = {
          name,
          path: parts.slice(0, i + 1).join("/"),
          children: [],
          isDir: !isLeaf,
        };
        cur.children.push(next);
      }
      cur = next;
    }
  }
  const sort = (n: TreeNode) => {
    n.children.sort((a, b) =>
      a.isDir === b.isDir ? a.name.localeCompare(b.name) : a.isDir ? -1 : 1,
    );
    n.children.forEach(sort);
  };
  sort(root);
  return root.children;
}

function collectDirPaths(nodes: TreeNode[], acc: Set<string> = new Set()): Set<string> {
  for (const n of nodes) {
    if (n.isDir) {
      acc.add(n.path);
      collectDirPaths(n.children, acc);
    }
  }
  return acc;
}

/**
 * Workspace Files — the real repository tree from the intelligence scanner,
 * with client-side filtering. Directories by the API convention; files by
 * the scanner's language label. Nothing simulated.
 */
export const FilesPanel: React.FC = () => {
  const [tree, setTree] = useState<FileTreeResult | null>(null);
  const [connected, setConnected] = useState<CapabilityStatus>("BLOCKED");
  const [note, setNote] = useState("connecting…");
  const [filter, setFilter] = useState("");
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const mounted = useRef(true);

  const refresh = useCallback(async () => {
    const result = await fetchFileTree();
    if (!mounted.current) return;
    if (result.kind === "ready") {
      setTree(result.data);
      setConnected("AVAILABLE");
      setNote(`${result.data.total_scanned} files indexed`);
    } else if (result.kind === "unavailable") {
      setTree(null);
      setConnected("BLOCKED");
      setNote(result.reason);
    } else {
      setConnected("LIMITED");
      setNote("loading tree…");
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    const t = setInterval(() => void refresh(), 15000);
    return () => {
      mounted.current = false;
      clearInterval(t);
    };
  }, [refresh]);

  const nodes = useMemo(() => buildTree(tree?.rows ?? []), [tree]);

  // Auto-expand top-level dirs once, so the tree opens showing real structure.
  useEffect(() => {
    if (tree && expanded.size === 0) {
      setExpanded(collectDirPaths(nodes.slice(0, 1)));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tree]);

  const visible = useMemo(() => {
    if (!filter.trim()) return nodes;
    const q = filter.trim().toLowerCase();
    const keep = (n: TreeNode): TreeNode | null => {
      const matched = n.name.toLowerCase().includes(q);
      const children = n.children.map(keep).filter((c): c is TreeNode => c !== null);
      if (matched || children.length > 0) {
        return { ...n, children };
      }
      return null;
    };
    return nodes.map(keep).filter((n): n is TreeNode => n !== null);
  }, [nodes, filter]);

  return (
    <div className="flex-1 min-h-0 flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <StatusBadge status={connected} />
        <span className="text-[10px] text-text-muted">{note}</span>
      </div>

      <input
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        placeholder="Filter files…"
        className="w-full bg-surface border border-border rounded px-2 py-1 text-xs text-text-primary placeholder:text-text-muted focus:outline-none focus:border-primary/50"
      />

      <div className="flex-1 min-h-0 overflow-y-auto">
        {visible.map((n) => (
          <Node
            key={n.path}
            node={n}
            depth={0}
            expanded={filter.trim() ? true : expanded.has(n.path)}
            onToggle={(p) =>
              setExpanded((prev) => {
                const next = new Set(prev);
                if (next.has(p)) next.delete(p);
                else next.add(p);
                return next;
              })
            }
            forceOpen={Boolean(filter.trim())}
          />
        ))}
        {tree && visible.length === 0 && (
          <p className="text-xs text-text-muted py-4 text-center">No files match.</p>
        )}
      </div>
    </div>
  );
};

function Node({
  node,
  depth,
  expanded,
  onToggle,
  forceOpen,
}: {
  node: TreeNode;
  depth: number;
  expanded: boolean;
  onToggle: (path: string) => void;
  forceOpen: boolean;
}) {
  const isOpen = forceOpen || expanded;
  return (
    <div>
      <button
        onClick={() => node.isDir && onToggle(node.path)}
        className="w-full flex items-center gap-1.5 py-0.5 text-left hover:bg-surface rounded px-1"
        style={{ paddingLeft: depth * 12 + 4 }}
      >
        <span className="text-[10px] w-3 text-text-muted">{node.isDir ? (isOpen ? "▾" : "▸") : ""}</span>
        <span className={`text-xs truncate ${node.isDir ? "text-text-primary font-medium" : "text-text-muted font-mono"}`} title={node.path}>
          {node.name}
        </span>
      </button>
      {node.isDir &&
        isOpen &&
        node.children.map((c) => (
          <Node
            key={c.path}
            node={c}
            depth={depth + 1}
            expanded={expanded}
            onToggle={onToggle}
            forceOpen={forceOpen}
          />
        ))}
    </div>
  );
}

export default FilesPanel;
