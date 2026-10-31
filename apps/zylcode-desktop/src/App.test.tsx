import { render, screen, fireEvent, cleanup, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

// ---------------------------------------------------------------------------
// App root (App.tsx) — the composition root of the desktop shell.
//
// This file is where the shell's wiring lives: the menu commands, the global
// keyboard shortcuts, the activity->surface mapping and the modal state. None
// of it is reachable by rendering once, so the tests below drive it the way a
// user does — open a menu, run a command, press a shortcut — and assert the
// surface actually changed. A menu item whose `run` is never invoked is dead
// wiring, and that is the defect these tests exist to catch.
//
// The suite runs as TEST, so `safeInvoke` short-circuits to its
// "desktop required" result and no Tauri IPC is ever touched.
// ---------------------------------------------------------------------------

function setSearch(search: string): void {
  window.history.replaceState({}, "", search === "" ? "/" : `/${search}`);
}

/**
 * Open a top-level menu by its label, then run the named item inside it.
 * Item labels are matched as substrings because MenuBar appends a shortcut
 * hint to the accessible name of any item that carries one.
 */
function runMenuCommand(menuLabel: string, itemPattern: string | RegExp): void {
  fireEvent.click(screen.getByRole("menuitem", { name: menuLabel }));
  const menu = screen.getByRole("menu", { name: menuLabel });
  fireEvent.click(within(menu).getByRole("menuitem", { name: itemPattern }));
}

function pressKey(key: string, ctrl = true, shift = false): void {
  fireEvent.keyDown(window, { key, ctrlKey: ctrl, shiftKey: shift });
}

afterEach(() => {
  cleanup();
  setSearch("");
  // `usePersistentState` writes layout choices to localStorage; a test that
  // opens or closes a panel must not decide the next test's starting layout.
  window.localStorage.clear();
  vi.restoreAllMocks();
});

/** The mission home is the default surface: any other surface replaces it. */
function expectLeftHome(): void {
  expect(screen.queryByText("Build from a mission")).not.toBeInTheDocument();
}

/** Ctrl+` toggles the bottom panel, which starts open. */
function closeBottomPanel(): void {
  pressKey("`");
  expect(screen.queryByText("Debug Console")).not.toBeInTheDocument();
}

describe("App root — shell chrome", () => {
  it("renders the desktop shell on its mission home", () => {
    render(<App />);

    expect(screen.getByRole("button", { name: "Go to Mission Home" })).toBeInTheDocument();
    expect(screen.getByText("Build from a mission")).toBeInTheDocument();
  });

  it("renders the menu bar with every top-level menu", () => {
    render(<App />);

    expect(screen.getByRole("menubar")).toBeInTheDocument();
    for (const label of ["Code", "Build", "Run", "Deploy", "AI", "View", "Window", "Help"]) {
      expect(screen.getByRole("menuitem", { name: label })).toBeInTheDocument();
    }
  });

  it("returns to mission home when the brand control is pressed", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Go to Mission Home" }));

    expect(screen.getByText("Build from a mission")).toBeInTheDocument();
  });

  it("renders a theme selector so appearance is user-controllable", () => {
    render(<App />);

    expect(screen.getAllByRole("combobox").length).toBeGreaterThan(0);
  });
});

describe("App root — menu commands drive the surfaces", () => {
  it("runs the AI menu's New Mission command and lands on the missions surface", () => {
    render(<App />);

    runMenuCommand("AI", /New Mission/);

    // The mission surface replaces the home surface.
    expect(screen.queryByText("Build from a mission")).not.toBeInTheDocument();
  });

  it("runs View > Code Navigation and switches away from home", () => {
    render(<App />);

    runMenuCommand("View", "Code Navigation");

    expect(screen.queryByText("Build from a mission")).not.toBeInTheDocument();
  });

  it("runs Help > About and opens the About modal", () => {
    render(<App />);

    runMenuCommand("Help", /^About/);

    expect(screen.getByText("Formally Verified AI Software Synthesis Engine")).toBeInTheDocument();
  });

  it("reports an honest reason on a command that is not commissioned", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("menuitem", { name: "Help" }));
    const menu = screen.getByRole("menu", { name: "Help" });
    const cliRef = within(menu).getByRole("menuitem", { name: /CLI Reference/ });

    expect(cliRef).toBeDisabled();
    expect(cliRef).toHaveAttribute("title", expect.stringContaining("zylcode --help"));
  });

  it("closes the menu after running a command", () => {
    render(<App />);

    runMenuCommand("View", /Source Control/);

    expect(screen.queryByRole("menu", { name: "View" })).not.toBeInTheDocument();
  });
});

describe("App root — every runnable menu command takes effect", () => {
  it("Code > Command Palette… opens the palette", () => {
    render(<App />);

    runMenuCommand("Code", /Command Palette/);

    expect(screen.getByPlaceholderText("Type a command…")).toBeInTheDocument();
  });

  it("Code > Settings… opens the settings dialog", () => {
    render(<App />);

    runMenuCommand("Code", /^Settings/);

    expect(screen.getByRole("dialog", { name: "Settings" })).toBeInTheDocument();
  });

  it("Code > About ZylCode opens the settings dialog", () => {
    render(<App />);

    runMenuCommand("Code", /About ZylCode/);

    expect(screen.getByRole("dialog", { name: "Settings" })).toBeInTheDocument();
  });

  it("Run > Open Run Surface leaves the mission home", () => {
    render(<App />);

    runMenuCommand("Run", /Open Run Surface/);

    expectLeftHome();
  });

  it("Run > Open Terminal re-opens a closed panel on the Terminal tab", () => {
    render(<App />);
    closeBottomPanel();

    runMenuCommand("Run", /Open Terminal/);

    expect(screen.getByLabelText("shell")).toBeInTheDocument();
  });

  it("Run > Show Test Results re-opens a closed panel on the Tests tab", () => {
    render(<App />);
    closeBottomPanel();

    runMenuCommand("Run", "Show Test Results");

    // The panel comes back, and it is no longer on the Terminal tab.
    expect(screen.getByText("Debug Console")).toBeInTheDocument();
    expect(screen.queryByLabelText("shell")).not.toBeInTheDocument();
  });

  it("AI > Mission Queue lands on the missions surface", () => {
    render(<App />);

    runMenuCommand("AI", "Mission Queue");

    expectLeftHome();
  });

  it("AI > Evidence Ledger lands on the evidence surface", () => {
    render(<App />);

    runMenuCommand("AI", "Evidence Ledger");

    expectLeftHome();
  });

  it("AI > Capability Forge lands on the forge surface", () => {
    render(<App />);

    runMenuCommand("AI", "Capability Forge");

    expectLeftHome();
  });

  it("View > Explorer lands on the editor", () => {
    render(<App />);

    runMenuCommand("View", /^Explorer/);

    expectLeftHome();
  });

  it("View > Search lands on the search surface", () => {
    render(<App />);

    runMenuCommand("View", /^Search/);

    expectLeftHome();
  });

  it("View > Toggle Terminal Panel hides the panel", () => {
    render(<App />);
    expect(screen.getByText("Debug Console")).toBeInTheDocument();

    runMenuCommand("View", /Toggle Terminal Panel/);

    expect(screen.queryByText("Debug Console")).not.toBeInTheDocument();
  });

  it("View > Problems replaces the Terminal tab with Problems", () => {
    render(<App />);
    expect(screen.getByLabelText("shell")).toBeInTheDocument();

    runMenuCommand("View", "Problems");

    expect(screen.queryByLabelText("shell")).not.toBeInTheDocument();
  });

  it("View > Output switches the panel off the Terminal tab", () => {
    render(<App />);
    expect(screen.getByLabelText("shell")).toBeInTheDocument();

    runMenuCommand("View", "Output");

    expect(screen.queryByLabelText("shell")).not.toBeInTheDocument();
  });

  it("View > Ports switches the panel to the Ports tab", async () => {
    render(<App />);

    runMenuCommand("View", "Ports");

    expect(await screen.findByText(/zylcode serve-intel/)).toBeInTheDocument();
  });

  it("Window > Editor Focus lands on the editor surface", () => {
    render(<App />);

    runMenuCommand("Window", "Editor Focus");

    expectLeftHome();
  });

  it("Window > Show Preview opens the agent dock and persists the choice", () => {
    window.localStorage.setItem("zylcode.layout.agentDockOpen", "false");
    render(<App />);

    runMenuCommand("Window", "Show Preview");

    expect(JSON.parse(window.localStorage.getItem("zylcode.layout.agentDockOpen") ?? "null")).toBe(
      true,
    );
  });

  it("Window > Theme cycles to the next theme", () => {
    render(<App />);

    const openWindowMenu = () => {
      fireEvent.click(screen.getByRole("menuitem", { name: "Window" }));
      return within(screen.getByRole("menu", { name: "Window" })).getByRole("menuitem", {
        name: /^Theme:/,
      });
    };

    const item = openWindowMenu();
    const before = item.textContent;
    // Running the command closes the menu, so re-open it to read the label.
    fireEvent.click(item);

    const after = openWindowMenu().textContent;
    expect(after).not.toBe(before);
  });
});

describe("App root — global shortcuts", () => {
  it("opens the command palette on Ctrl+Shift+P", () => {
    render(<App />);

    expect(screen.queryByPlaceholderText("Type a command…")).not.toBeInTheDocument();
    pressKey("P", true, true);

    expect(screen.getByPlaceholderText("Type a command…")).toBeInTheDocument();
  });

  it("opens settings on Ctrl+,", () => {
    render(<App />);

    pressKey(",");

    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("toggles the bottom panel on Ctrl+`", () => {
    render(<App />);
    // The panel is open by default; its tab strip is the observable marker.
    expect(screen.queryByText("Debug Console")).toBeInTheDocument();

    pressKey("`");

    expect(screen.queryByText("Debug Console")).not.toBeInTheDocument();

    pressKey("`");

    expect(screen.queryByText("Debug Console")).toBeInTheDocument();
  });
});

describe("App root — activity rail navigation", () => {
  it("moves the centre pane to the editor when Explorer is selected", () => {
    render(<App />);

    expect(screen.getByText("Build from a mission")).toBeInTheDocument();

    // `explorer` maps to the `code` surface, which swaps SurfaceHost for
    // EditorPane — if the mapping broke, this text would still be on screen.
    const explorer = screen.getByRole("button", { name: /explorer/i });
    fireEvent.click(explorer);

    expect(screen.queryByText("Build from a mission")).not.toBeInTheDocument();
  });

  // Every other rail target maps to a surface of its own; landing back on the
  // mission home would mean the mapping silently fell through.
  it.each([
    "Search",
    "Source Control",
    "Code Navigation",
    "Missions",
    "Run",
    "Evidence",
    "Forge",
    "Extensions",
  ])("%s in the activity rail leaves the mission home", (label) => {
    render(<App />);

    // Scope to the rail: the bottom panel also has an "Evidence" tab.
    const rail = within(screen.getByRole("navigation"));
    fireEvent.click(rail.getByRole("button", { name: label }));

    expect(screen.queryByText("Build from a mission")).not.toBeInTheDocument();
  });
});

describe("App root — overview shortcuts and the bottom panel control", () => {
  it("closes the bottom panel from its own control, not only from a shortcut", () => {
    render(<App />);
    expect(screen.getByText("Debug Console")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Close panel" }));

    expect(screen.queryByText("Debug Console")).not.toBeInTheDocument();
  });

  it("routes the rail's settings utility to the overview surface", () => {
    render(<App />);
    expect(screen.getByText("Build from a mission")).toBeInTheDocument();

    fireEvent.click(
      within(screen.getByRole("navigation")).getByRole("button", { name: "Settings" }),
    );

    expect(screen.queryByText("Build from a mission")).not.toBeInTheDocument();
    expect(screen.getByText("Current mission:")).toBeInTheDocument();
  });

  it("follows the mission-home shortcuts into the composer and into the editor", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: /Open mission composer/ }));
    expectLeftHome();
    expect(screen.getByText("MISSION COMPOSER")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Go to Mission Home" }));
    fireEvent.click(screen.getByRole("button", { name: /Open files/ }));
    expectLeftHome();
    expect(screen.queryByRole("button", { name: /Open files/ })).not.toBeInTheDocument();
  });
});

describe("App root — embedded preview mode", () => {
  it("renders content-only when embedded as an iframe", () => {
    setSearch("?embed=1");

    render(<App />);

    // Embedded preview must not draw a second shell around the content.
    expect(screen.queryByRole("button", { name: "Go to Mission Home" })).not.toBeInTheDocument();
    expect(screen.queryByRole("menubar")).not.toBeInTheDocument();
  });
});
