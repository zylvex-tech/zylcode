// ---------------------------------------------------------------------------
// Design Studio — project lifecycle and editing workspace
//
// Everything here is wired: create/rename/delete persist through the
// repository, canvas edits go through an undo history, Save writes the document,
// Export produces a standalone HTML file. Capabilities that do not exist (visual
// diff, agent repair, code generation) are named as absent in the footer rather
// than rendered as controls.
// ---------------------------------------------------------------------------

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Panel } from "../../components/Panel";
import { StatusBadge } from "../../components/CapabilityStatus";
import { DesignCanvas } from "./DesignCanvas";
import { exportFileName, renderProjectHtml } from "./export";
import {
  canRedo,
  canUndo,
  commit,
  commitGesture,
  initHistory,
  redo,
  replace,
  undo,
  type History,
} from "./history";
import {
  MAX_NAME_CHARS,
  MAX_NODES,
  formatUpdated,
  newNode,
  validateProject,
  type DesignNode,
  type DesignProject,
  type NodeKind,
  type ProjectSummary,
} from "./model";
import { designRepository, type DesignRepository } from "./repository";

type Status = { kind: "idle" | "ok" | "error"; message: string };

interface DesignStudioProps {
  /** Injected in tests; defaults to the repository this environment can honour. */
  repository?: DesignRepository;
}

export function DesignStudio({ repository }: DesignStudioProps = {}) {
  const repo = useMemo(() => repository ?? designRepository(), [repository]);

  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [history, setHistory] = useState<History<DesignProject> | null>(null);
  const [saved, setSaved] = useState<DesignProject | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [status, setStatus] = useState<Status>({ kind: "idle", message: "" });
  const [busy, setBusy] = useState(false);
  const [newName, setNewName] = useState("");
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [deleteArmedId, setDeleteArmedId] = useState<string | null>(null);
  const [pendingSwitchId, setPendingSwitchId] = useState<string | null>(null);

  const project = history?.present ?? null;
  const dirty = Boolean(project && saved && project !== saved);
  const selected = project?.nodes.find((n) => n.id === selectedId) ?? null;
  /** The document as it stood when the current drag began. */
  const gestureStart = useRef<DesignProject | null>(null);

  const refresh = useCallback(async () => {
    try {
      setProjects(await repo.list());
    } catch (e) {
      setStatus({ kind: "error", message: `Could not list projects: ${String(e)}` });
    }
  }, [repo]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const loadProject = useCallback(
    async (id: string) => {
      setBusy(true);
      try {
        const loaded = await repo.load(id);
        setHistory(initHistory(loaded));
        setSaved(loaded);
        setSelectedId(loaded.nodes[0]?.id ?? null);
        setPendingSwitchId(null);
        setStatus({ kind: "ok", message: `Opened "${loaded.name}" (revision ${loaded.revision})` });
      } catch (e) {
        setStatus({ kind: "error", message: `Could not open project: ${String(e)}` });
      } finally {
        setBusy(false);
      }
    },
    [repo],
  );

  const requestOpen = useCallback(
    (id: string) => {
      if (dirty) {
        setPendingSwitchId(id);
        setStatus({
          kind: "error",
          message: "This project has unsaved changes — save or discard them before switching.",
        });
        return;
      }
      void loadProject(id);
    },
    [dirty, loadProject],
  );

  async function createProject() {
    const name = newName.trim();
    if (!name) {
      setStatus({ kind: "error", message: "Enter a project name first." });
      return;
    }
    setBusy(true);
    try {
      const created = await repo.create(name);
      setNewName("");
      setHistory(initHistory(created));
      setSaved(created);
      setSelectedId(null);
      setStatus({
        kind: "ok",
        message: `Created "${created.name}" — add elements, then Save.`,
      });
      await refresh();
    } catch (e) {
      setStatus({ kind: "error", message: `Could not create project: ${String(e)}` });
    } finally {
      setBusy(false);
    }
  }

  async function saveProject() {
    if (!project) return;
    const errors = validateProject(project);
    if (errors.length > 0) {
      setStatus({ kind: "error", message: errors.join("; ") });
      return;
    }
    setBusy(true);
    try {
      const next = await repo.save(project);
      setHistory(replace(history!, next));
      setSaved(next);
      setStatus({ kind: "ok", message: `Saved "${next.name}" as revision ${next.revision}.` });
      await refresh();
    } catch (e) {
      setStatus({ kind: "error", message: `Save failed: ${String(e)}` });
    } finally {
      setBusy(false);
    }
  }

  async function renameProject(id: string) {
    const name = renameValue.trim();
    if (!name) {
      setStatus({ kind: "error", message: "A project name is required." });
      return;
    }
    setBusy(true);
    try {
      const renamed = await repo.rename(id, name);
      if (project?.id === renamed.id) {
        setHistory((h) => (h ? replace(h, renamed) : h));
        setSaved(renamed);
      }
      setRenamingId(null);
      setStatus({ kind: "ok", message: `Renamed to "${renamed.name}".` });
      await refresh();
    } catch (e) {
      setStatus({ kind: "error", message: `Rename failed: ${String(e)}` });
    } finally {
      setBusy(false);
    }
  }

  async function deleteProject(id: string) {
    if (deleteArmedId !== id) {
      setDeleteArmedId(id);
      return;
    }
    setBusy(true);
    try {
      await repo.remove(id);
      if (project?.id === id) {
        setHistory(null);
        setSaved(null);
        setSelectedId(null);
      }
      setDeleteArmedId(null);
      setStatus({ kind: "ok", message: "Project deleted." });
      await refresh();
    } catch (e) {
      setStatus({ kind: "error", message: `Delete failed: ${String(e)}` });
    } finally {
      setBusy(false);
    }
  }

  const edit = useCallback(
    (next: DesignProject) => setHistory((h) => (h ? commit(h, next) : h)),
    [],
  );

  function addNode(kind: NodeKind) {
    if (!project) return;
    if (project.nodes.length >= MAX_NODES) {
      setStatus({ kind: "error", message: `A project may hold at most ${MAX_NODES} elements.` });
      return;
    }
    const node = newNode(kind, project.nodes.length);
    edit({ ...project, nodes: [...project.nodes, node] });
    setSelectedId(node.id);
    setStatus({ kind: "idle", message: "" });
  }

  function updateNode(id: string, patch: Partial<DesignNode>) {
    if (!project) return;
    edit({
      ...project,
      nodes: project.nodes.map((n) => (n.id === id ? { ...n, ...patch } : n)),
    });
  }

  function moveNode(id: string, x: number, y: number) {
    if (!project) return;
    setHistory((h) =>
      h
        ? replace(h, {
            ...h.present,
            nodes: h.present.nodes.map((n) => (n.id === id ? { ...n, x, y } : n)),
          })
        : h,
    );
  }

  function removeNode(id: string) {
    if (!project) return;
    edit({ ...project, nodes: project.nodes.filter((n) => n.id !== id) });
    setSelectedId(null);
  }

  async function exportProject() {
    if (!project) return;
    const html = renderProjectHtml(project);
    setBusy(true);
    try {
      const outcome = await repo.exportHtml(project.id, html);
      if (outcome.kind === "file") {
        setStatus({ kind: "ok", message: `Exported standalone HTML to ${outcome.path}` });
      } else if (outcome.kind === "download") {
        setStatus({ kind: "ok", message: `Downloaded ${exportFileName(project)}` });
      } else {
        setStatus({ kind: "error", message: outcome.reason });
      }
    } catch (e) {
      setStatus({ kind: "error", message: `Export failed: ${String(e)}` });
    } finally {
      setBusy(false);
    }
  }

  // Keyboard: undo/redo/save/delete only when focus is not in a field.
  const shortcuts = useRef({ undo, redo, saveProject, removeNode, selectedId });
  shortcuts.current = { undo, redo, saveProject, removeNode, selectedId };

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null;
      const typing = target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName ?? "");
      if (typing) return;
      const mod = event.metaKey || event.ctrlKey;
      if (mod && event.key.toLowerCase() === "s") {
        event.preventDefault();
        void shortcuts.current.saveProject();
        return;
      }
      if (mod && event.key.toLowerCase() === "z") {
        event.preventDefault();
        setHistory((h) => (h ? (event.shiftKey ? redo(h) : undo(h)) : h));
        return;
      }
      if ((event.key === "Delete" || event.key === "Backspace") && shortcuts.current.selectedId) {
        event.preventDefault();
        shortcuts.current.removeNode(shortcuts.current.selectedId!);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const undoable = history ? canUndo(history) : false;
  const redoable = history ? canRedo(history) : false;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <h1 className="text-base font-semibold tracking-tight">Design Studio</h1>
          <StatusBadge status="LIMITED" size="xs" />
          {dirty && (
            <span className="font-mono text-[10px] text-warning" data-testid="dirty">
              UNSAVED
            </span>
          )}
        </div>
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            onClick={() => setHistory((h) => (h ? undo(h) : h))}
            disabled={!undoable}
            className="rounded border border-border px-2 py-1 text-xs hover:border-primary/50 disabled:opacity-40"
          >
            Undo
          </button>
          <button
            type="button"
            onClick={() => setHistory((h) => (h ? redo(h) : h))}
            disabled={!redoable}
            className="rounded border border-border px-2 py-1 text-xs hover:border-primary/50 disabled:opacity-40"
          >
            Redo
          </button>
          <button
            type="button"
            onClick={() => void saveProject()}
            disabled={!project || busy}
            className="rounded bg-primary px-3 py-1 text-xs font-medium text-primary-foreground hover:bg-primary-hover disabled:opacity-40"
          >
            Save
          </button>
          <button
            type="button"
            onClick={() => void exportProject()}
            disabled={!project || busy}
            className="rounded border border-border px-2 py-1 text-xs hover:border-primary/50 disabled:opacity-40"
          >
            Export HTML
          </button>
        </div>
      </div>

      <p className="text-[11px] text-text-muted">{repo.storageNote}</p>

      <div className="grid grid-cols-1 gap-3 xl:grid-cols-[220px_minmax(0,1fr)_240px]">
        {/* Projects */}
        <Panel title="PROJECTS">
          <div className="flex flex-col gap-2">
            <input
              aria-label="New project name"
              value={newName}
              maxLength={MAX_NAME_CHARS}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void createProject();
              }}
              placeholder="New project name"
              className="w-full rounded border border-border bg-background px-2 py-1 text-xs"
            />
            <button
              type="button"
              onClick={() => void createProject()}
              disabled={busy}
              className="rounded border border-border px-2 py-1 text-xs hover:border-primary/50 disabled:opacity-40"
            >
              Create project
            </button>
            {projects.length === 0 ? (
              <p className="text-[11px] text-text-muted">
                No design projects yet. Name one above and create it.
              </p>
            ) : (
              <ul className="flex flex-col gap-1">
                {projects.map((p) => (
                  <li key={p.id} className="rounded border border-border px-2 py-1.5">
                    {renamingId === p.id ? (
                      <div className="flex flex-col gap-1">
                        <input
                          aria-label={`Rename ${p.name}`}
                          value={renameValue}
                          maxLength={MAX_NAME_CHARS}
                          onChange={(e) => setRenameValue(e.target.value)}
                          className="w-full rounded border border-border bg-background px-1.5 py-0.5 text-xs"
                        />
                        <div className="flex gap-1">
                          <button
                            type="button"
                            onClick={() => void renameProject(p.id)}
                            className="rounded bg-primary px-2 py-0.5 text-[11px] text-primary-foreground"
                          >
                            Save name
                          </button>
                          <button
                            type="button"
                            onClick={() => setRenamingId(null)}
                            className="rounded border border-border px-2 py-0.5 text-[11px]"
                          >
                            Cancel
                          </button>
                        </div>
                      </div>
                    ) : (
                      <>
                        <button
                          type="button"
                          onClick={() => requestOpen(p.id)}
                          className={`w-full text-left text-xs ${
                            project?.id === p.id ? "text-primary" : ""
                          }`}
                        >
                          <span className="truncate">{p.name}</span>
                        </button>
                        <p className="mt-0.5 font-mono text-[10px] text-text-muted">
                          rev {p.revision} · {p.nodeCount} el · {formatUpdated(p.updatedAtMs)}
                        </p>
                        <div className="mt-1 flex gap-1">
                          <button
                            type="button"
                            onClick={() => {
                              setRenamingId(p.id);
                              setRenameValue(p.name);
                            }}
                            className="text-[10px] text-text-muted hover:text-primary"
                          >
                            Rename
                          </button>
                          <button
                            type="button"
                            onClick={() => void deleteProject(p.id)}
                            className="text-[10px] text-text-muted hover:text-error"
                          >
                            {deleteArmedId === p.id ? "Confirm delete" : "Delete"}
                          </button>
                        </div>
                      </>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </Panel>

        {/* Canvas */}
        <Panel
          title={project ? project.name : "CANVAS"}
          right={
            project ? (
              <span className="font-mono text-[10px] text-text-muted">
                {project.canvas.width}×{project.canvas.height}
              </span>
            ) : undefined
          }
        >
          {project ? (
            <div className="flex flex-col gap-2">
              <div className="flex flex-wrap gap-1.5">
                <button
                  type="button"
                  onClick={() => addNode("text")}
                  className="rounded border border-border px-2 py-1 text-xs hover:border-primary/50"
                >
                  Add text
                </button>
                <button
                  type="button"
                  onClick={() => addNode("rect")}
                  className="rounded border border-border px-2 py-1 text-xs hover:border-primary/50"
                >
                  Add shape
                </button>
                <span className="ml-auto self-center text-[10px] text-text-muted">
                  Drag to move · Delete removes selection
                </span>
              </div>
              <DesignCanvas
                project={project}
                selectedId={selectedId}
                onSelect={setSelectedId}
                onMoveStart={() => {
                  gestureStart.current = history?.present ?? null;
                }}
                onMove={moveNode}
                onMoveEnd={() => {
                  const before = gestureStart.current;
                  gestureStart.current = null;
                  if (!before) return;
                  setHistory((h) => (h ? commitGesture(h, before) : h));
                }}
              />
            </div>
          ) : (
            <div className="flex flex-col gap-2 py-8 text-center">
              <p className="text-sm font-medium">Start with a project</p>
              <ol className="mx-auto max-w-sm list-decimal space-y-1 text-left text-xs text-text-muted">
                <li>Name a project and create it.</li>
                <li>Add text and shapes, then arrange them on the canvas.</li>
                <li>Save, close the app, and reopen — the document reloads.</li>
                <li>Export HTML for a self-contained result you can open anywhere.</li>
              </ol>
            </div>
          )}
        </Panel>

        {/* Inspector */}
        <Panel title="INSPECTOR">
          {selected && project ? (
            <div className="flex flex-col gap-2 text-xs">
              <label className="flex flex-col gap-0.5">
                <span className="text-text-muted">Text</span>
                <input
                  aria-label="Element text"
                  value={selected.text}
                  disabled={selected.kind === "rect"}
                  onChange={(e) => updateNode(selected.id, { text: e.target.value })}
                  className="rounded border border-border bg-background px-2 py-1 disabled:opacity-40"
                />
              </label>
              <div className="grid grid-cols-2 gap-1.5">
                {(["x", "y", "width", "height"] as const).map((field) => (
                  <label key={field} className="flex flex-col gap-0.5">
                    <span className="text-text-muted">{field}</span>
                    <input
                      aria-label={`Element ${field}`}
                      type="number"
                      value={Math.round(selected[field])}
                      onChange={(e) =>
                        updateNode(selected.id, {
                          [field]: Number(e.target.value),
                        } as Partial<DesignNode>)
                      }
                      className="rounded border border-border bg-background px-2 py-1"
                    />
                  </label>
                ))}
              </div>
              <label className="flex items-center justify-between gap-2">
                <span className="text-text-muted">Colour</span>
                <input
                  aria-label="Element colour"
                  type="color"
                  value={selected.color}
                  onChange={(e) => updateNode(selected.id, { color: e.target.value })}
                  className="h-6 w-10 rounded border border-border bg-background"
                />
              </label>
              <label className="flex items-center justify-between gap-2">
                <span className="text-text-muted">Font size</span>
                <input
                  aria-label="Element font size"
                  type="number"
                  min={1}
                  max={1000}
                  value={Math.round(selected.fontSize)}
                  onChange={(e) => updateNode(selected.id, { fontSize: Number(e.target.value) })}
                  className="w-20 rounded border border-border bg-background px-2 py-1"
                />
              </label>
              <button
                type="button"
                onClick={() => removeNode(selected.id)}
                className="mt-1 rounded border border-border px-2 py-1 text-[11px] hover:border-error/60 hover:text-error"
              >
                Remove element
              </button>
            </div>
          ) : (
            <p className="text-[11px] text-text-muted">
              Select an element on the canvas to edit its text, position, size and colour.
            </p>
          )}
        </Panel>
      </div>

      {pendingSwitchId && (
        <div className="flex flex-wrap items-center gap-2 rounded border border-warning/40 bg-warning/10 px-3 py-2 text-xs">
          <span className="text-warning">Unsaved changes in “{project?.name}”.</span>
          <button
            type="button"
            onClick={() => {
              void saveProject();
              setPendingSwitchId(null);
            }}
            className="rounded bg-primary px-2 py-0.5 text-[11px] text-primary-foreground"
          >
            Save and switch
          </button>
          <button
            type="button"
            onClick={() => {
              const target = pendingSwitchId;
              setPendingSwitchId(null);
              void loadProject(target);
            }}
            className="rounded border border-border px-2 py-0.5 text-[11px]"
          >
            Discard and switch
          </button>
          <button
            type="button"
            onClick={() => setPendingSwitchId(null)}
            className="text-[11px] text-text-muted hover:text-text-primary"
          >
            Stay here
          </button>
        </div>
      )}

      {status.message && (
        <p
          role="status"
          className={`text-xs ${status.kind === "error" ? "text-error" : "text-text-muted"}`}
        >
          {status.message}
        </p>
      )}

      <p className="text-[11px] text-text-muted">
        Implemented: projects, canvas editing with undo/redo, inspector, save and standalone HTML
        export. Not implemented in this build: visual diff, design↔code generation, agent repair and
        Figma import — no control here pretends otherwise.
      </p>
    </div>
  );
}
