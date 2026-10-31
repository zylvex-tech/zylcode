import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
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
        files={new Map()}
        diffMode={false}
        setDiffMode={vi.fn()}
        saveStatus={{}}
        saveFile={vi.fn()}
        applyPatch={vi.fn()}
        closeTab={vi.fn()}
        deltas={[]}
        onNavigateToFile={vi.fn()}
      />,
    );

    expect(screen.getByRole("heading", { name: "Build from a mission" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Start a mission" }));
    expect(onOpenMissions).toHaveBeenCalledOnce();
  });

  // Regression guard: `intel` was declared as a surface long before this wave
  // but had no entry point, so nothing on it was reachable in the product.
  it("hosts the code-navigation views on the intel surface", () => {
    render(
      <SurfaceHost
        surface="intel"
        activeMission={null}
        missions={[]}
        verification={null}
        busy={false}
        isStreaming={false}
        onMissionStart={vi.fn()}
        onVerifyMission={vi.fn()}
        onOpenProject={vi.fn()}
        onOpenMissions={vi.fn()}
        activeTab={null}
        setActiveTab={vi.fn()}
        files={new Map()}
        diffMode={false}
        setDiffMode={vi.fn()}
        saveStatus={{}}
        saveFile={vi.fn()}
        applyPatch={vi.fn()}
        closeTab={vi.fn()}
        deltas={[]}
        onNavigateToFile={vi.fn()}
      />,
    );

    expect(screen.getByLabelText("Symbol name")).toBeInTheDocument();
    expect(screen.getByLabelText("Repository-relative file")).toBeInTheDocument();
  });
});

// Every declared surface gets rendered at least once: the switch in
// SurfaceHost is the app's whole routing table, so an unrendered case is an
// unreachable view — exactly how `intel` regressed before.

type Surface = ComponentProps<typeof SurfaceHost>["surface"];
type Mission = NonNullable<ComponentProps<typeof SurfaceHost>["missions"]>[number];

function baseProps() {
  return {
    activeMission: null,
    missions: [],
    verification: null,
    busy: false,
    isStreaming: false,
    onMissionStart: vi.fn(),
    onVerifyMission: vi.fn(),
    onOpenProject: vi.fn(),
    onOpenMissions: vi.fn(),
    activeTab: null,
    setActiveTab: vi.fn(),
    files: new Map(),
    diffMode: false,
    setDiffMode: vi.fn(),
    saveStatus: {},
    saveFile: vi.fn(),
    applyPatch: vi.fn(),
    closeTab: vi.fn(),
    deltas: [],
    onNavigateToFile: vi.fn(),
  };
}

describe("SurfaceHost renders every declared surface", () => {
  const shellOnly: Surface[] = [
    "forge",
    "source-control",
    "search",
    "evidence-live",
    "runtime",
    "evidence",
  ];

  it.each(shellOnly)("renders the %s surface without throwing", (surface) => {
    const { container } = render(<SurfaceHost {...baseProps()} surface={surface} />);
    expect(container.firstElementChild).toBeInTheDocument();
    expect((container.textContent ?? "").length).toBeGreaterThan(0);
  });

  it("renders the overview with an empty project and no mission", () => {
    render(<SurfaceHost {...baseProps()} surface="overview" />);

    expect(screen.getByText("Current mission:")).toBeInTheDocument();
    expect(screen.getByText("Current mission:").parentElement).toHaveTextContent("none");
    expect(screen.getByText(/Browser preview \(desktop engine idle\)/)).toBeInTheDocument();
    expect(screen.getByText("Evidence status:").parentElement).not.toHaveTextContent(
      "verification present",
    );
  });

  it("names the active mission and reports captured evidence on overview", () => {
    const mission = {
      id: "m1",
      title: "Ship the release",
      verification: { rung: "R4", checks: [] },
    } as unknown as Mission;

    render(
      <SurfaceHost
        {...baseProps()}
        surface="overview"
        missions={[mission]}
        activeMission={mission}
      />,
    );

    expect(screen.getByText("Current mission:").parentElement).toHaveTextContent(
      "Ship the release",
    );
    expect(screen.getByText("Evidence status:").parentElement).toHaveTextContent(
      "verification present",
    );
  });

  it("offers the composer with no missions yet on the missions surface", () => {
    render(<SurfaceHost {...baseProps()} surface="missions" />);

    expect(screen.getByText("MISSION COMPOSER")).toBeInTheDocument();
    expect(screen.getByText("No missions yet.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Verify active mission" })).toBeInTheDocument();
  });

  it("lists missions and their verification checks when they exist", () => {
    const mission = {
      id: "m1",
      title: "Ship the release",
      state: "DONE",
      artifacts: [],
      verification: null,
    } as unknown as Mission;
    const verification = {
      rung: "R4",
      checks: [{ name: "tests", passed: true, message: "all green" }],
    } as unknown as ComponentProps<typeof SurfaceHost>["verification"];

    render(
      <SurfaceHost
        {...baseProps()}
        surface="missions"
        missions={[mission]}
        verification={verification}
      />,
    );

    expect(screen.getByText("Ship the release")).toBeInTheDocument();
    expect(screen.getByText(/tests: all green/)).toBeInTheDocument();
  });

  it("renders a mission row as a real control, not inert text", () => {
    const mission = {
      id: "active",
      title: "Ship the release",
      state: "DONE",
      artifacts: [],
      verification: null,
    } as unknown as Mission;

    render(<SurfaceHost {...baseProps()} surface="missions" missions={[mission]} />);

    const row = screen.getByRole("button", { name: /Ship the release/ });
    expect(row).toBeInTheDocument();
    expect(row).toHaveTextContent("DONE");
    // The active row is visually distinguished from every other one.
    expect(row.className).toContain("border-primary/50");
    fireEvent.click(row);
    expect(row).toBeInTheDocument();
  });

  it("shows the empty artifacts state when no mission produced any", () => {
    render(<SurfaceHost {...baseProps()} surface="artifacts" />);

    expect(
      screen.getByText(/No artifacts yet — mission artifacts will appear here\./),
    ).toBeInTheDocument();
  });

  it("lists artifacts grouped from every mission", () => {
    const mission = {
      id: "m1",
      title: "t",
      artifacts: [{ label: "report.md", kind: "markdown" }],
    } as unknown as Mission;

    render(<SurfaceHost {...baseProps()} surface="artifacts" missions={[mission]} />);

    expect(screen.getByText("report.md")).toBeInTheDocument();
    expect(screen.getByText("markdown")).toBeInTheDocument();
  });

  it("explains what the code surface actually shows", () => {
    render(<SurfaceHost {...baseProps()} surface="code" />);

    expect(screen.getByText(/Full repository browsing is/)).toBeInTheDocument();
  });

  it("marks design and extensions as coming soon with their own titles", () => {
    const { unmount } = render(<SurfaceHost {...baseProps()} surface="design" />);
    expect(screen.getByText("DESIGN STUDIO")).toBeInTheDocument();
    unmount();

    render(<SurfaceHost {...baseProps()} surface="extensions" />);
    expect(screen.getByText("EXTENSIONS")).toBeInTheDocument();
  });

  it("falls back to a visible message for a surface that does not exist", () => {
    render(<SurfaceHost {...baseProps()} surface={"nope" as unknown as Surface} />);

    expect(screen.getByText(/Surface not implemented: nope/)).toBeInTheDocument();
  });
});
