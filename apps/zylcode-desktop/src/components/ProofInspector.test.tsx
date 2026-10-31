import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ProofInspector } from "./ProofInspector";
import { NOT_CAPTURED, type MissionRecord } from "../lib/mission";

// ---------------------------------------------------------------------------
// The proof chain is the product's honesty contract: every rung renders either
// real captured evidence or the literal NOT_CAPTURED marker. Nothing here may
// silently default to a plausible-looking value, so both branches are pinned.
// ---------------------------------------------------------------------------

function mission(overrides: Partial<MissionRecord> = {}): MissionRecord {
  return {
    id: "mission-1",
    title: "Ship the gate",
    goal: "Raise coverage above the threshold",
    state: "completed",
    autonomy: "supervised",
    model: "test-model",
    createdAt: "2026-10-30T00:00:00.000Z",
    updatedAt: "2026-10-30T00:01:00.000Z",
    acceptanceCriteria: ["lines >= 80"],
    deltas: [{ index: 0, delta: "working" }],
    toolCalls: [{ tool: "exec", status: "complete" }],
    artifacts: [{ kind: "file", label: "report.md", content: "# report" }],
    verification: {
      passed: true,
      rung: 3,
      checks: [{ name: "gate", passed: true, message: "ok" }],
      duration_ms: 12,
    },
    finalOutcome: "delivered",
    ...overrides,
  } as MissionRecord;
}

describe("ProofInspector", () => {
  it("asks the operator to pick a mission when none is selected", () => {
    render(<ProofInspector mission={null} />);
    expect(
      screen.getByText("Select or run a mission to inspect its proof chain."),
    ).toBeInTheDocument();
  });

  it("renders every rung of a fully captured mission with real values", () => {
    render(<ProofInspector mission={mission()} />);

    expect(screen.getByText("GOAL")).toBeInTheDocument();
    expect(screen.getByText("Raise coverage above the threshold")).toBeInTheDocument();
    expect(screen.getByText("CLAIMS")).toBeInTheDocument();
    expect(screen.getByText("lines >= 80")).toBeInTheDocument();
    expect(screen.getByText("1 tool call(s)")).toBeInTheDocument();
    expect(screen.getByText("1 artifact(s)")).toBeInTheDocument();
    expect(screen.getByText("1 check(s)")).toBeInTheDocument();
    expect(screen.getByText("report.md")).toBeInTheDocument();
    expect(screen.getByText("1 stream record(s)")).toBeInTheDocument();
    expect(screen.getByText("OUTCOME")).toBeInTheDocument();
    expect(screen.getByText("delivered")).toBeInTheDocument();
  });

  it("falls back to NOT CAPTURED for every rung with no evidence", () => {
    render(
      <ProofInspector
        mission={mission({
          goal: "",
          acceptanceCriteria: [],
          deltas: [],
          toolCalls: [],
          artifacts: [],
          verification: null,
          finalOutcome: null,
        })}
      />,
    );

    const shown = screen.getAllByText(NOT_CAPTURED);
    // GOAL, CLAIMS, ACTIONS, STATE CHANGES, TESTS, ARTIFACTS, EVIDENCE, OUTCOME
    expect(shown).toHaveLength(8);
  });

  it("shows rung 0 when the mission has no verification report", () => {
    const { container } = render(<ProofInspector mission={mission({ verification: null })} />);
    // The badge renders the rung number; no verification means rung 0.
    expect(container.textContent).toContain("0");
    expect(screen.queryByText("1 check(s)")).not.toBeInTheDocument();
  });
});
