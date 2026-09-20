import { useState, useRef, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";

export type ActivityId =
  | "explorer"
  | "search"
  | "source-control"
  | "missions"
  | "run"
  | "evidence"
  | "forge"
  | "extensions"
  | "settings"
  | "account";

export interface Activity {
  id: ActivityId;
  label: string;
  icon: React.ReactNode;
  shortcut?: string;
  status?: "available" | "limited" | "coming-soon";
}

const ACTIVITIES: Activity[] = [
  {
    id: "explorer",
    label: "Explorer",
    shortcut: "Ctrl+Shift+E",
    status: "available",
    icon: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
      </svg>
    ),
  },
  {
    id: "search",
    label: "Search",
    shortcut: "Ctrl+Shift+F",
    status: "available",
    icon: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <circle cx="11" cy="11" r="8"></circle>
        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
      </svg>
    ),
  },
  {
    id: "source-control",
    label: "Source Control",
    shortcut: "Ctrl+Shift+G",
    status: "available",
    icon: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <circle cx="18" cy="18" r="3"></circle>
        <circle cx="6" cy="6" r="3"></circle>
        <path d="M13 6h3a2 2 0 0 1 2 2v7"></path>
        <path d="M11 18H8a2 2 0 0 1-2-2V9"></path>
      </svg>
    ),
  },
  {
    id: "missions",
    label: "Missions",
    shortcut: "Ctrl+M",
    status: "limited",
    icon: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <path d="M9 11l3 3L22 4"></path>
        <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"></path>
      </svg>
    ),
  },
  {
    id: "run",
    label: "Run",
    shortcut: "Ctrl+R",
    status: "limited",
    icon: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <polygon points="5 3 19 12 5 21 5 3"></polygon>
      </svg>
    ),
  },
  {
    id: "evidence",
    label: "Evidence",
    shortcut: "Ctrl+E",
    status: "limited",
    icon: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <path d="M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z"></path>
        <path d="M12 6v6l4 2"></path>
      </svg>
    ),
  },
  {
    id: "forge",
    label: "Forge",
    shortcut: "Ctrl+F",
    status: "limited",
    icon: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <path d="M12 2L2 7l10 5 10-5-10-5z"></path>
        <path d="M2 17l10 5 10-5"></path>
        <path d="M2 12l10 5 10-5"></path>
      </svg>
    ),
  },
  {
    id: "extensions",
    label: "Extensions",
    shortcut: "Ctrl+X",
    status: "coming-soon",
    icon: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
        <path d="M8 21h8"></path>
        <path d="M12 17v4"></path>
      </svg>
    ),
  },
];

const BOTTOM_UTILITIES: Activity[] = [
  {
    id: "settings",
    label: "Settings",
    shortcut: "Ctrl+,",
    status: "available",
    icon: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <circle cx="12" cy="12" r="3"></circle>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
      </svg>
    ),
  },
  {
    id: "account",
    label: "Account",
    shortcut: "",
    status: "available",
    icon: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path>
        <circle cx="12" cy="7" r="4"></circle>
      </svg>
    ),
  },
];

interface ActivityRailProps {
  activeActivity: ActivityId;
  onActivityChange: (activity: ActivityId) => void;
  onBottomUtilityClick?: (id: string) => void;
}

export function ActivityRail({
  activeActivity,
  onActivityChange,
  onBottomUtilityClick,
}: ActivityRailProps) {
  const [expanded, setExpanded] = useState<string | null>(null);
  const railRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (railRef.current && !railRef.current.contains(event.target as Node)) {
        setExpanded(null);
      }
    }

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const renderActivityButton = (
    activity: Activity,
    index: number,
    isBottom: boolean
  ) => {
    const isActive = activity.id === activeActivity;
    const isExpanded = expanded === activity.id;

    return (
      <div key={activity.id} className="relative">
        <button
          onClick={() => {
            if (isBottom) {
              onBottomUtilityClick?.(activity.id);
            } else {
              onActivityChange(activity.id);
            }
            setExpanded(null);
          }}
          onMouseEnter={() => !isActive && setExpanded(activity.id)}
          onMouseLeave={() => setExpanded(null)}
          className={`w-10 h-10 flex items-center justify-center rounded-md transition-all duration-200 ${
            isActive
              ? "bg-primary/10 text-primary"
              : "text-text-muted hover:text-text-primary hover:bg-surface"
          }`}
          aria-label={activity.label}
          aria-expanded={isExpanded}
          title=""
        >
          <span className="text-lg">{activity.icon}</span>
        </button>

        <AnimatePresence>
          {isExpanded && !isActive && (
            <motion.div
              initial={{ opacity: 0, x: isBottom ? 10 : -10 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: isBottom ? 10 : -10 }}
              transition={{ duration: 0.15 }}
              className={`absolute ${isBottom ? "right-full" : "left-full"} top-0 ml-2 mr-2 whitespace-nowrap bg-surface border border-border rounded-lg shadow-lg p-2 z-50`}
              style={{ marginTop: `${index * 44}px` }}
            >
              <div className="flex items-center gap-2 px-2 py-1 text-sm font-medium text-text-primary">
                {activity.label}
              </div>
              {activity.shortcut && (
                <div className="flex items-center gap-2 px-2 py-1 text-xs text-text-muted">
                  <kbd className="px-1.5 py-0.5 bg-surface-hover border border-border rounded">{activity.shortcut}</kbd>
                </div>
              )}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    );
  };

  return (
    <nav
      ref={railRef}
      className="w-12 shrink-0 border-r border-border bg-surface/50 flex flex-col"
      onMouseLeave={() => setExpanded(null)}
    >
      <div className="flex flex-col items-center py-3 space-y-1">
        {ACTIVITIES.map((activity, index) =>
          renderActivityButton(activity, index, false)
        )}
      </div>

      <div className="flex-1" />

      <div className="flex flex-col items-center py-3 space-y-1 border-t border-border">
        {BOTTOM_UTILITIES.map((utility, index) =>
          renderActivityButton(utility, index, true)
        )}
      </div>
    </nav>
  );
}