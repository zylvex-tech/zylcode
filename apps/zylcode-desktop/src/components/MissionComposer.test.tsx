import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { MissionComposer } from "./MissionComposer";

// ---------------------------------------------------------------------------
// Mission composer (components/MissionComposer.tsx).
//
// The contract under test: RUN MISSION is the one irreversible action in this
// view, so it stays disabled until there is a real goal, it is disabled while
// the engine is already running, and it never sends a whitespace-only or
// blank model override — `model.trim() || null` keeps "no override" distinct
// from "override to the empty string".
// ---------------------------------------------------------------------------

function renderComposer(overrides: Partial<Parameters<typeof MissionComposer>[0]> = {}) {
  const onMissionStart = vi.fn();
  const props = {
    onMissionStart,
    busy: false,
    streaming: false,
    ...overrides,
  };
  render(<MissionComposer {...props} />);
  return { onMissionStart, props };
}

const GOAL = "Make the release pipeline fail fast";

describe("MissionComposer", () => {
  it("opens on the guided autonomy with its description visible", () => {
    renderComposer();

    expect(screen.getByPlaceholderText(/Describe the mission/)).toBeInTheDocument();
    expect(screen.getByLabelText("Autonomy")).toHaveValue("GUIDED");
    expect(screen.getByText("Ask before consequential changes.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "RUN MISSION" })).toBeDisabled();
  });

  it("keeps RUN MISSION disabled until there is a real goal", () => {
    const { onMissionStart } = renderComposer();
    const textarea = screen.getByPlaceholderText(/Describe the mission/);

    fireEvent.change(textarea, { target: { value: "   " } });
    expect(screen.getByRole("button", { name: "RUN MISSION" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "RUN MISSION" }));
    expect(onMissionStart).not.toHaveBeenCalled();
  });

  it("starts the mission with the goal, autonomy and no model override", () => {
    const { onMissionStart } = renderComposer();

    fireEvent.change(screen.getByPlaceholderText(/Describe the mission/), {
      target: { value: GOAL },
    });
    fireEvent.click(screen.getByRole("button", { name: "RUN MISSION" }));

    expect(onMissionStart).toHaveBeenCalledTimes(1);
    expect(onMissionStart).toHaveBeenCalledWith(GOAL, "GUIDED", null);
  });

  it("trims a model override and passes it through verbatim otherwise", () => {
    const { onMissionStart } = renderComposer();

    fireEvent.change(screen.getByPlaceholderText(/Describe the mission/), {
      target: { value: GOAL },
    });
    fireEvent.change(screen.getByPlaceholderText("provider/model"), {
      target: { value: "  anthropic/claude-sonnet-4.5  " },
    });
    fireEvent.click(screen.getByRole("button", { name: "RUN MISSION" }));

    expect(onMissionStart).toHaveBeenCalledTimes(1);
    expect(onMissionStart).toHaveBeenCalledWith(GOAL, "GUIDED", "anthropic/claude-sonnet-4.5");
  });

  it("stays disabled while the engine is already busy", () => {
    const { onMissionStart } = renderComposer({ busy: true });

    fireEvent.change(screen.getByPlaceholderText(/Describe the mission/), {
      target: { value: GOAL },
    });

    expect(screen.getByRole("button", { name: "RUN MISSION" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "RUN MISSION" }));
    expect(onMissionStart).not.toHaveBeenCalled();
  });

  it("reflects the running state on the button while streaming", () => {
    renderComposer({ streaming: true });

    fireEvent.change(screen.getByPlaceholderText(/Describe the mission/), {
      target: { value: GOAL },
    });

    expect(screen.getByRole("button", { name: /Running…/ })).toBeEnabled();
    expect(screen.queryByRole("button", { name: "RUN MISSION" })).not.toBeInTheDocument();
  });

  it("follows the autonomy selection in both its description and its payload", () => {
    const { onMissionStart } = renderComposer();

    fireEvent.change(screen.getByLabelText("Autonomy"), { target: { value: "OBSERVE" } });

    expect(screen.getByText("No modification. Watch the agent work.")).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText(/Describe the mission/), {
      target: { value: GOAL },
    });
    fireEvent.click(screen.getByRole("button", { name: "RUN MISSION" }));

    expect(onMissionStart).toHaveBeenCalledTimes(1);
    expect(onMissionStart).toHaveBeenCalledWith(GOAL, "OBSERVE", null);
  });

  it("offers every autonomy mode and disables the ones not yet available", () => {
    renderComposer();

    const options = screen.getAllByRole("option");
    const sovereign = options.find((o) => o.textContent?.includes("Sovereign"));

    expect(options.length).toBeGreaterThanOrEqual(5);
    expect(sovereign).toBeDefined();
    expect(sovereign).toBeDisabled();
  });
});
