import { useState } from "react";
import { Panel } from "./Panel";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import { EVIDENCE_CATEGORIES, NOT_CAPTURED, type MissionRecord } from "../lib/mission";

export function EvidenceCenter({ missions }: { missions: MissionRecord[] }) {
  if (missions.length === 0) {
    return (
      <Panel title="EVIDENCE CENTER">
        <p className="text-sm text-text-muted">
          No missions yet — run a mission to generate verifiable evidence.
        </p>
      </Panel>
    );
  }
  return (
    <div className="space-y-3">
      {missions.map((m) => (
        <Panel key={m.id} title={`EVIDENCE — ${m.title}`}>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 text-xs">
            {EVIDENCE_CATEGORIES.map((cat) => {
              const has =
                (cat === "Tool actions" && m.toolCalls.length > 0) ||
                (cat === "Artifacts" && m.artifacts.length > 0) ||
                (cat === "Verification" && m.verification !== null);
              return (
                <div key={cat} className="rounded border border-border px-2 py-1.5 flex items-center justify-between gap-2">
                  <span className="truncate">{cat}</span>
                  {has ? (
                    <StatusBadge status="AVAILABLE" size="xs" />
                  ) : (
                    <span className="font-mono text-[10px] text-text-muted shrink-0">{NOT_CAPTURED}</span>
                  )}
                </div>
              );
            })}
          </div>
        </Panel>
      ))}
    </div>
  );
}