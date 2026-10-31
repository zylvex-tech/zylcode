import { render, screen, fireEvent, cleanup, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MenuBar, type Menu } from "./MenuBar";

// ---------------------------------------------------------------------------
// Menu bar (components/shell/MenuBar.tsx).
//
// The contract under test is the one the whole shell leans on:
//
//   * a menu opens on click, switches on hover, and closes on a second click,
//     on Escape, or on a click anywhere outside the bar;
//   * running an item closes the menu; a disabled item never runs and always
//     carries its reason as the tooltip;
//   * `separatorBefore` draws a separator *and* keeps the item — an earlier
//     implementation dropped such items from the menu entirely, so "About
//     ZylCode" was unreachable;
//   * an item with no `run` is inert rather than an error.
// ---------------------------------------------------------------------------

function fixture(): Menu[] {
  return [
    {
      id: "code",
      label: "Code",
      items: [
        { id: "palette", label: "Command Palette…", shortcut: "Ctrl+Shift+P", run: vi.fn() },
        { id: "about", label: "About ZylCode", separatorBefore: true, run: vi.fn() },
        { id: "inert", label: "Not wired up yet" },
      ],
    },
    {
      id: "build",
      label: "Build",
      items: [
        {
          id: "clean",
          label: "Rebuild Workspace",
          disabled: true,
          disabledReason: "No build orchestrator is commissioned yet",
        },
        { id: "verify", label: "Verify Project", checked: true, run: vi.fn() },
      ],
    },
  ];
}

function setup() {
  const menus = fixture();
  render(<MenuBar menus={menus} brand={<span>ZylCode</span>} right={<span>right</span>} />);
  return { menus };
}

function openMenu(label: string): HTMLElement {
  fireEvent.click(screen.getByRole("menuitem", { name: label }));
  return screen.getByRole("menu", { name: label });
}

afterEach(() => {
  cleanup();
});

describe("MenuBar", () => {
  it("renders its brand and right-hand slots alongside the menus", () => {
    render(<MenuBar menus={fixture()} brand={<span>ZylCode</span>} right={<span>right</span>} />);

    expect(screen.getByRole("menubar")).toBeInTheDocument();
    expect(screen.getByText("ZylCode")).toBeInTheDocument();
    expect(screen.getByText("right")).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Code" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Build" })).toBeInTheDocument();
  });

  it("opens a menu and lists its items with their shortcuts", () => {
    setup();

    const menu = openMenu("Code");

    expect(within(menu).getByRole("menuitem", { name: /Command Palette…/ })).toBeInTheDocument();
    expect(within(menu).getByText("Ctrl+Shift+P")).toBeInTheDocument();
  });

  it("runs a command and closes the menu", () => {
    const { menus } = setup();

    const item = within(openMenu("Code")).getByRole("menuitem", { name: /Command Palette…/ });
    fireEvent.click(item);

    expect(menus[0].items[0].run).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("menu", { name: "Code" })).not.toBeInTheDocument();
  });

  it("closes when the same menu button is pressed again", () => {
    setup();

    openMenu("Code");
    fireEvent.click(screen.getByRole("menuitem", { name: "Code" }));

    expect(screen.queryByRole("menu", { name: "Code" })).not.toBeInTheDocument();
  });

  it("switches to another menu on hover while one is open", () => {
    setup();

    openMenu("Code");
    // React derives onMouseEnter from the underlying mouseover event.
    fireEvent.mouseOver(screen.getByRole("menuitem", { name: "Build" }));

    expect(screen.queryByRole("menu", { name: "Code" })).not.toBeInTheDocument();
    expect(screen.getByRole("menu", { name: "Build" })).toBeInTheDocument();
  });

  it("closes on Escape", () => {
    setup();

    openMenu("Code");
    fireEvent.keyDown(document, { key: "Escape" });

    expect(screen.queryByRole("menu", { name: "Code" })).not.toBeInTheDocument();
  });

  it("closes on a click outside the bar", () => {
    setup();

    openMenu("Code");
    fireEvent.mouseDown(document.body);

    expect(screen.queryByRole("menu", { name: "Code" })).not.toBeInTheDocument();
  });

  it("does not run a disabled item and publishes the reason it is disabled", () => {
    setup();

    const item = within(openMenu("Build")).getByRole("menuitem", {
      name: /Rebuild Workspace/,
    });

    expect(item).toBeDisabled();
    expect(item).toHaveAttribute("title", "No build orchestrator is commissioned yet");

    fireEvent.click(item);

    // The guard returns before the menu closes, so the command never ran.
    expect(screen.getByRole("menu", { name: "Build" })).toBeInTheDocument();
  });

  it("keeps a separator item in the menu and draws the separator for it", () => {
    setup();

    const menu = openMenu("Code");

    expect(within(menu).getByRole("separator")).toBeInTheDocument();
    expect(within(menu).getByRole("menuitem", { name: /About ZylCode/ })).toBeInTheDocument();
  });

  it("marks a checked item", () => {
    setup();

    const item = within(openMenu("Build")).getByRole("menuitem", { name: /Verify Project/ });

    expect(item).toHaveTextContent("✓");
  });

  it("treats an unwired item as inert rather than an error", () => {
    setup();

    const item = within(openMenu("Code")).getByRole("menuitem", { name: "Not wired up yet" });
    expect(item).not.toBeDisabled();

    expect(() => fireEvent.click(item)).not.toThrow();
    expect(screen.queryByRole("menu", { name: "Code" })).not.toBeInTheDocument();
  });
});
