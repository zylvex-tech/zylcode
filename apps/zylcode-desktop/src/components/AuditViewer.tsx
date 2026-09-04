import React, { useState } from "react";

export interface AuditEntry {
  id: string;
  timestamp: string;
  toolName: string;
  durationMs: number;
  status: "success" | "error" | "retried";
  payloadHash: string;
}

interface AuditViewerProps {
  entries: AuditEntry[];
  isOpen: boolean;
  onClose: () => void;
}

export const AuditViewer: React.FC<AuditViewerProps> = ({ entries, isOpen, onClose }) => {
  const [filter, setFilter] = useState<string>("");

  if (!isOpen) return null;

  const filteredEntries = entries.filter((e) =>
    e.toolName.toLowerCase().includes(filter.toLowerCase()) ||
    e.payloadHash.toLowerCase().includes(filter.toLowerCase())
  );

  return (
    <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-6">
      <div className="bg-slate-900 border border-slate-800 rounded-lg max-w-3xl w-full h-[550px] flex flex-col shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="px-5 py-4 border-b border-slate-800 flex justify-between items-center bg-slate-950">
          <div>
            <h2 className="text-base font-bold text-cyan-400 font-mono flex items-center gap-2">
              <span className="inline-block w-2 h-2 rounded-full bg-cyan-400 animate-ping"></span>
              Audit Trail & Chain Hashes
            </h2>
            <p className="text-xs text-slate-400">Tamper-evident tool execution telemetry</p>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-white font-mono text-sm px-2">
            ✕
          </button>
        </div>

        {/* Search / Filter */}
        <div className="p-3 bg-slate-900/50 border-b border-slate-800 flex gap-2">
          <input
            type="text"
            placeholder="Filter by tool or SHA-256 hash..."
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-cyan-500 font-mono"
          />
        </div>

        {/* Table Content */}
        <div className="flex-1 overflow-y-auto p-4 space-y-2">
          {filteredEntries.length === 0 ? (
            <p className="text-xs text-slate-500 font-mono text-center py-8">No audit log entries recorded yet.</p>
          ) : (
            filteredEntries.map((entry) => (
              <div
                key={entry.id}
                className="bg-slate-950 border border-slate-800 rounded p-3 text-xs font-mono space-y-1 hover:border-slate-700 transition-colors"
              >
                <div className="flex items-center justify-between">
                  <span className="text-cyan-300 font-bold">{entry.toolName}</span>
                  <div className="flex items-center gap-2">
                    <span
                      className={`px-1.5 py-0.5 rounded text-[10px] uppercase font-bold ${entry.status === "success" ? "bg-emerald-950 text-emerald-400 border border-emerald-800" : entry.status === "retried" ? "bg-amber-950 text-amber-400 border border-amber-800" : "bg-rose-950 text-rose-400 border border-rose-800"}`}
                    >
                      {entry.status}
                    </span>
                    <span className="text-slate-500">{entry.durationMs}ms</span>
                  </div>
                </div>

                <div className="flex items-center justify-between text-[11px] text-slate-400 pt-1">
                  <span className="truncate max-w-[320px] font-mono text-slate-500">
                    Hash: <code className="text-slate-300">{entry.payloadHash}</code>
                  </span>
                  <span className="text-slate-500">{entry.timestamp}</span>
                </div>
              </div>
            ))
          )}
        </div>

        {/* Footer */}
        <div className="px-5 py-3 border-t border-slate-800 bg-slate-950 flex justify-between items-center text-xs text-slate-400">
          <span>Total Recorded: <strong className="text-slate-200">{entries.length}</strong></span>
          <button
            onClick={onClose}
            className="px-3 py-1 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded text-xs font-mono"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};