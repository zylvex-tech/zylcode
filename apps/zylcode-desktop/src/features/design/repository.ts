// ---------------------------------------------------------------------------
// Design Studio — project repositories
//
// Two environments, two honest storage stories:
//
//   desktop  → Tauri commands; JSON documents under the app data directory.
//   preview  → `localStorage`; labelled in the UI as browser-only storage.
//
// Both implement the same interface so the workspace, its tests and its error
// handling have one shape. The desktop store owns ids, revisions and timestamps;
// the preview store mirrors that contract so a UI behaviour cannot depend on
// which environment it happens to be running in.
// ---------------------------------------------------------------------------

import { safeInvoke, detectEnvironment, type RuntimeEnvironment } from "../../lib/runtime";
import {
  draftProject,
  isSafeId,
  validateProject,
  type DesignProject,
  type ProjectSummary,
} from "./model";

export type ExportOutcome =
  | { kind: "file"; path: string }
  | { kind: "download"; filename: string }
  | { kind: "unavailable"; reason: string };

export interface DesignRepository {
  readonly kind: "desktop" | "preview";
  /** One sentence the UI can show verbatim, describing where projects live. */
  readonly storageNote: string;
  list(): Promise<ProjectSummary[]>;
  create(name: string): Promise<DesignProject>;
  load(id: string): Promise<DesignProject>;
  save(project: DesignProject): Promise<DesignProject>;
  rename(id: string, name: string): Promise<DesignProject>;
  remove(id: string): Promise<void>;
  exportHtml(id: string, html: string): Promise<ExportOutcome>;
}

async function invokeOrThrow<T>(
  env: RuntimeEnvironment,
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const result = await safeInvoke<T>(env, command, args);
  if (!result.ok) throw new Error(result.reason);
  return result.data;
}

export function desktopRepository(env: RuntimeEnvironment = detectEnvironment()): DesignRepository {
  return {
    kind: "desktop",
    storageNote:
      "Projects are JSON documents in the ZylCode app data directory. Exports are written beside them.",
    list: () => invokeOrThrow<ProjectSummary[]>(env, "design_list"),
    create: (name) => invokeOrThrow<DesignProject>(env, "design_create", { name }),
    load: (id) => invokeOrThrow<DesignProject>(env, "design_load", { id }),
    save: (project) => invokeOrThrow<DesignProject>(env, "design_save", { project }),
    rename: (id, name) => invokeOrThrow<DesignProject>(env, "design_rename", { id, name }),
    remove: async (id) => {
      await invokeOrThrow<null>(env, "design_delete", { id });
    },
    exportHtml: async (id, html) => ({
      kind: "file",
      path: await invokeOrThrow<string>(env, "design_export", { id, html }),
    }),
  };
}

// ---------------------------------------------------------------------------
// Browser preview: localStorage
// ---------------------------------------------------------------------------

const STORAGE_KEY = "zylcode.design.projects.v1";

type StoredProjects = Record<string, DesignProject>;

function readStore(): StoredProjects {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    return parsed && typeof parsed === "object" ? (parsed as StoredProjects) : {};
  } catch {
    return {};
  }
}

function writeStore(all: StoredProjects): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(all));
}

function writeOne(project: DesignProject): void {
  const all = readStore();
  all[project.id] = project;
  writeStore(all);
}

function previewId(): string {
  return `p${Date.now().toString(36)}${Math.floor(Math.random() * 1e9).toString(36)}`;
}

export function previewRepository(): DesignRepository {
  const note =
    "Browser preview: projects are stored in this browser's localStorage only. " +
    "The desktop app stores them as JSON files in its app data directory.";

  return {
    kind: "preview",
    storageNote: note,
    list: async () => {
      const all = readStore();
      return Object.values(all)
        .map((p) => ({
          id: p.id,
          name: p.name,
          revision: p.revision,
          updatedAtMs: p.updatedAtMs,
          nodeCount: p.nodes.length,
        }))
        .sort((a, b) => b.updatedAtMs - a.updatedAtMs || a.id.localeCompare(b.id));
    },
    create: async (name) => {
      const trimmed = name.trim();
      if (!trimmed) throw new Error("a project name is required");
      const now = Date.now();
      const project = draftProject(previewId(), trimmed, now);
      writeOne(project);
      return project;
    },
    load: async (id) => {
      const project = readStore()[id];
      if (!project) throw new Error(`project ${id} is not in preview storage`);
      return project;
    },
    save: async (project) => {
      const errors = validateProject(project);
      if (errors.length > 0) throw new Error(errors.join("; "));
      const stored = readStore()[project.id];
      if (!stored) throw new Error(`project ${project.id} is not in preview storage`);
      if (stored.revision !== project.revision) {
        throw new Error(
          `project ${project.id} changed since it was opened (stored revision ${stored.revision}, offered ${project.revision}); reopen it before saving`,
        );
      }
      const next: DesignProject = {
        ...project,
        revision: stored.revision + 1,
        createdAtMs: stored.createdAtMs,
        updatedAtMs: Math.max(Date.now(), stored.updatedAtMs),
      };
      writeOne(next);
      return next;
    },
    rename: async (id, name) => {
      const trimmed = name.trim();
      if (!trimmed) throw new Error("a project name is required");
      const stored = readStore()[id];
      if (!stored) throw new Error(`project ${id} is not in preview storage`);
      const next: DesignProject = {
        ...stored,
        name: trimmed,
        revision: stored.revision + 1,
        updatedAtMs: Math.max(Date.now(), stored.updatedAtMs),
      };
      writeOne(next);
      return next;
    },
    remove: async (id) => {
      if (!isSafeId(id)) throw new Error(`"${id}" is not a valid project id`);
      const all = readStore();
      delete all[id];
      writeStore(all);
    },
    exportHtml: async (_id, html) => {
      if (typeof URL === "undefined" || typeof URL.createObjectURL !== "function") {
        return {
          kind: "unavailable",
          reason: "this preview environment cannot create a download; run the desktop app",
        };
      }
      const url = URL.createObjectURL(new Blob([html], { type: "text/html" }));
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = "design-export.html";
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(url);
      return { kind: "download", filename: anchor.download };
    },
  };
}

/** The repository the running environment can honour. */
export function designRepository(env: RuntimeEnvironment = detectEnvironment()): DesignRepository {
  return env === "TAURI_DESKTOP" ? desktopRepository(env) : previewRepository();
}
