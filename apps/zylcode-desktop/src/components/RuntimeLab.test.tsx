import { beforeEach, describe, expect, it, vi } from "vitest";

// ---------------------------------------------------------------------------
// RuntimeLab reports which execution targets the current runtime can actually
// reach. The only dynamic part is the desktop flag: DESKTOP and LOGS flip
// between AVAILABLE and LIMITED depending on the shell, so both readings are
// exercised — a target must never be advertised as available when it is not.
// ---------------------------------------------------------------------------

const TARGETS = ["WEB", "DESKTOP", "TERMINAL", "ANDROID", "IOS", "CONTAINER", "REMOTE", "LOGS"];

async function load(desktop: boolean) {
  vi.resetModules();
  vi.doMock("../lib/runtime", async (importOriginal) => {
    const actual = await importOriginal<typeof import("../lib/runtime")>();
    return { ...actual, IS_DESKTOP: desktop };
  });
  const mod = await import("./RuntimeLab");
  const { render, screen } = await import("@testing-library/react");
  render(<mod.RuntimeLab />);
  return screen;
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("RuntimeLab", () => {
  it("lists every execution target exactly once", async () => {
    const screen = await load(false);
    for (const name of TARGETS) {
      expect(screen.getAllByText(name)).toHaveLength(1);
    }
  });

  it("marks commissioned-off targets as not installed", async () => {
    const screen = await load(false);
    const labels = ["NOT INSTALLED", "LIMITED", "AVAILABLE"];
    for (const label of labels) {
      expect(screen.getAllByText(label).length).toBeGreaterThan(0);
    }
    expect(screen.getAllByText("NOT INSTALLED")).toHaveLength(4);
  });

  it("downgrades the desktop and log targets outside the Tauri shell", async () => {
    const screen = await load(false);
    expect(screen.getByText("Requires the Tauri desktop shell")).toBeInTheDocument();
    expect(screen.getByText("Streams require desktop runtime")).toBeInTheDocument();
    // WEB is always available
    expect(screen.getByText("Vite dev/preview of the current project")).toBeInTheDocument();
  });

  it("upgrades the desktop and log targets inside the Tauri shell", async () => {
    const screen = await load(true);
    expect(screen.getByText("Tauri shell active")).toBeInTheDocument();
    expect(screen.getByText("Live streams active")).toBeInTheDocument();
    expect(screen.getAllByText("AVAILABLE")).toHaveLength(3);
    expect(screen.getAllByText("LIMITED")).toHaveLength(1);
    expect(screen.queryByText("Requires the Tauri desktop shell")).not.toBeInTheDocument();
  });
});
