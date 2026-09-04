import React from "react";

export interface MetricPoint {
  timestamp: string;
  memoryMb: number;
  tokenSavingsRatio: number;
}

interface ResourceGraphProps {
  data: MetricPoint[];
}

export const ResourceGraph: React.FC<ResourceGraphProps> = ({ data }) => {
  const maxMem = Math.max(...data.map((d) => d.memoryMb), 64);
  const graphHeight = 80;
  const graphWidth = 300;

  const points = data
    .map((d, idx) => {
      const x = (idx / (data.length - 1 || 1)) * graphWidth;
      const y = graphHeight - (d.memoryMb / maxMem) * graphHeight;
      return `${x},${y}`;
    })
    .join(" ");

  const latestSavings = data.length > 0 ? data[data.length - 1].tokenSavingsRatio : 0;

  return (
    <div className="bg-slate-900 border border-slate-800 rounded-lg p-4 space-y-3 font-mono">
      <div className="flex justify-between items-center">
        <h3 className="text-xs font-bold text-slate-200 uppercase tracking-wider">
          Runtime Memory & Token Efficiency
        </h3>
        <span className="text-xs text-cyan-400 font-bold">
          {(latestSavings * 100).toFixed(1)}% Token Savings
        </span>
      </div>

      <div className="bg-slate-950 border border-slate-800 p-3 rounded flex flex-col items-center">
        <svg viewBox={`0 0 ${graphWidth} ${graphHeight}`} className="w-full h-20 overflow-visible">
          <polyline
            fill="none"
            stroke="#06b6d4"
            strokeWidth="2"
            points={points}
          />
        </svg>
        <div className="w-full flex justify-between text-[10px] text-slate-500 pt-2 border-t border-slate-800/60 mt-1">
          <span>0 MB</span>
          <span>Peak: {maxMem.toFixed(1)} MB</span>
        </div>
      </div>
    </div>
  );
};