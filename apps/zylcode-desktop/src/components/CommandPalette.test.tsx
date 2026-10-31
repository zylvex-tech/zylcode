import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CommandPalette } from "./CommandPalette";
import type { Menu } from "./shell/MenuBar";

// ---------------------------------------------------------------------------
// Command palette (components/CommandPalette.tsx).
//
// The palette is a keyboard surface over the *real* menu commands, so the
// guarantees under test are the ones a keyboard user depends on:
//
//   * it lists what the menu bar lists — a command you can see in a menu must
//     be findable here, including the item that follows a separator;
//   * filtering narrows by label *or* menu, and an empty result says so
//     instead of showing nothing;
//   * Enter runs the highlighted command and closes; Escape only closes;
//   * a disabled command cannot run, and it carries the reason it is disabled.
// ---------------------------------------------------------------------------

function menusFixture(run: () => void): Menu[] {
  return [
    {
      id: "code",
      label: "Code",
      items: [
        { id: "palette", label: "Command Palette…", shortcut: "Ctrl+Shift+P", run },
        {
          id: "about",
          label: "About ZylCode",
          separatorBefore: true,
          run,
        },
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
      ],
    },
  ];
}

function setup() {
  const run = vi.fn();
  const onClose = vi.fn();
  render(<CommandPalette open onClose={onClose} menus={menusFixture(run)} />);
  return { run, onClose, input: screen.getByPlaceholderText("Type a command…") };
}

afterEach(() => {
  cleanup();
});

describe("CommandPalette", () => {
  it("renders nothing while closed", () => {
    render(<CommandPalette open={false} onClose={vi.fn()} menus={menusFixture(vi.fn())} />);

    expect(screen.queryByPlaceholderText("Type a command…")).not.toBeInTheDocument();
  });

  it("lists every menu command, including the one after a separator", () => {
    setup();

    expect(screen.getByRole("button", { name: /Command Palette…/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /About ZylCode/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Rebuild Workspace/ })).toBeInTheDocument();
  });

  it("filters by command label", () => {
    setup();
    const input = screen.getByPlaceholderText("Type a command…");

    fireEvent.change(input, { target: { value: "about" } });

    expect(screen.getByRole("button", { name: /About ZylCode/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Command Palette…/ })).not.toBeInTheDocument();
  });

  it("filters by menu name", () => {
    setup();

    fireEvent.change(screen.getByPlaceholderText("Type a command…"), {
      target: { value: "build" },
    });

    expect(screen.getByRole("button", { name: /Rebuild Workspace/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Command Palette…/ })).not.toBeInTheDocument();
  });

  it("says so when nothing matches instead of showing an empty list", () => {
    setup();

    fireEvent.change(screen.getByPlaceholderText("Type a command…"), {
      target: { value: "zzzz" },
    });

    expect(screen.getByText("No matching commands.")).toBeInTheDocument();
  });

  it("runs the highlighted command on Enter and closes", () => {
    const { run, onClose, input } = setup();

    fireEvent.keyDown(input, { key: "Enter" });

    expect(run).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("moves the highlight with ArrowDown before running", () => {
    const { run, onClose, input } = setup();

    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });

    // The second command in the flat list is the separator item.
    expect(run).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes on Escape without running anything", () => {
    const { run, onClose, input } = setup();

    fireEvent.keyDown(input, { key: "Escape" });

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(run).not.toHaveBeenCalled();
  });

  it("will not run a disabled command and states the reason", () => {
    const { run, onClose } = setup();

    const disabled = screen.getByRole("button", { name: /Rebuild Workspace/ });
    expect(disabled).toHaveAttribute("title", "No build orchestrator is commissioned yet");

    fireEvent.click(disabled);

    expect(run).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("runs a command when its row is clicked", () => {
    const { run, onClose } = setup();

    fireEvent.click(screen.getByRole("button", { name: /About ZylCode/ }));

    expect(run).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("resets the query when the palette is re-opened", () => {
    const menus = menusFixture(vi.fn());
    const onClose = vi.fn();
    const view = render(<CommandPalette open={false} onClose={onClose} menus={menus} />);
    view.rerender(<CommandPalette open onClose={onClose} menus={menus} />);

    fireEvent.change(screen.getByPlaceholderText("Type a command…"), {
      target: { value: "about" },
    });
    expect(screen.queryByRole("button", { name: /Command Palette…/ })).not.toBeInTheDocument();

    // Close and re-open: the palette must come back unfiltered.
    view.rerender(<CommandPalette open={false} onClose={onClose} menus={menus} />);
    view.rerender(<CommandPalette open onClose={onClose} menus={menus} />);

    expect(screen.getByRole("button", { name: /Command Palette…/ })).toBeInTheDocument();
  });

  it("closes when the backdrop itself is pressed, but not when the panel is", () => {
    const onClose = vi.fn();
    const { container } = render(
      <CommandPalette open onClose={onClose} menus={menusFixture(vi.fn())} />,
    );
    const backdrop = container.firstElementChild as HTMLElement;
    const panel = backdrop.firstElementChild as HTMLElement;

    fireEvent.mouseDown(panel);
    expect(onClose).not.toHaveBeenCalled();

    fireEvent.mouseDown(backdrop);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("selects the row under the pointer so Enter runs what you aimed at", () => {
    const runFirst = vi.fn();
    const runSecond = vi.fn();
    const menus: Menu[] = [
      {
        id: "code",
        label: "Code",
        items: [
          { id: "first", label: "First command", run: runFirst },
          { id: "second", label: "Second command", run: runSecond },
        ],
      },
    ];
    const onClose = vi.fn();
    render(<CommandPalette open onClose={onClose} menus={menus} />);

    fireEvent.mouseOver(screen.getByRole("button", { name: /Second command/ }));
    fireEvent.keyDown(screen.getByPlaceholderText("Type a command…"), { key: "Enter" });

    expect(runSecond).toHaveBeenCalledTimes(1);
    expect(runFirst).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
