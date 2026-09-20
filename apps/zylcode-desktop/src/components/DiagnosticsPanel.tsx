import { useState } from "react";
import { Panel } from "./Panel";
import { SectionTitle } from "./Panel";
import { getDiagnostics, clearDiagnostics } from "../lib/runtime";

export function DiagnosticsPanel() {
  const [, force] = useState(0);
  const entries = getDiagnostics();
  return (
    <Panel
      title="DIAGNOSTICS"
      right={
        <button
          onClick={() => {
            clearDiagnostics();
            force((n) => n + 1);
          }}
          className="text-[11px] text-text-muted hover:text-text-primary underline"
        >
          Clear
        </button>
      }
    >
      {entries.length === 0 ? (
        <p className="text-xs text-text-muted">No diagnostics recorded.</p>
      ) : (
        <div className="space-y-1 max-h-48 overflow-y-auto">
          {entries.map((d, i) => (
            <div key={i} className="font-mono text-[10px] text-text-muted break-all">
              [{d.timestamp}] {d.source}: {d.message}
            </div>
          ))}
        </div>
      )}
    </Panel>
  );
}