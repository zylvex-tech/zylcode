// ---------------------------------------------------------------------------
// Mission queue service (lib/missions.ts)
//
// Build/Plan missions backed by the real queue: browser preview POSTs to
// /api/missions through the Vite proxy; the desktop app uses the
// mission_* IPC commands. Both drain the queue by actually running the
// pipeline — a "done" mission means the real test suite passed.
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic } from "./runtime";

export type MissionMode = "build" | "plan";
export type MissionState = "queued" | "running" | "done" | "failed";

export interface Mission {
  id: string;
  task: string;
  mode: MissionMode;
  state: MissionState;
  createdAt: string;
  updatedAt: string;
  ledgerSession?: string;
  summary?: string;
}

function normalize(raw: Record<string, unknown>): Mission {
  return {
    id: String(raw.id ?? ""),
    task: String(raw.task ?? ""),
    mode: (raw.mode === "plan" ? "plan" : "build") as MissionMode,
    state: (["queued", "running", "done", "failed"].includes(String(raw.state))
      ? raw.state
      : "queued") as MissionState,
    createdAt: String(raw.createdAt ?? raw.created_at ?? ""),
    updatedAt: String(raw.updatedAt ?? raw.updated_at ?? ""),
    ledgerSession: (raw.ledgerSession ?? raw.ledger_session) as string | undefined,
    summary: (raw.summary ?? undefined) as string | undefined,
  };
}

const isDesktop = () => detectEnvironment() === "TAURI_DESKTOP";

export async function listMissions(): Promise<Mission[]> {
  try {
    if (isDesktop()) {
      const { invoke } = await import("@tauri-apps/api/core");
      const res = await invoke<{ missions: Record<string, unknown>[] }>("missions_list");
      return (res.missions ?? []).map(normalize);
    }
    const res = await fetch("/api/missions");
    if (!res.ok) return [];
    const data = await res.json();
    return (data.missions ?? []).map(normalize);
  } catch (err) {
    recordDiagnostic("missions", `list failed: ${String(err)}`);
    return [];
  }
}

export async function enqueueMission(task: string, mode: MissionMode): Promise<Mission | null> {
  try {
    if (isDesktop()) {
      const { invoke } = await import("@tauri-apps/api/core");
      const raw = await invoke<Record<string, unknown>>("mission_enqueue", {
        task,
        mode,
      });
      return normalize(raw as Record<string, unknown>);
    }
    const res = await fetch("/api/missions", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ task, mode }),
    });
    if (!res.ok) return null;
    return normalize(await res.json());
  } catch (err) {
    recordDiagnostic("missions", `enqueue failed: ${String(err)}`);
    return null;
  }
}

/** Drain the queue: runs at most one mission, blocking until it finishes. */
export async function runNextMission(): Promise<Record<string, unknown> | null> {
  try {
    if (isDesktop()) {
      const { invoke } = await import("@tauri-apps/api/core");
      return (await invoke("mission_run_next")) as Record<string, unknown>;
    }
    const res = await fetch("/api/missions/run-next", { method: "POST" });
    if (!res.ok) return null;
    return await res.json();
  } catch (err) {
    recordDiagnostic("missions", `run-next failed: ${String(err)}`);
    return null;
  }
}

export async function clearMissions(): Promise<boolean> {
  try {
    if (isDesktop()) return true; // queue lives with the workspace file
    const res = await fetch("/api/missions/clear", { method: "POST" });
    return res.ok;
  } catch {
    return false;
  }
}
