import { useCallback, useEffect, useState } from "react";
import { Panel } from "./Panel";
import {
  cancelEngineMission,
  citeEngineEvidence,
  createEngineMission,
  fetchEngineStatus,
  isCriterionCited,
  resumeEngineMission,
  type EngineMission,
  type EngineStatus,
  type EvidenceKind,
  type MissionPhase,
} from "../lib/missionEngine";

// ---------------------------------------------------------------------------
// Mission Engine surface (Phase 3B)
//
// This panel drives the durable engine, not the scheduling queue: a mission is
// created with explicit acceptance criteria, advanced through the eleven
// states, and can only report COMPLETE when every criterion carries a cited
// proof. The journal lives in the workspace, so state here is reloaded from
// disk rather than held in React.
// ---------------------------------------------------------------------------

function toLines(value: string): string[] {
  return value
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

const PHASE_CLASS: Record<MissionPhase, string> = {
  CREATED: "border-border text-text-muted",
  UNDERSTANDING: "border-primary/40 text-primary",
  PLANNING: "border-primary/40 text-primary",
  AWAITING_APPROVAL: "border-warning/50 text-warning",
  EXECUTING: "border-primary/60 text-primary",
  VERIFYING: "border-warning/50 text-warning",
  REPAIRING: "border-warning/50 text-warning",
  BLOCKED: "border-error/50 text-error",
  COMPLETE: "border-success/50 text-success",
  FAILED: "border-error/50 text-error",
  CANCELLED: "border-border text-text-muted",
};

const TERMINAL: MissionPhase[] = ["COMPLETE", "FAILED", "CANCELLED"];

function PhaseBadge({ phase }: { phase: MissionPhase }) {
  return (
    <span
      data-testid="mission-phase"
      className={`rounded border px-2 py-0.5 font-mono text-[10px] ${PHASE_CLASS[phase]}`}
    >
      {phase}
    </span>
  );
}

function CreateForm({
  busy,
  onCreate,
}: {
  busy: boolean;
  onCreate: (objective: string, criteria: string[], steps: string[]) => void;
}) {
  const [objective, setObjective] = useState("");
  const [criteriaText, setCriteriaText] = useState("");
  const [stepsText, setStepsText] = useState("");

  const criteria = toLines(criteriaText);
  const ready = objective.trim().length > 0 && criteria.length > 0;

  return (
    <div className="space-y-2">
      <label className="block space-y-1 text-xs text-text-muted">
        <span>Objective</span>
        <textarea
          value={objective}
          onChange={(e) => setObjective(e.target.value)}
          rows={2}
          placeholder="What must this mission achieve?"
          className="w-full resize-none rounded-md border border-border bg-background px-3 py-2 text-sm text-text-primary outline-none focus:border-primary"
        />
      </label>
      <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
        <label className="block space-y-1 text-xs text-text-muted">
          <span>Acceptance criteria (one per line)</span>
          <textarea
            value={criteriaText}
            onChange={(e) => setCriteriaText(e.target.value)}
            rows={4}
            placeholder={"tests pass\ndocs updated"}
            className="w-full resize-none rounded-md border border-border bg-background px-3 py-2 text-sm text-text-primary outline-none focus:border-primary"
          />
        </label>
        <label className="block space-y-1 text-xs text-text-muted">
          <span>Steps (one per line, optional)</span>
          <textarea
            value={stepsText}
            onChange={(e) => setStepsText(e.target.value)}
            rows={4}
            placeholder={"write code\nrun tests"}
            className="w-full resize-none rounded-md border border-border bg-background px-3 py-2 text-sm text-text-primary outline-none focus:border-primary"
          />
        </label>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-[11px] text-text-muted">
          At least one criterion is required: a mission with none could never be honestly complete.
        </p>
        <button
          onClick={() => onCreate(objective.trim(), criteria, toLines(stepsText))}
          disabled={busy || !ready}
          className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary-hover disabled:cursor-not-allowed disabled:opacity-50"
        >
          CREATE MISSION
        </button>
      </div>
    </div>
  );
}

function MissionBody({
  mission,
  busy,
  note,
  error,
  onResume,
  onCite,
  onCancel,
}: {
  mission: EngineMission;
  busy: boolean;
  note: string | null;
  error: string | null;
  onResume: (approve: boolean) => void;
  onCite: (criterion: string, evidence: string, kind: EvidenceKind) => void;
  onCancel: (reason: string) => void;
}) {
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [kinds, setKinds] = useState<Record<string, EvidenceKind>>({});
  const [cancelReason, setCancelReason] = useState("");

  const terminal = TERMINAL.includes(mission.phase);
  const doneSteps = mission.steps.filter((s) => s.done).length;
  const cited = mission.criteria.filter(isCriterionCited).length;

  const primaryAction =
    mission.phase === "AWAITING_APPROVAL"
      ? { label: "APPROVE & RUN", approve: true }
      : mission.phase === "EXECUTING" || mission.phase === "REPAIRING"
        ? { label: "RESUME", approve: true }
        : mission.phase === "VERIFYING"
          ? { label: "CHECK COMPLETION", approve: false }
          : mission.phase === "BLOCKED"
            ? null
            : terminal
              ? null
              : { label: "ADVANCE", approve: false };

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-2">
        <PhaseBadge phase={mission.phase} />
        <p className="text-sm font-medium text-text-primary">{mission.objective}</p>
        <span className="font-mono text-[10px] text-text-muted">{mission.id.slice(0, 8)}</span>
      </div>

      {mission.blocked_reason && (
        <p className="text-[11px] text-error">Blocked: {mission.blocked_reason}</p>
      )}
      {mission.failure_reason && (
        <p className="text-[11px] text-error">Failed: {mission.failure_reason}</p>
      )}

      <div className="space-y-1">
        <p className="text-[10px] uppercase tracking-widest text-text-muted">
          Acceptance criteria — {cited}/{mission.criteria.length} cited
        </p>
        {mission.criteria.map((criterion) => {
          const satisfied = isCriterionCited(criterion);
          return (
            <div
              key={criterion.id}
              className="rounded border border-border px-2 py-1.5 text-xs"
              data-testid={`criterion-${criterion.id}`}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="truncate">
                  <span className="font-mono text-[10px] text-text-muted">{criterion.id}</span>{" "}
                  {criterion.statement}
                </span>
                <span
                  className={`shrink-0 font-mono text-[10px] ${
                    satisfied ? "text-success" : "text-warning"
                  }`}
                >
                  {satisfied ? "CITED" : "UNSATISFIED"}
                </span>
              </div>
              {criterion.citations?.map((citation) => (
                <p key={citation.evidence_id} className="mt-0.5 text-[10px] text-text-muted">
                  ← {citation.kind}: {citation.evidence_id}
                  {citation.summary ? ` — ${citation.summary}` : ""}
                </p>
              ))}
              {!satisfied && mission.phase === "VERIFYING" && (
                <div className="mt-1 flex flex-wrap items-center gap-1">
                  <input
                    value={drafts[criterion.id] ?? ""}
                    onChange={(e) =>
                      setDrafts((prev) => ({ ...prev, [criterion.id]: e.target.value }))
                    }
                    placeholder={`Evidence id for ${criterion.id}`}
                    aria-label={`Evidence id for ${criterion.id}`}
                    className="min-w-[160px] flex-1 rounded border border-border bg-background px-2 py-1 font-mono text-[11px] outline-none focus:border-primary"
                  />
                  <select
                    value={kinds[criterion.id] ?? "test"}
                    onChange={(e) =>
                      setKinds((prev) => ({
                        ...prev,
                        [criterion.id]: e.target.value as EvidenceKind,
                      }))
                    }
                    aria-label={`Evidence kind for ${criterion.id}`}
                    className="rounded border border-border bg-background px-1.5 py-1 text-[11px] outline-none focus:border-primary"
                  >
                    {(["test", "ledger", "file", "command", "artifact", "human"] as const).map(
                      (kind) => (
                        <option key={kind} value={kind}>
                          {kind}
                        </option>
                      ),
                    )}
                  </select>
                  <button
                    onClick={() =>
                      onCite(
                        criterion.id,
                        (drafts[criterion.id] ?? "").trim(),
                        kinds[criterion.id] ?? "test",
                      )
                    }
                    disabled={busy || (drafts[criterion.id] ?? "").trim().length === 0}
                    className="rounded border border-border px-2 py-1 text-[11px] hover:border-primary disabled:opacity-40"
                  >
                    Cite {criterion.id}
                  </button>
                </div>
              )}
            </div>
          );
        })}
      </div>

      {mission.steps.length > 0 && (
        <div>
          <p className="text-[10px] uppercase tracking-widest text-text-muted">
            Steps — {doneSteps}/{mission.steps.length} complete
          </p>
          {mission.steps.map((step) => (
            <p key={step.id} className="text-[11px] text-text-muted">
              {step.done ? "✓" : "·"} {step.description}
            </p>
          ))}
        </div>
      )}

      {note && <p className="text-[11px] text-text-secondary">{note}</p>}
      {error && <p className="text-[11px] text-error">{error}</p>}

      <div className="flex flex-wrap items-center gap-2">
        {primaryAction && (
          <button
            onClick={() => onResume(primaryAction.approve)}
            disabled={busy}
            className="rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary-hover disabled:opacity-50"
          >
            {primaryAction.label}
          </button>
        )}
        {!terminal && (
          <div className="flex items-center gap-1">
            <input
              value={cancelReason}
              onChange={(e) => setCancelReason(e.target.value)}
              placeholder="Reason for cancelling"
              aria-label="Cancel reason"
              className="rounded border border-border bg-background px-2 py-1 text-[11px] outline-none focus:border-primary"
            />
            <button
              onClick={() => onCancel(cancelReason.trim())}
              disabled={busy || cancelReason.trim().length === 0}
              className="rounded border border-border px-2 py-1 text-[11px] hover:border-error disabled:opacity-40"
            >
              CANCEL MISSION
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

export function MissionEnginePanel() {
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const res = await fetchEngineStatus();
    setLoading(false);
    if (res.ok) {
      setStatus(res.value);
      setError(null);
    } else {
      setStatus(null);
      setError(res.error);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const run = useCallback(
    async (action: () => Promise<{ ok: boolean; error?: string }>, doneNote?: string) => {
      setBusy(true);
      setNote(null);
      const res = await action();
      if (!res.ok) setError(res.error ?? "Mission Engine request failed");
      else {
        if (doneNote) setNote(doneNote);
        await refresh();
      }
      setBusy(false);
    },
    [refresh],
  );

  const mission = status?.mission ?? null;

  return (
    <Panel title="MISSION ENGINE (PHASE 3B)">
      {loading ? (
        <p className="text-xs text-text-muted">Reading mission state…</p>
      ) : mission === null ? (
        <div className="space-y-2">
          {status?.note && <p className="text-[11px] text-text-muted">{status.note}</p>}
          <CreateForm
            busy={busy}
            onCreate={(objective, criteria, steps) =>
              void run(
                async () => createEngineMission(objective, criteria, steps),
                "Mission created.",
              )
            }
          />
          {error && <p className="text-[11px] text-error">{error}</p>}
        </div>
      ) : (
        <MissionBody
          mission={mission}
          busy={busy}
          note={note}
          error={error}
          onResume={(approve) =>
            void run(async () => {
              const res = await resumeEngineMission(approve);
              if (!res.ok) return res;
              const progress = res.value.progress;
              const message =
                progress.outcome === "awaiting_evidence"
                  ? `Not complete: ${progress.unsatisfied.join(", ")} still need a cited proof.`
                  : progress.outcome === "complete"
                    ? "COMPLETE — every criterion has a cited proof."
                    : progress.outcome === "awaiting_approval"
                      ? "Waiting for approval; nothing has executed."
                      : progress.outcome === "executing"
                        ? `Executing: ${progress.done} done, ${progress.remaining} remaining (saved).`
                        : `Mission is ${progress.phase}.`;
              setNote(message);
              return res;
            })
          }
          onCite={(criterion, evidence, kind) =>
            void run(async () => citeEngineEvidence(criterion, evidence, kind, ""))
          }
          onCancel={(reason) =>
            void run(async () => cancelEngineMission(reason), "Mission cancelled.")
          }
        />
      )}
    </Panel>
  );
}
