import { useEffect, useState } from "react";
import { fetchTools, type ToolsState } from "../lib/service";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";

/**
 * MCP tool catalogue: the committed `mcp.tools.yaml` with executor binding
 * state per tool id. Reachable in the browser via serve-intel and on the
 * desktop via the engine — the same real data in both contexts.
 */
export const ToolsCataloguePanel: React.FC = () => {
  const [state, setState] = useState<ToolsState>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    const load = () => {
      fetchTools().then((next) => {
        if (!cancelled) setState(next);
      });
    };
    load();
    const t = setInterval(load, 15000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, []);

  const status: CapabilityStatus =
    state.kind === "ready" ? "AVAILABLE" : state.kind === "loading" ? "LIMITED" : "BLOCKED";

  return (
    <div className="space-y-2" data-testid="tools-catalogue">
      <div className="flex items-center justify-between">
        <StatusBadge status={status} size="xs" />
        {state.kind === "ready" && (
          <span className="text-[10px] font-mono text-text-muted">
            {state.tools.filter((t) => t.executor_bound).length}/{state.tools.length} executors bound
          </span>
        )}
      </div>

      {state.kind === "unavailable" && (
        <p className="text-xs text-text-muted">{state.reason}</p>
      )}
      {state.kind === "loading" && (
        <p className="text-xs text-text-muted">Reading the tool catalogue…</p>
      )}

      {state.kind === "ready" && (
        <div className="space-y-1">
          {state.tools.map((t) => (
            <div key={t.id} className="flex items-center gap-2 text-xs py-0.5">
              <span
                className={
                  t.executor_bound ? "text-emerald-400" : "text-amber-400"
                }
                title={
                  t.executor_bound
                    ? "A real executor is bound to this id"
                    : "No executor bound — fails closed at call time"
                }
              >
                {t.executor_bound ? "✓" : "▲"}
              </span>
              <span className="font-mono text-text-secondary">{t.id}</span>
              <span className="text-text-muted/70 truncate">{t.description ?? ""}</span>
            </div>
          ))}
          <p className="text-[10px] text-text-muted pt-1">
            Source: {state.source} — ids without a bound executor fail closed and are excluded
            from dispatch.
          </p>
        </div>
      )}
    </div>
  );
};

export default ToolsCataloguePanel;
