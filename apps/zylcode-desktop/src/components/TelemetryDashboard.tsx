import React from "react";
import { BridgeMonitor, BridgeServer } from "./BridgeMonitor";
import { ResourceGraph, MetricPoint } from "./ResourceGraph";

interface TelemetryDashboardProps {
  bridges: BridgeServer[];
  metrics: MetricPoint[];
  onOpenAuditLogs: () => void;
}

export const TelemetryDashboard: React.FC<TelemetryDashboardProps> = ({
  bridges,
  metrics,
  onOpenAuditLogs,
}) => {
  return (
    <div className="p-4 space-y-4 max-w-5xl mx-auto">
      <div className="flex justify-between items-center border-b border-slate-800 pb-3">
        <div>
          <h1 className="text-lg font-bold text-slate-100 font-mono">
            System Telemetry
          </h1>
          <p className="text-xs text-slate-400">
            Real-time MCP bridges, memory footprint, and security logs
          </p>
        </div>
        <button
          onClick={onOpenAuditLogs}
          className="px-3 py-1.5 bg-cyan-950 border border-cyan-800 hover:border-cyan-600 text-cyan-300 text-xs font-mono rounded transition-colors"
        >
          View Audit Logs
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <BridgeMonitor bridges={bridges} />
        <ResourceGraph data={metrics} />
      </div>
    </div>
  );
};