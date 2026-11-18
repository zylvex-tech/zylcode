import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as engineModule from "../lib/missionEngine";
import type { EngineMission, EngineStatus } from "../lib/missionEngine";
import { MissionEnginePanel } from "./MissionEnginePanel";

// ---------------------------------------------------------------------------
// Mission Engine surface (components/MissionEnginePanel.tsx).
//
// The contract under test is the one this surface exists for: a mission has an
// objective and criteria, citations are only offered during VERIFYING, an
// approval gate is visible rather than automatic, and "not complete" is shown
// as such instead of being rounded up to success.
// ---------------------------------------------------------------------------

vi.mock("../lib/missionEngine", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/missionEngine")>();
  return {
    ...actual,
    fetchEngineStatus: vi.fn(),
    createEngineMission: vi.fn(),
    resumeEngineMission: vi.fn(),
    citeEngineEvidence: vi.fn(),
    cancelEngineMission: vi.fn(),
  };
});

const {
  fetchEngineStatus,
  createEngineMission,
  resumeEngineMission,
  citeEngineEvidence,
  cancelEngineMission,
} = engineModule;
const statusMock = vi.mocked(fetchEngineStatus);
const createMock = vi.mocked(createEngineMission);
const resumeMock = vi.mocked(resumeEngineMission);
const citeMock = vi.mocked(citeEngineEvidence);
const cancelMock = vi.mocked(cancelEngineMission);

function mission(overrides: Partial<EngineMission> = {}): EngineMission {
  return {
    id: "m-12345678",
    objective: "ship the mission engine",
    phase: "AWAITING_APPROVAL",
    criteria: [
      { id: "c1", statement: "tests pass", citations: [] },
      { id: "c2", statement: "docs updated", citations: [] },
    ],
    steps: [{ id: "s1", description: "write code", done: true }],
    seq: 4,
    created_at: "2026-11-15T00:00:00Z",
    updated_at: "2026-11-15T00:00:00Z",
    ...overrides,
  };
}

function status(m: EngineMission | null): EngineStatus {
  return {
    mission: m,
    journalEntries: m ? m.seq : 0,
    discardedPartialTail: false,
    repairedSnapshot: false,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("MissionEnginePanel", () => {
  it("creates a mission from an objective and one criterion per line", async () => {
    statusMock
      .mockResolvedValueOnce({ ok: true, value: status(null) })
      .mockResolvedValueOnce({ ok: true, value: status(mission({ phase: "CREATED" })) });
    createMock.mockResolvedValue({ ok: true, value: mission({ phase: "CREATED" }) });

    render(<MissionEnginePanel />);

    fireEvent.change(await screen.findByPlaceholderText("What must this mission achieve?"), {
      target: { value: "ship the engine" },
    });
    fireEvent.change(screen.getByPlaceholderText(/tests pass/), {
      target: { value: "tests pass\n  \ndocs updated" },
    });
    fireEvent.click(screen.getByRole("button", { name: "CREATE MISSION" }));

    await waitFor(() =>
      expect(createMock).toHaveBeenCalledWith(
        "ship the engine",
        ["tests pass", "docs updated"],
        [],
      ),
    );
    expect(await screen.findByTestId("mission-phase")).toHaveTextContent("CREATED");
  });

  it("refuses to create a mission without an acceptance criterion", async () => {
    statusMock.mockResolvedValue({ ok: true, value: status(null) });
    render(<MissionEnginePanel />);

    fireEvent.change(await screen.findByPlaceholderText("What must this mission achieve?"), {
      target: { value: "do something vague" },
    });

    expect(screen.getByRole("button", { name: "CREATE MISSION" })).toBeDisabled();
  });

  it("offers citations only during VERIFYING and records the cited evidence", async () => {
    statusMock
      .mockResolvedValueOnce({ ok: true, value: status(mission({ phase: "VERIFYING" })) })
      .mockResolvedValueOnce({
        ok: true,
        value: status(
          mission({
            phase: "VERIFYING",
            criteria: [
              {
                id: "c1",
                statement: "tests pass",
                citations: [
                  {
                    evidence_id: "test:mission_engine",
                    kind: "test",
                    cited_at: "2026-11-15T00:00:00Z",
                  },
                ],
              },
              { id: "c2", statement: "docs updated", citations: [] },
            ],
          }),
        ),
      });
    citeMock.mockResolvedValue({ ok: true, value: mission() });

    render(<MissionEnginePanel />);

    fireEvent.change(await screen.findByLabelText("Evidence id for c1"), {
      target: { value: "test:mission_engine" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Cite c1" }));

    await waitFor(() =>
      expect(citeMock).toHaveBeenCalledWith("c1", "test:mission_engine", "test", ""),
    );
    expect(await screen.findByText(/CITED/)).toBeInTheDocument();
    expect(screen.getByTestId("criterion-c2")).toHaveTextContent("UNSATISFIED");
  });

  it("does not offer a cite control before verification", async () => {
    statusMock.mockResolvedValue({
      ok: true,
      value: status(mission({ phase: "EXECUTING" })),
    });
    render(<MissionEnginePanel />);

    expect(await screen.findByTestId("mission-phase")).toHaveTextContent("EXECUTING");
    expect(screen.queryByRole("button", { name: "Cite c1" })).not.toBeInTheDocument();
  });

  it("reports incomplete verification instead of claiming completion", async () => {
    statusMock.mockResolvedValue({
      ok: true,
      value: status(mission({ phase: "AWAITING_APPROVAL" })),
    });
    resumeMock.mockResolvedValue({
      ok: true,
      value: {
        progress: { outcome: "awaiting_evidence", unsatisfied: ["c1", "c2"] },
        mission: mission({ phase: "VERIFYING" }),
      },
    });

    render(<MissionEnginePanel />);

    fireEvent.click(await screen.findByRole("button", { name: "APPROVE & RUN" }));

    expect(await screen.findByText(/Not complete: c1, c2/)).toBeInTheDocument();
  });

  it("cancels a mission only with a recorded reason", async () => {
    statusMock.mockResolvedValue({
      ok: true,
      value: status(mission({ phase: "EXECUTING" })),
    });
    cancelMock.mockResolvedValue({ ok: true, value: mission({ phase: "CANCELLED" }) });

    render(<MissionEnginePanel />);

    const cancelButton = await screen.findByRole("button", { name: "CANCEL MISSION" });
    expect(cancelButton).toBeDisabled();

    fireEvent.change(screen.getByLabelText("Cancel reason"), {
      target: { value: "owner changed direction" },
    });
    fireEvent.click(cancelButton);

    await waitFor(() => expect(cancelMock).toHaveBeenCalledWith("owner changed direction"));
  });
});
