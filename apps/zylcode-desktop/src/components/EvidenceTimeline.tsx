import { useCallback, useEffect, useState } from "react";
import { Panel } from "./Panel";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import { fetchEvidence, type EvidenceState } from "../lib/surfaces";

/**
 * Evidence timeline: the real ledger. The hash-chain verdict is computed
 * backend-side with the same replay an auditor uses; `empty` and `error`
 * states are honest — no fabricated timeline while no evidence exists.
 */
export function EvidenceTimeline() {
  const [state, setState] = useState<EvidenceState>({ kind: "loading" });

  const load = useCallback(() => {
    setState({ kind: "loading" });
    let cancelled = false;
    fetchEvidence().then((next) => {
      if (!cancelled) setState(next);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const headerStatus: CapabilityStatus =
    state.kind === "ready" ? "AVAILABLE" : state.kind === "loading" ? "LIMITED" : "BLOCKED";

  return (
    <Panel title="EVIDENCE LEDGER">
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <StatusBadge status={headerStatus} size="xs" />
          <button type="button" onClick={load} className="btn btn-secondary text-xs" disabled={state.kind === "loading"}>
            {state.kind === "loading" ? "Reading…" : "Refresh"}
          </button>
        </div>

        {state.kind === "unavailable" && <p className="text-xs text-text-muted">{state.reason}</p>}
        {state.kind === "loading" && <p className="text-xs text-text-muted">Reading the ledger…</p>}

        {state.kind === "empty" && (
          <div className="space-y-1">
            <p className="text-xs text-text-muted">{state.reason}</p>
            <p className="text-[11px] text-text-muted">
              Evidence lands here automatically: every mission step, tool dispatch, and Best-of-N
              candidate outcome is appended to the hash-chained ledger.
            </p>
          </div>
        )}

        {state.kind === "ready" && (
          <>
            <div className="flex flex-wrap items-center gap-2 text-[11px]">
              <span className="font-mono text-text-muted">
                {state.entry_count} entr{state.entry_count === 1 ? "y" : "ies"} ·{" "}
                {state.session_count} session{state.session_count === 1 ? "" : "s"}
              </span>
              {state.chain_intact ? (
                <span className="rounded-full border border-primary/40 bg-primary/10 px-2 py-0.5 font-mono text-[10px] text-primary">
                  ✓ hash chain intact
                </span>
              ) : (
                <span className="rounded-full border border-[#c0392b]/40 bg-[#c0392b]/10 px-2 py-0.5 font-mono text-[10px] text-[#c0392b]">
                  ✗ chain broken — evidence has been tampered with or corrupted
                </span>
              )}
            </div>

            {state.entries.length === 0 ? (
              <p className="text-xs text-text-muted">No entries.</p>
            ) : (
              <ol className="max-h-[60vh] space-y-1 overflow-y-auto">
                {state.entries.map((e) => (
                  <li key={e.id} className="rounded border border-border px-2 py-1.5 text-[11px]">
                    <div className="flex items-center justify-between gap-2">
                      <span className="truncate font-mono">{e.action_id}</span>
                      <span
                        className={`shrink-0 font-mono text-[10px] ${
                          e.state.includes("Failed") || e.error
                            ? "text-[#c0392b]"
                            : "text-primary"
                        }`}
                      >
                        {e.state.replace("\"", "")}
                      </span>
                    </div>
                    <p className="mt-0.5 truncate font-mono text-[10px] text-text-muted">
                      {new Date(e.timestamp).toLocaleString()} · session {e.session_id.slice(0, 8)}
                    </p>
                    {e.error && <p className="mt-0.5 text-[10px] text-[#c0392b]">{e.error}</p>}
                  </li>
                ))}
              </ol>
            )}
          </>
        )}
      </div>
    </Panel>
  );
}
