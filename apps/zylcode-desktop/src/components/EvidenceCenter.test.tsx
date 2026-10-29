import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EvidenceCenter } from "./EvidenceCenter";
import {
  EVIDENCE_CATEGORIES,
  NOT_CAPTURED,
  type MissionRecord,
} from "../lib/mission";

// ---------------------------------------------------------------------------
// Evidence centre (components/EvidenceCenter.tsx).
//
// The panel exists to answer "what proof do I actually have?", so the tests
// pin the two honesty behaviours: an absent category reads NOT CAPTURED (it
// never implies evidence exists), and a category flips to AVAILABLE only
// when the underlying record really carries that evidence.
// ---------------------------------------------------------------------------

function mission(overrides: Partial<MissionRecord> = {}): MissionRecord {
  return {
    id: "m1",
    title: "Ship the release",
    goal: "cut v0.2.3",
    state: "COMPLETE",
    autonomy: "GUIDED",
    model: "test-model",
    createdAt: "2026-10-28T10:00:00Z",
    updatedAt: "2026-10-28T10:05:00Z",
    acceptanceCriteria: ["tests pass"],
    deltas: [],
    toolCalls: [],
    artifacts: [],
    verification: null,
    finalOutcome: null,
    ...overrides,
  };
}

describe("EvidenceCenter", () => {
  it("states plainly that no evidence exists yet", () => {
    render(<EvidenceCenter missions={[]} />);
    expect(
      screen.getByText(
        "No missions yet — run a mission to generate verifiable evidence.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("EVIDENCE CENTER")).toBeInTheDocument();
  });

  it("marks every category NOT CAPTURED for a mission with no evidence", () => {
    render(<EvidenceCenter missions={[mission()]} />);
    expect(
      screen.getByText("EVIDENCE — Ship the release"),
    ).toBeInTheDocument();
    // All nine categories are listed, none captured.
    for (const cat of EVIDENCE_CATEGORIES) {
      expect(screen.getByText(cat)).toBeInTheDocument();
    }
    expect(screen.getAllByText(NOT_CAPTURED)).toHaveLength(
      EVIDENCE_CATEGORIES.length,
    );
  });

  it("flips exactly the three supported categories to AVAILABLE", () => {
    render(
      <EvidenceCenter
        missions={[
          mission({
            toolCalls: [{ tool: "apply_patch", status: "complete" }],
            artifacts: [{ kind: "file", label: "a.ts", content: "x" }],
            verification: {
              passed: true,
              rung: 3,
              checks: [],
              duration_ms: 12,
            },
          }),
        ]}
      />,
    );
    expect(screen.getAllByText("AVAILABLE")).toHaveLength(3);
    // The remaining categories keep the honest answer.
    expect(screen.getAllByText(NOT_CAPTURED)).toHaveLength(
      EVIDENCE_CATEGORIES.length - 3,
    );
  });

  it("renders one panel per mission", () => {
    render(
      <EvidenceCenter
        missions={[mission({ id: "m1", title: "First" }), mission({ id: "m2", title: "Second" })]}
      />,
    );
    expect(screen.getByText("EVIDENCE — First")).toBeInTheDocument();
    expect(screen.getByText("EVIDENCE — Second")).toBeInTheDocument();
  });
});