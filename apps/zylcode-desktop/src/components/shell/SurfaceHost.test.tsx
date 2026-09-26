import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SurfaceHost } from "./SurfaceHost";

describe("SurfaceHost mission home", () => {
  it("gives a new user a clear path to start a mission", () => {
    const onOpenMissions = vi.fn();

    render(
      <SurfaceHost
        surface="home"
        activeMission={null}
        missions={[]}
        verification={null}
        busy={false}
        isStreaming={false}
        onMissionStart={vi.fn()}
        onVerifyMission={vi.fn()}
        onOpenProject={vi.fn()}
        onOpenMissions={onOpenMissions}
        activeTab={null}
        setActiveTab={vi.fn()}
        files={[]}
        diffMode={false}
        setDiffMode={vi.fn()}
        saveStatus="idle"
        saveFile={vi.fn()}
        applyPatch={vi.fn()}
        closeTab={vi.fn()}
        deltas={[]}
      />,
    );

    expect(screen.getByRole("heading", { name: "Build from a mission" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Start a mission" }));
    expect(onOpenMissions).toHaveBeenCalledOnce();
  });
});
