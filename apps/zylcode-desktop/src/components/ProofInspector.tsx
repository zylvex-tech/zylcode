import { Panel } from "./Panel";
import { VerificationRungBadge } from "./VerificationRungBadge";
import { NOT_CAPTURED, type MissionRecord } from "../lib/mission";

export function ProofInspector({ mission }: { mission: MissionRecord | null }) {
  if (!mission) {
    return (
      <Panel title="PROOF INSPECTOR">
        <p className="text-sm text-text-muted">Select or run a mission to inspect its proof chain.</p>
      </Panel>
    );
  }
  const chain: { step: string; value: string }[] = [
    { step: "GOAL", value: mission.goal || NOT_CAPTURED },
    {
      step: "CLAIMS",
      value:
        mission.acceptanceCriteria.length > 0
          ? mission.acceptanceCriteria.join("; ")
          : NOT_CAPTURED,
    },
    {
      step: "ACTIONS",
      value: mission.toolCalls.length > 0 ? `${mission.toolCalls.length} tool call(s)` : NOT_CAPTURED,
    },
    {
      step: "STATE CHANGES",
      value: mission.artifacts.length > 0 ? `${mission.artifacts.length} artifact(s)` : NOT_CAPTURED,
    },
    {
      step: "TESTS",
      value: mission.verification ? `${mission.verification.checks.length} check(s)` : NOT_CAPTURED,
    },
    {
      step: "ARTIFACTS",
      value:
        mission.artifacts.length > 0
          ? mission.artifacts.map((a) => a.label).join(", ")
          : NOT_CAPTURED,
    },
    {
      step: "EVIDENCE",
      value: mission.deltas.length > 0 ? `${mission.deltas.length} stream record(s)` : NOT_CAPTURED,
    },
    { step: "OUTCOME", value: mission.finalOutcome ?? NOT_CAPTURED },
  ];
  return (
    <Panel
      title="PROOF INSPECTOR"
      right={
        <VerificationRungBadge rung={mission.verification ? mission.verification.rung : 0} />
      }
    >
      <div className="space-y-1.5">
        {chain.map((c) => (
          <div key={c.step} className="flex items-start gap-2 text-xs">
            <span className="font-mono text-[10px] text-text-muted w-28 shrink-0">{c.step}</span>
            <span className="break-words min-w-0">{c.value}</span>
          </div>
        ))}
      </div>
    </Panel>
  );
}