// ---------------------------------------------------------------------------
// Design Studio — project document model
//
// Mirrors `src-tauri/src/design_store.rs`. The Rust store is authoritative for
// ids, revisions and timestamps; everything here is what the editor manipulates
// and what the store validates before writing.
// ---------------------------------------------------------------------------

export type NodeKind = "text" | "rect";

export const MAX_NODES = 500;
export const MAX_NAME_CHARS = 80;
export const SCHEMA_VERSION = 1;

export interface DesignNode {
  id: string;
  kind: NodeKind;
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
  color: string;
  fontSize: number;
}

export interface CanvasSettings {
  width: number;
  height: number;
  background: string;
}

export interface DesignProject {
  schemaVersion: number;
  id: string;
  name: string;
  revision: number;
  createdAtMs: number;
  updatedAtMs: number;
  canvas: CanvasSettings;
  nodes: DesignNode[];
}

export interface ProjectSummary {
  id: string;
  name: string;
  revision: number;
  updatedAtMs: number;
  nodeCount: number;
}

export const DEFAULT_CANVAS: CanvasSettings = {
  width: 960,
  height: 600,
  background: "#0b0e14",
};

/** Ids double as filenames in the Rust store, so they are an allow-list. */
export function isSafeId(id: string): boolean {
  return /^[A-Za-z0-9_-]{1,64}$/.test(id);
}

export function newNodeId(): string {
  return `n${Date.now().toString(36)}${Math.floor(Math.random() * 1e6).toString(36)}`;
}

/**
 * A fresh project that the store has not seen yet.
 *
 * The desktop store assigns the real id and revision on create; this shape only
 * exists so the editor has something to render while that call is in flight.
 */
export function draftProject(id: string, name: string, now = Date.now()): DesignProject {
  return {
    schemaVersion: SCHEMA_VERSION,
    id,
    name,
    revision: 1,
    createdAtMs: now,
    updatedAtMs: now,
    canvas: { ...DEFAULT_CANVAS },
    nodes: [],
  };
}

export function newNode(kind: NodeKind, index: number): DesignNode {
  return kind === "text"
    ? {
        id: newNodeId(),
        kind,
        text: "New label",
        x: 60 + index * 12,
        y: 60 + index * 12,
        width: 280,
        height: 56,
        color: "#e6edf3",
        fontSize: 24,
      }
    : {
        id: newNodeId(),
        kind,
        text: "",
        x: 80 + index * 12,
        y: 120 + index * 12,
        width: 240,
        height: 160,
        color: "#3b82f6",
        fontSize: 16,
      };
}

/** Validation messages, empty when the document may be written. */
export function validateProject(project: DesignProject): string[] {
  const errors: string[] = [];
  if (!isSafeId(project.id)) errors.push("project id is not filename-safe");
  const name = project.name.trim();
  if (name.length === 0) errors.push("a project name is required");
  if (name.length > MAX_NAME_CHARS) errors.push(`name exceeds ${MAX_NAME_CHARS} characters`);
  if (project.nodes.length > MAX_NODES) errors.push(`more than ${MAX_NODES} elements`);

  const dimension = (label: string, value: number, min: number, max: number) => {
    if (!Number.isFinite(value)) errors.push(`${label} must be a finite number`);
    else if (value < min || value > max) errors.push(`${label} must be between ${min} and ${max}`);
  };
  const colour = (label: string, value: string) => {
    if (!/^#[0-9a-fA-F]{6}$/.test(value)) errors.push(`${label} must be #rrggbb`);
  };

  dimension("canvas width", project.canvas.width, 1, 100_000);
  dimension("canvas height", project.canvas.height, 1, 100_000);
  colour("canvas background", project.canvas.background);

  for (const node of project.nodes) {
    if (!isSafeId(node.id)) errors.push(`element id ${node.id} is not filename-safe`);
    dimension(`element ${node.id} x`, node.x, -100_000, 100_000);
    dimension(`element ${node.id} y`, node.y, -100_000, 100_000);
    dimension(`element ${node.id} width`, node.width, 1, 100_000);
    dimension(`element ${node.id} height`, node.height, 1, 100_000);
    dimension(`element ${node.id} font size`, node.fontSize, 1, 1_000);
    colour(`element ${node.id} colour`, node.color);
  }
  return errors;
}

/** A short, locale-stable label; the raw epoch stays in the document. */
export function formatUpdated(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return "unknown";
  const iso = new Date(ms).toISOString();
  return `${iso.slice(0, 10)} ${iso.slice(11, 16)} UTC`;
}
