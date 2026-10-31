import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ToolsCataloguePanel } from "./ToolsCataloguePanel";
import { fetchTools, type ToolsState } from "../lib/service";

// ---------------------------------------------------------------------------
// The catalogue polls `fetchTools` every 15s. Stubbing it lets the test drive
// all three states (loading / unavailable / ready) and, in the ready state,
// assert the executor-binding contract: ids without a bound executor are the
// ones that fail closed at dispatch time, so their count must be visible.
// ---------------------------------------------------------------------------

vi.mock("../lib/service", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/service")>();
  return { ...actual, fetchTools: vi.fn() };
});

const tools = vi.mocked(fetchTools);

const READY: ToolsState = {
  kind: "ready",
  source: "mcp.tools.yaml",
  tools: [
    {
      id: "exec.shell",
      transport: "stdio",
      description: "Run a shell command",
      enabled: true,
      executor_bound: true,
    },
    {
      id: "nav.search",
      transport: "stdio",
      description: "Search the repository",
      enabled: true,
      executor_bound: false,
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  tools.mockResolvedValue({ kind: "loading" });
});

describe("ToolsCataloguePanel", () => {
  it("shows the reading state while the first fetch is in flight", () => {
    tools.mockReturnValue(new Promise(() => {}));
    render(<ToolsCataloguePanel />);
    expect(screen.getByText(/Reading the tool catalogue…/)).toBeInTheDocument();
    expect(screen.getByText("LIMITED")).toBeInTheDocument();
  });

  it("lists tools and reports how many executors are bound", async () => {
    tools.mockResolvedValue(READY);
    render(<ToolsCataloguePanel />);

    expect(await screen.findByText("exec.shell")).toBeInTheDocument();
    expect(screen.getByText("nav.search")).toBeInTheDocument();
    expect(screen.getByText("1/2 executors bound")).toBeInTheDocument();
    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();
    expect(screen.getByText(/ids without a bound executor fail closed/)).toBeInTheDocument();
    // Unbound ids carry the warning glyph and the fail-closed title.
    expect(screen.getByText("▲")).toBeInTheDocument();
    expect(screen.getByTitle("No executor bound — fails closed at call time")).toBeInTheDocument();
  });

  it("explains an unavailable catalogue and marks the panel BLOCKED", async () => {
    tools.mockResolvedValue({ kind: "unavailable", reason: "no backend connected" });
    render(<ToolsCataloguePanel />);

    expect(await screen.findByText("no backend connected")).toBeInTheDocument();
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
    expect(screen.queryByText(/executors bound/)).not.toBeInTheDocument();
  });

  it("stops polling on unmount", async () => {
    tools.mockResolvedValue(READY);
    const clearIntervalSpy = vi.spyOn(globalThis, "clearInterval");
    const { unmount } = render(<ToolsCataloguePanel />);
    await waitFor(() => expect(tools).toHaveBeenCalledTimes(1));

    unmount();
    expect(clearIntervalSpy).toHaveBeenCalled();
    clearIntervalSpy.mockRestore();
  });
});
