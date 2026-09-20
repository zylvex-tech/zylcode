import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { McpInspector, ProviderSettings, DiagnosticsPanel } from "../../components";
import { IS_DESKTOP } from "../../lib/runtime";
import { DESKTOP_REQUIRED_MESSAGE } from "../../lib/runtime";

type BottomTab = "terminal" | "output" | "problems" | "tests" | "evidence" | "dev-tools";

interface BottomPanelProps {
  isOpen: boolean;
  onToggle: () => void;
  activeTab: BottomTab;
  onTabChange: (tab: BottomTab) => void;
}

const BOTTOM_TABS: { id: BottomTab; label: string; icon: React.ReactNode }[] = [
  {
    id: "terminal",
    label: "Terminal",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <polyline points="4 17 10 11 4 5"></polyline>
        <line x1="12" y1="19" x2="20" y2="11"></line>
      </svg>
    ),
  },
  {
    id: "output",
    label: "Output",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <polyline points="22 18 12 12 2 18"></polyline>
        <polyline points="22 6 12 12 2 6"></polyline>
      </svg>
    ),
  },
  {
    id: "problems",
    label: "Problems",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <circle cx="12" cy="12" r="10"></circle>
        <line x1="12" y1="8" x2="12" y2="12"></line>
        <line x1="12" y1="16" x2="12.01" y2="16"></line>
      </svg>
    ),
  },
  {
    id: "tests",
    label: "Tests",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <path d="M9 18V5l12-2v13"></path>
        <circle cx="6" cy="18" r="6"></circle>
        <path d="M22 18h-12v-12"></path>
      </svg>
    ),
  },
  {
    id: "evidence",
    label: "Evidence",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <path d="M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z"></path>
        <path d="M12 6v6l4 2"></path>
      </svg>
    ),
  },
  {
    id: "dev-tools",
    label: "Dev Tools",
    icon: (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
        <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
        <path d="M8 21h8"></path>
        <path d="M12 17v4"></path>
      </svg>
    ),
  },
];

function TerminalPlaceholder() {
  return (
    <div className="flex flex-col h-full bg-code-background text-code-text font-mono text-sm">
      <div className="px-3 py-2 border-b border-border flex items-center gap-2 text-xs text-text-muted">
        <span>zylcode@localhost</span>
        <span className="text-primary">~/projects/zylcode</span>
        <span className="ml-auto">$</span>
      </div>
      <div className="flex-1 p-3 overflow-y-auto">
        <p className="text-text-muted">Terminal not yet implemented.</p>
        <p className="text-text-muted mt-1">The zylcode CLI is available in your shell.</p>
      </div>
    </div>
  );
}

function OutputPlaceholder() {
  return (
    <div className="flex flex-col h-full p-3 overflow-y-auto">
      <p className="text-text-muted">Build output will appear here.</p>
    </div>
  );
}

function ProblemsPlaceholder() {
  return (
    <div className="flex flex-col h-full p-3 overflow-y-auto">
      <p className="text-text-muted">No problems detected.</p>
    </div>
  );
}

function TestsPlaceholder() {
  return (
    <div className="flex flex-col h-full p-3 overflow-y-auto">
      <p className="text-text-muted">Test results will appear here.</p>
    </div>
  );
}

function EvidencePlaceholder() {
  return (
    <div className="flex flex-col h-full p-3 overflow-y-auto">
      <p className="text-text-muted">Mission evidence will appear here.</p>
    </div>
  );
}

function DevToolsPlaceholder() {
  return (
    <div className="flex flex-col h-full p-3 overflow-y-auto space-y-3">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
        <div className="border border-border rounded-lg p-3">
          <h4 className="text-sm font-medium mb-2">MCP Activity</h4>
          {IS_DESKTOP ? <McpInspector /> : <DesktopRequired />}
        </div>
        <div className="border border-border rounded-lg p-3">
          <h4 className="text-sm font-medium mb-2">Providers</h4>
          {IS_DESKTOP ? <ProviderSettings /> : <DesktopRequired />}
        </div>
        <div className="border border-border rounded-lg p-3 lg:col-span-2">
          <h4 className="text-sm font-medium mb-2">Diagnostics</h4>
          <DiagnosticsPanel />
        </div>
      </div>
    </div>
  );
}

function DesktopRequired() {
  return (
    <div className="flex flex-col items-center justify-center gap-2 py-4 text-center text-sm">
      <p className="text-text-muted max-w-xs">{DESKTOP_REQUIRED_MESSAGE}</p>
      <p className="text-[11px] text-text-muted/70">Technical details are available in Diagnostics.</p>
    </div>
  );
}

export function BottomPanel({
  isOpen,
  onToggle,
  activeTab,
  onTabChange,
}: BottomPanelProps) {
  const [height, setHeight] = useState(200);
  const isDragging = false; // Could be implemented for resizable panel

  const renderTabContent = () => {
    switch (activeTab) {
      case "terminal":
        return <TerminalPlaceholder />;
      case "output":
        return <OutputPlaceholder />;
      case "problems":
        return <ProblemsPlaceholder />;
      case "tests":
        return <TestsPlaceholder />;
      case "evidence":
        return <EvidencePlaceholder />;
      case "dev-tools":
        return <DevToolsPlaceholder />;
      default:
        return <TerminalPlaceholder />;
    }
  };

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: height, opacity: 1 }}
          exit={{ height: 0, opacity: 0 }}
          transition={{ duration: 0.2, ease: [0.4, 0, 0.2, 1] }}
          className="fixed bottom-0 left-0 right-0 z-50 border-t border-border bg-surface shadow-2xl flex flex-col"
          style={{ height: `${height}px` }}
        >
          <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-surface/80 backdrop-blur-sm">
            <div className="flex items-center gap-1 overflow-x-auto">
              {BOTTOM_TABS.map((tab) => (
                <button
                  key={tab.id}
                  onClick={() => onTabChange(tab.id)}
                  className={`px-3 py-1.5 rounded-md text-sm whitespace-nowrap transition-colors flex items-center gap-1.5 ${
                    activeTab === tab.id
                      ? "bg-primary/10 text-primary font-medium"
                      : "text-text-muted hover:text-text-primary hover:bg-surface"
                  }`}
                >
                  <span>{tab.icon}</span>
                  <span>{tab.label}</span>
                </button>
              ))}
            </div>
            <button
              onClick={onToggle}
              className="p-1 text-text-muted hover:text-text-primary transition-colors"
              aria-label="Close panel"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </button>
          </div>

          <div className="flex-1 min-h-0 overflow-hidden">
            {renderTabContent()}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}