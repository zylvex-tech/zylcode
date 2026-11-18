// ---------------------------------------------------------------------------
// Mission Engine client (Phase 3B)
//
// Thin wrapper over the Tauri commands that talk to
// `zylcode_core::mission_engine`. The engine owns the journal; this module
// never invents state. In the browser preview there is no engine process, so
// every call refuses with a reason the UI can show — it does not fall back to
// a simulated mission.
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic } from "./runtime";

export type MissionPhase =
  | "CREATED"
  | "UNDERSTANDING"
  | "PLANNING"
  | "AWAITING_APPROVAL"
  | "EXECUTING"
  | "VERIFYING"
  | "REPAIRING"
  | "BLOCKED"
  | "COMPLETE"
  | "FAILED"
  | "CANCELLED";

export type EvidenceKind = "test" | "ledger" | "file" | "command" | "artifact" | "human";

export interface MissionCitation {
  evidence_id: string;
  kind: EvidenceKind;
  summary?: string;
  cited_at: string;
}

export interface MissionCriterion {
  id: string;
  statement: string;
  citations?: MissionCitation[];
}

export interface MissionStep {
  id: string;
  description: string;
  done: boolean;
}

export interface EngineMission {
  id: string;
  objective: string;
  phase: MissionPhase;
  criteria: MissionCriterion[];
  steps: MissionStep[];
  seq: number;
  created_at: string;
  updated_at: string;
  blocked_reason?: string | null;
  failure_reason?: string | null;
}

export type MissionProgress =
  | { outcome: "awaiting_approval" }
  | { outcome: "executing"; done: number; remaining: number }
  | { outcome: "awaiting_evidence"; unsatisfied: string[] }
  | { outcome: "complete"; mission_id: string }
  | { outcome: "terminal"; phase: MissionPhase };

export interface EngineStatus {
  mission: EngineMission | null;
  journalEntries: number;
  discardedPartialTail: boolean;
  repairedSnapshot: boolean;
  note?: string;
}

export type EngineResult<T> = { ok: true; value: T } | { ok: false; error: string };

/** Truthful reason shown when this surface runs without the desktop engine. */
export const ENGINE_UNAVAILABLE =
  "Mission Engine needs the desktop runtime — this browser preview has no engine process, so no mission state exists here.";

const isDesktop = () => detectEnvironment() === "TAURI_DESKTOP";

function unavailable<T>(): EngineResult<T> {
  return { ok: false, error: ENGINE_UNAVAILABLE };
}

function failed<T>(label: string, err: unknown): EngineResult<T> {
  const message = err instanceof Error ? err.message : String(err);
  recordDiagnostic("mission_engine", `${label} failed: ${message}`);
  return { ok: false, error: message };
}

export function isCriterionCited(criterion: MissionCriterion): boolean {
  return (criterion.citations?.length ?? 0) > 0;
}

export async function fetchEngineStatus(): Promise<EngineResult<EngineStatus>> {
  if (!isDesktop()) return unavailable();
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const raw = await invoke<{
      mission: EngineMission | null;
      journal_entries: number;
      discarded_partial_tail: boolean;
      repaired_snapshot: boolean;
      describe?: string;
    }>("mission_engine_status");
    return {
      ok: true,
      value: {
        mission: raw.mission,
        journalEntries: raw.journal_entries,
        discardedPartialTail: raw.discarded_partial_tail,
        repairedSnapshot: raw.repaired_snapshot,
        note: raw.describe,
      },
    };
  } catch (err) {
    return failed("status", err);
  }
}

export async function createEngineMission(
  objective: string,
  criteria: string[],
  steps: string[],
): Promise<EngineResult<EngineMission>> {
  if (!isDesktop()) return unavailable();
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const mission = await invoke<EngineMission>("mission_engine_create", {
      objective,
      criteria,
      steps,
    });
    return { ok: true, value: mission };
  } catch (err) {
    return failed("create", err);
  }
}

export async function resumeEngineMission(
  approve: boolean,
): Promise<EngineResult<{ progress: MissionProgress; mission: EngineMission | null }>> {
  if (!isDesktop()) return unavailable();
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const res = await invoke<{ progress: MissionProgress; mission: EngineMission | null }>(
      "mission_engine_resume",
      { approve },
    );
    return { ok: true, value: res };
  } catch (err) {
    return failed("resume", err);
  }
}

export async function citeEngineEvidence(
  criterion: string,
  evidence: string,
  kind: EvidenceKind,
  summary: string,
): Promise<EngineResult<EngineMission>> {
  if (!isDesktop()) return unavailable();
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const mission = await invoke<EngineMission>("mission_engine_cite", {
      criterion,
      evidence,
      kind,
      summary,
    });
    return { ok: true, value: mission };
  } catch (err) {
    return failed("cite", err);
  }
}

export async function cancelEngineMission(reason: string): Promise<EngineResult<EngineMission>> {
  if (!isDesktop()) return unavailable();
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const mission = await invoke<EngineMission>("mission_engine_cancel", { reason });
    return { ok: true, value: mission };
  } catch (err) {
    return failed("cancel", err);
  }
}
