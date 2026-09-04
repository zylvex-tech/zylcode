import React from "react";

export interface BridgeServer {
  id: string;
  name: string;
  transport: "stdio" | "sse";
  status: "connected" | "degraded" | "disconnected";
  latencyMs: number;
  circuitBreaker: "closed" | "open" | "half-open";
}

interface BridgeMonitorProps {
  bridges: BridgeServer[];
}

export const BridgeMonitor: React.FC<BridgeMonitorProps> = ({ bridges }) => {
  return (
    <div className="bg-slate-900 border border-slate-800 rounded-lg p-4 space-y-3">
      <div className="flex justify-between items-center">
        <h3 className="text-xs font-bold text-slate-200 uppercase tracking-wider font-mono">
          MCP Active Bridges
        </h3>
        <span className="text-[11px] text-emerald-400 font-mono">
          {bridges.filter((b) => b.status === "connected").length}/{bridges.length} Online
        </span>
      </div>

      <div className="space-y-2">
        {bridges.map((bridge) => (
          <div
            key={bridge.id}
            className="bg-slate-950 border border-slate-800 rounded p-2.5 flex items-center justify-between text-xs font-mono"
          >
            <div className="flex items-center gap-2">
              <span
                className={`w-2 h-2 rounded-full ${bridge.status === "connected" ? "bg-emerald-400" : bridge.status === "degraded" ? "bg-amber-400" : "bg-rose-500"}`}
              />
              <span className="text-slate-200 font-medium">{bridge.name}</span>
              <span className="text-[10px] text-slate-500 bg-slate-900 px-1 rounded">
                {bridge.transport}
              </span>
            </div>

            <div className="flex items-center gap-3 text-[11px]">
              <span className="text-slate-400">{bridge.latencyMs}ms</span>
              <span
                className={`text-[10px] font-bold uppercase ${bridge.circuitBreaker === "closed" ? "text-emerald-400" : "text-rose-400"}`}
              >
                [{bridge.circuitBreaker}]
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};