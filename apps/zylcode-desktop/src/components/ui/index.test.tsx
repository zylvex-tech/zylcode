import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  Avatar,
  Badge,
  Button,
  Card,
  EmptyState,
  Input,
  Modal,
  ProgressBar,
  Select,
  Spinner,
  Tabs,
  ThemeProvider,
  ThemeSelector,
  ToastProvider,
  Tooltip,
  Toggle,
  themes,
  useTheme,
  useToast,
} from "./index";

// ---------------------------------------------------------------------------
// Design-system primitives (components/ui/index.tsx).
//
// What these tests pin down:
//   * the theme is a real, persisted control — choosing one writes both the
//     `data-theme` attribute and localStorage, and it is read back on load;
//   * stateful controls report their state through ARIA (switch/combobox/
//     invalid input), so the accessibility contract is under test, not just
//     the pixels;
//   * Modal owns body scroll while open and restores it on close — a leak
//     there locks scrolling for the whole shell;
//   * every failure path (hook outside its provider) throws a message that
//     names the missing provider, because a silent undefined context is the
//     worst thing to debug in this file.
// ---------------------------------------------------------------------------

afterEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
});

function renderWithTheme(ui: React.ReactElement) {
  return render(<ThemeProvider>{ui}</ThemeProvider>);
}

describe("ThemeProvider / useTheme", () => {
  it("throws a message naming the missing provider", () => {
    function Bare() {
      useTheme();
      return null;
    }
    expect(() => render(<Bare />)).toThrow(
      "useTheme must be used within a ThemeProvider",
    );
  });

  it("defaults to midnight-pro and writes the attribute + storage", async () => {
    renderWithTheme(<span>child</span>);
    await waitFor(() =>
      expect(document.documentElement.getAttribute("data-theme")).toBe(
        "midnight-pro",
      ),
    );
    expect(localStorage.getItem("zylcode-theme")).toBe("midnight-pro");
  });

  it("restores the persisted theme on load", async () => {
    localStorage.setItem("zylcode-theme", "dracula");
    renderWithTheme(<span>child</span>);
    await waitFor(() =>
      expect(document.documentElement.getAttribute("data-theme")).toBe(
        "dracula",
      ),
    );
  });

  it("exposes isDark and the catalogue from the selected theme", async () => {
    let seen: { isDark: boolean; count: number } | null = null;
    function Probe() {
      const { isDark, themes: list } = useTheme();
      seen = { isDark, count: list.length };
      return null;
    }
    renderWithTheme(<Probe />);
    await waitFor(() => expect(seen).not.toBeNull());
    expect(seen!.count).toBe(themes.length);
    // midnight-pro is a dark theme.
    expect(seen!.isDark).toBe(true);
  });
});

describe("ThemeSelector", () => {
  it("lists every theme and applies the chosen one", async () => {
    renderWithTheme(<ThemeSelector />);

    const trigger = screen.getByLabelText("Select theme");
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(trigger);

    expect(await screen.findByText("Select Theme")).toBeInTheDocument();
    for (const theme of themes) {
      expect(screen.getByText(theme.name)).toBeInTheDocument();
    }

    fireEvent.click(screen.getByText("Dracula"));
    expect(localStorage.getItem("zylcode-theme")).toBe("dracula");
    // Selecting closes the dropdown.
    expect(screen.getByLabelText("Select theme")).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("closes when the click lands outside the dropdown", async () => {
    renderWithTheme(<ThemeSelector />);
    fireEvent.click(screen.getByLabelText("Select theme"));
    expect(await screen.findByText("Select Theme")).toBeInTheDocument();

    fireEvent.mouseDown(document.body);
    await waitFor(() =>
      expect(screen.queryByText("Select Theme")).not.toBeInTheDocument(),
    );
  });
});

describe("Button", () => {
  it("applies variant/size classes and forwards click handlers", () => {
    const onClick = vi.fn();
    render(
      <Button variant="danger" size="lg" onClick={onClick}>
        Delete
      </Button>,
    );
    const button = screen.getByRole("button", { name: "Delete" });
    // `danger` is a utility pair, not a `btn-*` token class.
    expect(button.className).toContain("bg-error");
    expect(button.className).toContain("px-6");
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("disables itself while loading and renders the spinner", () => {
    render(
      <Button loading>
        Save
      </Button>,
    );
    const button = screen.getByRole("button", { name: "Save" });
    expect(button).toBeDisabled();
    expect(button.querySelector("svg.animate-spin")).not.toBeNull();
  });

  it("renders a supplied icon when not loading", () => {
    render(
      <Button icon={<span data-testid="icon">+</span>}>Add</Button>,
    );
    expect(screen.getByTestId("icon")).toBeInTheDocument();
    const button = screen.getByText("Add").closest("button") as HTMLElement;
    // Icon path renders a span, not the loading spinner.
    expect(button.querySelector("svg")).toBeNull();
  });
});

describe("Input", () => {
  it("wires the label to the input id", () => {
    render(<Input id="mission-name" label="Mission" />);
    const input = screen.getByLabelText("Mission");
    expect(input).toHaveAttribute("id", "mission-name");
  });

  it("marks itself invalid and surfaces the error as an alert", () => {
    render(<Input id="port" label="Port" error="not a number" />);
    const input = screen.getByLabelText("Port");
    expect(input).toHaveAttribute("aria-invalid", "true");
    expect(input.getAttribute("aria-describedby")).toBe("port-error");
    expect(screen.getByRole("alert")).toHaveTextContent("not a number");
  });

  it("prefers the error over helper text", () => {
    render(<Input id="a" helperText="hint" error="broken" />);
    expect(screen.queryByText("hint")).not.toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("broken");
  });

  it("describes itself with helper text when there is no error", () => {
    render(<Input id="b" helperText="hint" />);
    const input = screen.getByRole("textbox");
    expect(input.getAttribute("aria-describedby")).toBe("b-helper");
    expect(screen.getByText("hint")).toBeInTheDocument();
  });

  it("renders both icons", () => {
    render(
      <Input
        id="c"
        leftIcon={<span data-testid="left" />}
        rightIcon={<span data-testid="right" />}
      />,
    );
    expect(screen.getByTestId("left")).toBeInTheDocument();
    expect(screen.getByTestId("right")).toBeInTheDocument();
  });
});

describe("Card / Badge / Spinner / EmptyState", () => {
  it("applies padding, hover and pass-through classes", () => {
    const { container } = render(
      <Card padding="lg" hoverable className="extra">
        body
      </Card>,
    );
    const card = container.firstChild as HTMLElement;
    expect(card.className).toContain("p-6");
    expect(card.className).toContain("hover:border-border-hover");
    expect(card.className).toContain("extra");
  });

  it("renders every badge variant with its colour token", () => {
    const variants = [
      "primary",
      "secondary",
      "success",
      "warning",
      "error",
      "info",
    ] as const;
    variants.forEach((variant, i) => {
      const { unmount } = render(<Badge variant={variant}>{i}</Badge>);
      expect(screen.getByText(String(i)).className).toContain(
        `text-${variant}`,
      );
      unmount();
    });
  });

  it("sizes the spinner", () => {
    // SVG elements carry an SVGAnimatedString in `className` — read the
    // attribute instead of the property.
    const cls = (c: HTMLElement) =>
      c.firstElementChild?.getAttribute("class") ?? "";
    const sm = render(<Spinner size="sm" />);
    expect(cls(sm.container)).toContain("w-4");
    sm.unmount();
    const md = render(<Spinner />);
    expect(cls(md.container)).toContain("w-6");
    md.unmount();
    const lg = render(<Spinner size="lg" />);
    expect(cls(lg.container)).toContain("w-8");
  });

  it("renders title, description and optional icon/action", () => {
    render(
      <EmptyState
        icon={<span data-testid="icon" />}
        title="Nothing here"
        description="No results yet"
        action={<button>Retry</button>}
      />,
    );
    expect(screen.getByText("Nothing here")).toBeInTheDocument();
    expect(screen.getByText("No results yet")).toBeInTheDocument();
    expect(screen.getByTestId("icon")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Retry" })).toBeInTheDocument();
  });

  it("omits icon and action when not supplied", () => {
    const { container } = render(
      <EmptyState title="Empty" description="none" />,
    );
    expect(container.querySelector("[data-testid]")).toBeNull();
    expect(screen.queryByRole("button")).toBeNull();
  });
});

describe("Tooltip", () => {
  it("shows content on hover and hides it on leave", async () => {
    render(
      <Tooltip content="Runs the mission">
        <button>Run</button>
      </Tooltip>,
    );
    fireEvent.mouseEnter(screen.getByRole("button", { name: "Run" }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent(
      "Runs the mission",
    );
    fireEvent.mouseLeave(screen.getByRole("button", { name: "Run" }));
    await waitFor(() =>
      expect(screen.queryByRole("tooltip")).not.toBeInTheDocument(),
    );
  });

  it("positions per side", async () => {
    render(
      <Tooltip content="tip" side="left">
        <button>x</button>
      </Tooltip>,
    );
    fireEvent.mouseEnter(screen.getByRole("button", { name: "x" }));
    const tip = await screen.findByRole("tooltip");
    expect(tip.className).toContain("right-full");
  });
});

describe("Modal", () => {
  it("renders nothing while closed", () => {
    render(
      <Modal isOpen={false} onClose={vi.fn()} title="Hidden">
        body
      </Modal>,
    );
    expect(screen.queryByText("Hidden")).not.toBeInTheDocument();
  });

  it("locks body scroll while open and restores it when closed", () => {
    const { rerender } = render(
      <Modal isOpen onClose={vi.fn()} title="Release">
        body
      </Modal>,
    );
    expect(document.body.style.overflow).toBe("hidden");
    rerender(
      <Modal isOpen={false} onClose={vi.fn()} title="Release">
        body
      </Modal>,
    );
    expect(document.body.style.overflow).not.toBe("hidden");
  });

  it("closes from the header button, the backdrop, and reports children", () => {
    const onClose = vi.fn();
    const { container } = render(
      <Modal isOpen onClose={onClose} title="Release" size="lg">
        build log
      </Modal>,
    );
    expect(screen.getByText("build log")).toBeInTheDocument();
    const panel = screen.getByRole("heading", { name: "Release" })
      .parentElement!.parentElement as HTMLElement;
    expect(panel.className).toContain("max-w-lg");

    fireEvent.click(screen.getByLabelText("Close modal"));
    expect(onClose).toHaveBeenCalledTimes(1);

    const backdrop = container.querySelector(".backdrop-blur-sm")!;
    fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});

describe("Tabs", () => {
  const tabs = [
    { id: "files", label: "Files", content: <p>Files pane</p> },
    { id: "search", label: "Search", content: <p>Search pane</p> },
  ];

  it("shows the first tab by default and switches on click", () => {
    render(<Tabs tabs={tabs} />);
    expect(screen.getByText("Files pane")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Search" }));
    expect(screen.getByText("Search pane")).toBeInTheDocument();
    expect(screen.queryByText("Files pane")).not.toBeInTheDocument();
  });

  it("honours defaultTab", () => {
    render(<Tabs tabs={tabs} defaultTab="search" />);
    expect(screen.getByText("Search pane")).toBeInTheDocument();
  });
});

describe("ToastProvider / useToast", () => {
  it("throws outside its provider", () => {
    function Bare() {
      useToast();
      return null;
    }
    expect(() => render(<Bare />)).toThrow(
      "useToast must be used within a ToastProvider",
    );
  });

  it("renders added toasts and removes them on dismiss", async () => {
    function Toaster() {
      const { addToast } = useToast();
      return (
        <button onClick={() => addToast("error", "backend refused")}>
          notify
        </button>
      );
    }
    render(
      <ToastProvider>
        <Toaster />
      </ToastProvider>,
    );
    fireEvent.click(screen.getByRole("button", { name: "notify" }));
    expect(await screen.findByText("backend refused")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "" }));
    await waitFor(() =>
      expect(screen.queryByText("backend refused")).not.toBeInTheDocument(),
    );
  });
});

describe("ProgressBar", () => {
  it("clamps the width into 0-100%", () => {
    const { container, rerender } = render(<ProgressBar value={150} />);
    let fill = container.querySelector(".rounded-full.bg-primary")!;
    expect((fill as HTMLElement).style.width).toBe("100%");

    rerender(<ProgressBar value={-20} />);
    fill = container.querySelector(".rounded-full.bg-primary")!;
    expect((fill as HTMLElement).style.width).toBe("0%");

    rerender(<ProgressBar value={40} max={80} variant="success" />);
    fill = container.querySelector(".rounded-full.bg-success")!;
    expect((fill as HTMLElement).style.width).toBe("50%");
  });

  it("shows the value/max label on request", () => {
    render(<ProgressBar value={3} max={9} showLabel />);
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("9")).toBeInTheDocument();
  });
});

describe("Avatar", () => {
  it("renders an image when a source is given", () => {
    render(<Avatar alt="logo" src="/logo.png" />);
    const img = screen.getByRole("img", { name: "logo" });
    expect(img).toHaveAttribute("src", "/logo.png");
  });

  it("falls back to the explicit fallback, then the initial", () => {
    const { unmount } = render(<Avatar alt="zylvex" fallback="ZX" />);
    expect(screen.getByText("ZX")).toBeInTheDocument();
    unmount();
    render(<Avatar alt="zylvex" />);
    expect(screen.getByText("Z")).toBeInTheDocument();
  });
});

describe("Toggle", () => {
  it("reports its state through aria-checked and flips it on click", () => {
    const onChange = vi.fn();
    render(<Toggle checked={false} onChange={onChange} label="Telemetry" />);
    const sw = screen.getByRole("switch");
    expect(sw).toHaveAttribute("aria-checked", "false");
    expect(screen.getByText("Telemetry")).toBeInTheDocument();
    fireEvent.click(sw);
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("cannot be toggled while disabled", () => {
    const onChange = vi.fn();
    render(<Toggle checked onChange={onChange} disabled />);
    const sw = screen.getByRole("switch");
    expect(sw).toBeDisabled();
    fireEvent.click(sw);
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe("Select", () => {
  const options = [
    { value: "stable", label: "Stable" },
    { value: "beta", label: "Beta", disabled: true },
  ];

  it("renders options, placeholder and reports changes", () => {
    const onChange = vi.fn();
    render(
      <Select
        options={options}
        value="stable"
        onChange={onChange}
        label="Channel"
        placeholder="Pick one"
      />,
    );
    // NB: the label element has no `for` attribute — the association gap is
    // asserted as-is rather than papered over; the control itself is queried
    // by role.
    expect(screen.getByText("Channel")).toBeInTheDocument();
    expect(screen.getByText("Pick one")).toBeInTheDocument();
    expect(screen.getByText("Stable")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("combobox"), {
      target: { value: "stable" },
    });
    expect(onChange).toHaveBeenCalledWith("stable");
  });

  it("surfaces an error and honours disabled", () => {
    render(
      <Select
        options={options}
        value=""
        onChange={vi.fn()}
        error="required"
        disabled
      />,
    );
    expect(screen.getByRole("combobox")).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent("required");
  });
});

describe("module-scope theme catalogue", () => {
  it("ships eight themes, each with a preview palette", () => {
    expect(themes).toHaveLength(8);
    for (const theme of themes) {
      expect(theme.preview.background).toMatch(/^#/);
      expect(theme.preview.surface).toMatch(/^#/);
      expect(theme.name.length).toBeGreaterThan(0);
    }
    // No duplicate ids: the selector keys rows by id.
    expect(new Set(themes.map((t) => t.id)).size).toBe(themes.length);
  });
});
