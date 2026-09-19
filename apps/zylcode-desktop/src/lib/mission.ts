// ---------------------------------------------------------------------------
// ZylCode Mission domain (Product Architecture v3 §5–§8)
// ---------------------------------------------------------------------------

export type MissionState =
  | "DRAFT"
  | "PLANNING"
  | "READY"
  | "RUNNING"
  | "WAITING_APPROVAL"
  | "BLOCKED"
  | "VERIFYING"
  | "FAILED"
  | "COMPLETE"
  | "CANCELLED";

export type AutonomyMode =
  | "OBSERVE"
  | "GUIDED"
  | "BUILD"
  | "MISSION"
 | "SOVEREIGN";

export type AutonomyModeInfo = {
  mode: AutonomyMode;
  label: string;
  description: string;
  /** Capability status of the mode itself (never pretends). */
  status: "AVAILABLE" | "LIMITED" | "COMING SOON" | "BLOCKED" | "NOT INSTALLED";
};

/**
 * Autonomy modes with their CURRENT truthful availability. These modes do
 * not override the permission engine — high-risk actions remain governed
 * independently regardless of mode.
 */
export const AUTONOMY_MODES: AutonomyModeInfo[] = [
  {
    mode: "OBSERVE",
    label: "Observe",
    description: "No modification. Watch the agent work.",
    status: "AVAILABLE",
  },
  {
    mode: "GUIDED",
    label: "Guided",
    description: "Ask before consequential changes.",
    status: "AVAILABLE",
  },
  {
    mode: "BUILD",
    label: "Build",
    description: "May edit/build/test inside project boundaries.",
    status: "LIMITED",
  },
  {
    mode: "MISSION",
    label: "Mission",
    description: "Autonomously pursues declared acceptance criteria within permission policy.",
    status: "LIMITED",
  },
  {
    mode: "SOVEREIGN",
    label: "Sovereign",
    description: "Local/private execution where compatible.",
    status: "COMING SOON",
  },
];

export type MissionRecord = {
  id: string;
  title: string;
  goal: string;
  state: MissionState;
  autonomy: AutonomyMode;
  model: string | null;
  createdAt: string;
  updatedAt: string;
  /** Acceptance criteria — may be empty; never fabricated. */
  acceptanceCriteria: string[];
  /** Stream deltas captured during execution (real, from the event layer). */
  deltas: { index: number; delta: string; phase?: string }[];
  /** Real MCP tool calls captured during execution. */
  toolCalls: {
    tool: string;
    status: "calling" | "retry" | "complete" | "failed";
    attempt?: number;
    duration_ms?: number;
    error?: string;
  }[];
  /** Real artifacts returned by the backend. */
  artifacts: { kind: string; label: string; content: string }[];
  /** Real verification report if the mission reached verification. */
  verification: {
    passed: boolean;
    rung: number;
    checks: { name: string; passed: boolean; message: string }[];
    duration_ms: number;
  } | null;
  finalOutcome: string | null;
};

/** Evidence category names used by the Evidence Center (v3 §12). */
export const EVIDENCE_CATEGORIES = [
  "Build",
  "Tests",
  "Runtime",
  "Screenshots",
  "Tool actions",
  "Approvals",
  "Git changes",
  "Artifacts",
  "Verification",
] as const;

export type EvidenceCategory = (typeof EVIDENCE_CATEGORIES)[number];

/** The universal honest answer for absent evidence. */
export const NOT_CAPTURED = "NOT CAPTURED";

export function newMissionId(): string {
  return `mission-${Date.now().toString(36)}-${Math.floor(Math.random() * 1e6).toString(36)}`;
}
