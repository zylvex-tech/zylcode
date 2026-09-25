import { useCallback, useEffect, useRef, useState } from "react";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import { fetchGitStatus, type GitStatusState } from "../lib/gitStatus";

/** Color for a porcelain XY pair: staged column dominates, then worktree. */
function statusChip(xy: string): { label: string; cls: string } {
  const x = xy[0] ?? " ";
  const y = xy[1] ?? " ";
  const letter = x !== " " ? x : y;
  if (letter === "?") return { label: "U", cls: "text-sky-400 border-sky-500/40 bg-sky-500/10" };
  switch (letter) {
    case "A":
      return { label: "A", cls: "text-emerald-400 border-emerald-500/40 bg-emerald-500/10" };
    case "M":
      return { label: "M", cls: "text-amber-400 border-amber-500/40 bg-amber-500/10" };
    case "D":
      return { label: "D", cls: "text-red-400 border-red-500/40 bg-red-500/10" };
    case "R":
      return { label: "R", cls: "text-violet-400 border-violet-500/40 bg-violet-500/10" };
    default:
      return { label: letter.trim() || "•", cls: "text-text-muted border-border bg-surface" };
  }
}

function isStaged(xy: string): boolean {
  return (xy[0] ?? " ") !== " " && (xy[0] ?? " ") !== "?";
}

/**
 * Source Control — real git state from the engine: branch + tracking,
 * per-file status chips, and the working-tree diffstat. Staged changes
 * are listed above unstaged, matching how commits are actually built.
 */
export const ChangesPanel: React.FC = () => {
  const [state, setState] = useState<GitStatusState | null>(null);
  const [connected, setConnected] = useState<CapabilityStatus>("BLOCKED");
  const [note, setNote] = useState("connecting…");
  const mounted = useRef(true);

  const refresh = useCallback(async () => {
    const s = await fetchGitStatus();
    if (!mounted.current) return;
    setState(s);
    if (s.kind === "ready") {
      setConnected("AVAILABLE");
      const c = s.data.counts;
      setNote(
        s.data.clean
          ? "working tree clean"
          : `${c.total} change${c.total === 1 ? "" : "s"} · ${c.untracked} untracked`,
      );
    } else if (s.kind === "unavailable") {
      setConnected("BLOCKED");
      setNote(s.reason);
    } else {
      setConnected("LIMITED");
      setNote("reading git state…");
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    const t = setInterval(() => void refresh(), 5000);
    return () => {
      mounted.current = false;
      clearInterval(t);
    };
  }, [refresh]);

  const data = state?.kind === "ready" ? state.data : null;
  const staged = data ? data.entries.filter((e) => isStaged(e.status)) : [];
  const rest = data ? data.entries.filter((e) => !isStaged(e.status)) : [];

  return (
    <div className="flex-1 min-h-0 flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <StatusBadge status={connected} />
        <span className="text-[10px] text-text-muted truncate" title={note}>
          {note}
        </span>
      </div>

      {data && (
        <>
          <div className="flex items-center gap-2 text-xs">
            <span className="font-medium text-text-primary">⎇ {data.branch}</span>
            {data.upstream && (
              <span className="text-text-muted">
                {data.ahead > 0 && <span className="text-emerald-400">↑{data.ahead}</span>}
                {data.behind > 0 && <span className="text-amber-400">↓{data.behind}</span>}
                {data.ahead === 0 && data.behind === 0 && `=${data.upstream}`}
              </span>
            )}
          </div>

          <div className="flex-1 min-h-0 overflow-y-auto space-y-1">
            {staged.length > 0 && (
              <p className="text-[10px] uppercase tracking-wider text-text-muted pt-1">Staged</p>
            )}
            {staged.map((e) => (
              <Row key={`s-${e.path}`} status={e.status} path={e.path} />
            ))}
            {rest.length > 0 && (
              <p className="text-[10px] uppercase tracking-wider text-text-muted pt-1">
                {staged.length > 0 ? "Unstaged & untracked" : "Changes"}
              </p>
            )}
            {rest.map((e) => (
              <Row key={`u-${e.path}`} status={e.status} path={e.path} />
            ))}
            {data.entries.length === 0 && (
              <p className="text-xs text-text-muted py-4 text-center">
                No changes — working tree clean.
              </p>
            )}
          </div>

          {data.diffstat.files_changed > 0 && (
            <div className="border-t border-border pt-1.5 text-[10px] text-text-muted">
              {data.diffstat.files_changed} file
              {data.diffstat.files_changed === 1 ? "" : "s"} changed{" "}
              <span className="text-emerald-400">+{data.diffstat.insertions}</span>{" "}
              <span className="text-red-400">−{data.diffstat.deletions}</span>
            </div>
          )}
        </>
      )}
    </div>
  );
};

function Row({ status, path }: { status: string; path: string }) {
  const chip = statusChip(status);
  return (
    <div className="flex items-center gap-2 py-0.5">
      <span
        className={`shrink-0 w-5 h-5 rounded border flex items-center justify-center text-[10px] font-mono font-bold ${chip.cls}`}
      >
        {chip.label}
      </span>
      <span className="text-xs text-text-primary truncate font-mono" title={path}>
        {path}
      </span>
    </div>
  );
}

export default ChangesPanel;
