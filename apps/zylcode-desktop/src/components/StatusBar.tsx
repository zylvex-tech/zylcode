import React from "react";
import { Theme, THEMES } from "../lib/theme";

interface StatusBarProps {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
  mcpBridgeCount: number;
}

export const StatusBar: React.FC<StatusBarProps> = ({ theme, onThemeChange, mcpBridgeCount }) => {
  return (
    <footer className="h-7 bg-slate-950 border-t border-slate-800 text-xs text-slate-400 flex items-center justify-between px-3 select-none">
      <div className="flex items-center gap-4">
        <span className="flex items-center gap-1 text-cyan-400 font-mono">
          <span className="inline-block w-2 h-2 rounded-full bg-cyan-400 animate-pulse"></span>
          memchr: zero-alloc
        </span>
        <span className="text-slate-600">|</span>
        <span className="text-emerald-400">Circuit: CLOSED</span>
        <span className="text-slate-600">|</span>
        <span>Bridges: <strong className="text-slate-200">{mcpBridgeCount}</strong></span>
      </div>

      <div className="flex items-center gap-3">
        <label className="text-slate-400 text-[11px] flex items-center gap-1">
          Theme:
          <select
            value={theme}
            onChange={(e) => onThemeChange(e.target.value as Theme)}
            className="bg-slate-900 border border-slate-700 text-slate-200 rounded px-1.5 py-0.5 text-xs focus:outline-none focus:border-cyan-500"
          >
            {Object.values(THEMES).map((t) => (
              <option key={t.id} value={t.id}>
                {t.name}
              </option>
            ))}
          </select>
        </label>
        <span className="text-slate-600">|</span>
        <span className="font-mono text-slate-500">v0.2.0</span>
      </div>
    </footer>
  );
};