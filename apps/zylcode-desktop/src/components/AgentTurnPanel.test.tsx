import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AgentTurnPanel } from "./AgentTurnPanel";
import type { AgentTurnState, IntentResult } from "../lib/agentTurn";

// ---------------------------------------------------------------------------
// Agent turn surface (components/AgentTurnPanel.tsx).
//
// The rules under test:
//   * nothing runs until a prompt exists; Enter+Ctrl runs it too;
//   * `success: false` is rendered as a failure (✕ failed / BLOCKED), never
//     softened — and unavailable/failed transports show their own reason;
//   * the result panel carries the engine's summary and artifacts verbatim,
//     with the transport it came from;
//   * the badge is LIMITED while running and reflects the verdict after.
// ---------------------------------------------------------------------------

vi.mock("../lib/agentTurn", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/agentTurn")>();
  return { ...actual, runAgentTurn: vi.fn() };
});

const { runAgentTurn } = await import("../lib/agentTurn");
const run = vi.mocked(runAgentTurn);

const result = (over: Partial<IntentResult> = {}): IntentResult => ({
  success: true,
  summary: "Indexed 612 files and cited 3 navigation sources.",
  artifacts: [
    { kind: "report", label: "analysis", content: "scanner.rs owns the walk" },
  ],
  ...over,
});

function type(prompt: string) {
  fireEvent.change(screen.getByLabelText("Agent turn prompt"), {
    target: { value: prompt },
  });
}

beforeEach(() => {
  run.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("AgentTurnPanel", () => {
  it("starts idle: limited badge, honest copy, run disabled", () => {
    run.mockResolvedValue({ kind: "idle" });
    render(<AgentTurnPanel />);
    expect(screen.getByTestId("agent-turn-badge")).toHaveTextContent("LIMITED");
    expect(
      screen.getByText(/the result and the summary are the engine's own/),
    ).toBeInTheDocument();
    expect(screen.getByTestId("agent-turn-run")).toBeDisabled();
    expect(run).not.toHaveBeenCalled();
  });

  it("runs the turn and renders the engine's success verdict verbatim", async () => {
    let settle: (s: AgentTurnState) => void = () => {};
    run.mockReturnValue(
      new Promise((r) => {
        settle = r;
      }),
    );
    render(<AgentTurnPanel />);
    type("analyze this repository structure");
    fireEvent.click(screen.getByTestId("agent-turn-run"));

    // Running state is visible while the promise is still pending.
    expect(
      await screen.findByText("Gathering context, planning, executing tools…"),
    ).toBeInTheDocument();
    settle({ kind: "done", result: result(), via: "service" });
    expect(screen.getByTestId("agent-turn-run")).toHaveTextContent(
      "Running the journey…",
    );
    expect(screen.getByTestId("agent-turn-run")).toBeDisabled();
    expect(run).toHaveBeenCalledWith("analyze this repository structure");

    expect(await screen.findByTestId("agent-turn-verdict")).toBeInTheDocument();
    expect(screen.getByText("✓ success")).toBeInTheDocument();
    expect(screen.getByText("via serve-intel")).toBeInTheDocument();
    expect(
      screen.getByText("Indexed 612 files and cited 3 navigation sources."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("[report] analysis: scanner.rs owns the walk"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("agent-turn-badge")).toHaveTextContent("AVAILABLE");
    expect(screen.getByTestId("agent-turn-timing")).toHaveTextContent(
      /completed in \d+\.\d+s/,
    );
    expect(
      screen.getByText(/never reshaped by this UI/),
    ).toBeInTheDocument();
  });

  it("renders success: false as a failure, not a success", async () => {
    run.mockResolvedValue({
      kind: "done",
      result: result({
        success: false,
        summary: "verification rejected the change",
        artifacts: [],
      }),
      via: "desktop",
    });
    render(<AgentTurnPanel />);
    type("fix the bug");
    fireEvent.click(screen.getByTestId("agent-turn-run"));

    expect(await screen.findByText("✕ failed")).toBeInTheDocument();
    expect(screen.getByText("via desktop engine")).toBeInTheDocument();
    expect(screen.getByTestId("agent-turn-badge")).toHaveTextContent("BLOCKED");
    expect(
      screen.getByText("verification rejected the change"),
    ).toBeInTheDocument();
    // No artifacts → no artifact blocks rendered.
    expect(screen.queryByText(/\[report\]/)).not.toBeInTheDocument();
  });

  it("shows transport unavailability as an alert with the backend's reason", async () => {
    run.mockResolvedValue({
      kind: "unavailable",
      reason: "agent turn requires a runtime",
    });
    render(<AgentTurnPanel />);
    type("anything");
    fireEvent.click(screen.getByTestId("agent-turn-run"));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "agent turn requires a runtime",
    );
    expect(screen.getByTestId("agent-turn-badge")).toHaveTextContent("BLOCKED");
  });

  it("shows engine errors as an alert, not a success", async () => {
    run.mockResolvedValue({ kind: "failed", error: "process_intent exploded" });
    render(<AgentTurnPanel />);
    type("anything");
    fireEvent.click(screen.getByTestId("agent-turn-run"));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "process_intent exploded",
    );
    expect(screen.getByTestId("agent-turn-badge")).toHaveTextContent("BLOCKED");
  });

  it("runs on Ctrl/Cmd+Enter from the textarea", async () => {
    run.mockResolvedValue({ kind: "done", result: result(), via: "service" });
    render(<AgentTurnPanel />);
    type("keyboard driven");
    fireEvent.keyDown(screen.getByLabelText("Agent turn prompt"), {
      key: "Enter",
      ctrlKey: true,
    });
    await waitFor(() => expect(run).toHaveBeenCalledWith("keyboard driven"));
  });

  it("ignores Enter without a modifier and blank prompts", () => {
    run.mockResolvedValue({ kind: "idle" });
    render(<AgentTurnPanel />);
    fireEvent.keyDown(screen.getByLabelText("Agent turn prompt"), {
      key: "Enter",
    });
    expect(run).not.toHaveBeenCalled();

    type("   ");
    fireEvent.click(screen.getByTestId("agent-turn-run"));
    expect(run).not.toHaveBeenCalled();
  });

  it("refuses a second run while one is in flight", async () => {
    let resolveRun: (s: AgentTurnState) => void = () => {};
    run.mockReturnValue(
      new Promise((r) => {
        resolveRun = r;
      }),
    );
    render(<AgentTurnPanel />);
    type("long journey");
    fireEvent.click(screen.getByTestId("agent-turn-run"));
    expect(screen.getByTestId("agent-turn-run")).toBeDisabled();

    fireEvent.click(screen.getByTestId("agent-turn-run"));
    expect(run).toHaveBeenCalledTimes(1);

    resolveRun({ kind: "done", result: result(), via: "service" });
    await screen.findByTestId("agent-turn-verdict");
    // Re-enabled after settling: a second run is allowed again.
    expect(screen.getByTestId("agent-turn-run")).toBeEnabled();
  });
});