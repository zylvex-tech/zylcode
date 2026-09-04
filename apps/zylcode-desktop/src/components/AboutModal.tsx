import React from "react";

interface AboutModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AboutModal: React.FC<AboutModalProps> = ({ isOpen, onClose }) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-800 rounded-lg max-w-md w-full p-6 shadow-2xl text-slate-200">
        <div className="flex justify-between items-start mb-4">
          <div>
            <h2 className="text-xl font-bold text-cyan-400 font-mono">ZylCode Workspace</h2>
            <p className="text-xs text-slate-400">Formally Verified AI Software Synthesis Engine</p>
          </div>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-white text-lg font-bold px-2 py-1"
          >
            ✕
          </button>
        </div>

        <div className="space-y-3 text-xs font-mono bg-slate-950 p-4 rounded border border-slate-800">
          <div className="flex justify-between">
            <span className="text-slate-400">Version:</span>
            <span className="text-slate-200">v0.2.0</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400">Core Runtime:</span>
            <span className="text-emerald-400">Rust (memchr / tokio)</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400">Protocol:</span>
            <span className="text-cyan-400">MCP (Model Context Protocol)</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400">Circuit State:</span>
            <span className="text-emerald-400">CLOSED (Normal operation)</span>
          </div>
        </div>

        <div className="mt-6 flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs rounded font-medium transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};