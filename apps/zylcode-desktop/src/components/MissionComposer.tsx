import { useState } from "react";
import { Panel } from "./Panel";
import { StatusBadge } from "./CapabilityStatus";
import { AUTONOMY_MODES, type MissionRecord } from "../lib/mission";

export function MissionComposer({
  onMissionStart,
  busy,
  streaming,
}: {
  onMissionStart: (goal: string, autonomy: string, model: string | null) => void;
  busy: boolean;
  streaming: boolean;
}) {
  const [goal, setGoal] = useState("");
  const [autonomy, setAutonomy] = useState("GUIDED");
  const [model, setModel] = useState("");

  const autonomyInfo = AUTONOMY_MODES.find((m) => m.mode === autonomy) ?? AUTONOMY_MODES[1];

  return (
    <div className="space-y-3">
      <textarea
        value={goal}
        onChange={(e) => setGoal(e.target.value)}
        rows={3}
        placeholder="Describe the mission — what should change, and what proves it worked?"
        className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary focus:ring-2 focus:ring-primary/20 transition-all resize-none"
      />

      <div className="flex flex-wrap items-center gap-1.5 text-[11px] text-text-muted">
        <span className="rounded border border-border px-1.5 py-0.5" title="Attach context files — desktop runtime">
          + Context
        </span>
        <span className="rounded border border-border px-1.5 py-0.5" title="Reference project files">
          @ Files
        </span>
        <span className="rounded border border-border px-1.5 py-0.5" title="Recent missions">
          # History
        </span>
        <span className="rounded border border-border px-1.5 py-0.5" title="Commands — limited set available">
          / Commands
        </span>
        <span className="rounded border border-border px-1.5 py-0.5 opacity-60" title="Skills — coming soon">
          $ Skills <StatusBadge status="COMING SOON" size="xs" />
        </span>
        <span className="rounded border border-border px-1.5 py-0.5" title="Capabilities from the registry">
          ⚡ Capabilities
        </span>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
        <label className="text-xs text-text-muted space-y-1 block">
          <span>Autonomy</span>
          <select
            value={autonomy}
            onChange={(e) => setAutonomy(e.target.value)}
            className="w-full rounded-md border border-border bg-background px-2 py-1.5 text-sm outline-none focus:border-primary"
          >
            {AUTONOMY_MODES.map((m) => (
              <option key={m.mode} value={m.mode} disabled={m.status === "COMING SOON"}>
                {m.label} — {m.status}
              </option>
            ))}
          </select>
        </label>
        <label className="text-xs text-text-muted space-y-1 block">
          <span>Model override (optional)</span>
          <input
            value={model}
            onChange={(e) => setModel(e.target.value)}
            placeholder="provider/model"
            className="w-full rounded-md border border-border bg-background px-2 py-1.5 text-sm font-mono outline-none focus:border-primary"
          />
        </label>
      </div>

      <div className="flex items-center justify-between gap-2">
        <p className="text-[11px] text-text-muted">{autonomyInfo.description}</p>
        <button
          onClick={() => onMissionStart(goal, autonomy, model.trim() || null)}
          disabled={busy || !goal.trim() || autonomyInfo.status === "COMING SOON"}
          className="bg-primary text-primary-foreground px-4 py-2 rounded-md text-sm font-medium hover:bg-primary-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {streaming ? (
            <span className="flex items-center gap-2">
              <svg className="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              Running…
            </span>
          ) : (
            "RUN MISSION"
          )}
        </button>
      </div>
    </div>
  );
}