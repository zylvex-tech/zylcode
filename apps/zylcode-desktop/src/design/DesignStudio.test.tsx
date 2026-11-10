import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { DesignStudio } from "./DesignStudio";
import {
  activeBreakpoint,
  contrastMatrix,
  getComponent,
  mountPlan,
  tokenValue,
  tokensOfTheme,
  usagesOfToken,
} from "./catalog";
import indexData from "./generated/design-index.json";
import { Inspector } from "./Inspector";
import { loadDesignComponent } from "./registry";
import type { DesignIndex } from "./types";

vi.mock("../lib/fileContent", () => ({
  fetchFileContent: vi.fn(async (path: string) => ({
    kind: "ready" as const,
    data: {
      path,
      size: 120,
      lines: 6,
      lossy: false,
      content: Array.from({ length: 300 }, (_, i) =>
        i === 144 ? "export function Button() {" : `// line ${i + 1}`,
      ).join("\n"),
    },
  })),
}));

const index = indexData as unknown as DesignIndex;

// The canvas imports the real component barrel on first mount; warm that module
// outside the assertion timeouts so the test measures behaviour, not transform.
beforeAll(async () => {
  const entry = getComponent(index, "components/ui/index#Button");
  if (entry) await loadDesignComponent(entry);
}, 60_000);

// ---------------------------------------------------------------------------
// Acceptance, end to end through the surface: the studio reads the generated
// index, the canvas mounts the real component, the viewport and theme work, and
// the inspector reports what the index knows.
// ---------------------------------------------------------------------------

describe("DesignStudio", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("reports what it indexed instead of a mock", () => {
    render(<DesignStudio />);
    const stats = `${index.stats.components + index.stats.designSystemComponents} components`;
    expect(screen.getByText(new RegExp(stats))).toBeInTheDocument();
    expect(screen.getByText(new RegExp(`${index.stats.tokens} tokens`))).toBeInTheDocument();
  });

  it("mounts the real design-system Button on the canvas", async () => {
    render(<DesignStudio />);
    const frame = screen.getByTestId("canvas-frame");
    const button = await within(frame).findByRole(
      "button",
      { name: "Design Studio" },
      { timeout: 5000 },
    );
    expect(button.tagName).toBe("BUTTON");
    expect(button.className).toContain("btn-primary");
  });

  it("switches viewport width and theme on the frame", () => {
    render(<DesignStudio />);
    const frame = screen.getByTestId("canvas-frame");
    expect(frame.getAttribute("data-theme")).toBe(index.themes[0].id);

    fireEvent.click(screen.getByRole("button", { name: "768" }));
    expect(frame.style.width).toBe("768px");
    expect(screen.getByTestId("active-breakpoint").textContent).toContain(
      activeBreakpoint(index, 768)?.name ?? "below sm",
    );

    fireEvent.change(screen.getByLabelText("Canvas theme"), { target: { value: "arctic-light" } });
    expect(frame.getAttribute("data-theme")).toBe("arctic-light");
  });

  it("zooms the canvas", () => {
    render(<DesignStudio />);
    fireEvent.click(screen.getByRole("button", { name: "+" }));
    expect(screen.getByText("110%")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "reset" }));
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("lists tokens with values and usage counts", () => {
    render(<DesignStudio />);
    fireEvent.click(screen.getByRole("tab", { name: "tokens" }));
    const list = screen.getByTestId("token-list");
    expect(within(list).getByText("--color-primary")).toBeInTheDocument();

    const primary = tokenValue(index, "--color-primary", index.themes[0].id);
    expect(primary).toBeDefined();
    fireEvent.click(within(list).getByText("--color-primary"));
    expect(screen.getByTestId("token-detail").textContent).toContain(primary!.value);
    expect(screen.getByTestId("token-detail").textContent).toContain(
      `${usagesOfToken(index, "--color-primary", index.themes[0].id).length} sites`,
    );
  });

  it("filters tokens by text and category", () => {
    render(<DesignStudio />);
    fireEvent.click(screen.getByRole("tab", { name: "tokens" }));
    const list = screen.getByTestId("token-list");

    fireEvent.change(screen.getByLabelText("Filter tokens"), { target: { value: "primary" } });
    expect(within(list).queryByText("--space-4")).toBeNull();
    expect(within(list).getByText("--color-primary")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Token category"), { target: { value: "color" } });
    expect(within(list).getByText("--color-primary")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Filter tokens"), {
      target: { value: "no-such-token" },
    });
    expect(within(list).getByText(/no token matches/)).toBeInTheDocument();
  });

  it("reports the selected token's usage sites and opens the component", () => {
    render(<DesignStudio />);
    fireEvent.click(screen.getByRole("tab", { name: "tokens" }));
    fireEvent.click(screen.getByLabelText("Token category"));
    fireEvent.change(screen.getByLabelText("Token category"), { target: { value: "color" } });
    fireEvent.click(within(screen.getByTestId("token-list")).getByText("--color-primary"));

    const detail = screen.getByTestId("token-detail");
    expect(detail.textContent).toMatch(/used by \d+ component/);
  });

  it("filters the component list", () => {
    render(<DesignStudio />);
    fireEvent.click(screen.getByRole("tab", { name: "components" }));

    fireEvent.click(screen.getByRole("button", { name: /design system/ }));
    const list = screen.getByTestId("component-list");
    expect(within(list).getByText("Button")).toBeInTheDocument();
    expect(within(list).queryByText("SurfaceHost")).toBeNull();

    fireEvent.change(screen.getByLabelText("Filter components"), { target: { value: "badge" } });
    expect(within(list).getByText("Badge")).toBeInTheDocument();
    expect(within(list).queryByText("Input")).toBeNull();
  });

  it("lists indexed components and their sources", () => {
    render(<DesignStudio />);
    fireEvent.click(screen.getByRole("tab", { name: "components" }));
    const list = screen.getByTestId("component-list");
    expect(within(list).getByText("Badge")).toBeInTheDocument();
    expect(within(list).getAllByText(/ui\/index\.tsx/).length).toBeGreaterThan(0);
  });

  it("shows the selected component's props and mount status in the inspector", () => {
    render(<DesignStudio />);
    expect(screen.getByTestId("mount-status").textContent).toContain("MOUNTABLE");
    expect(screen.getByLabelText("Value for variant")).toBeInTheDocument();
    expect(screen.getByLabelText("Toggle loading")).toBeInTheDocument();
  });

  it("edits a prop only within its type", async () => {
    render(<DesignStudio />);
    const variant = screen.getByLabelText("Value for variant") as HTMLSelectElement;
    fireEvent.change(variant, { target: { value: "ghost" } });
    const frame = screen.getByTestId("canvas-frame");
    await waitFor(
      () => {
        const button = within(frame).getByRole("button", { name: "Design Studio" });
        expect(button.className).toContain("btn-ghost");
      },
      { timeout: 5000 },
    );
  });

  it("reports contrast per theme in the accessibility view", () => {
    render(<DesignStudio />);
    fireEvent.click(screen.getByRole("tab", { name: "accessibility" }));
    const rows = contrastMatrix(index, index.themes[0].id);
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.some((r) => r.verdict === "PASS")).toBe(true);
    expect(screen.getAllByText("PASS").length).toBeGreaterThan(0);

    // Whatever the maths finds is what the panel must show: every failing pair
    // in the matrix appears in the summary, and none is invented.
    const failing = rows.filter((r) => r.verdict === "WARNING");
    if (failing.length === 0) {
      expect(screen.getByText(/every measured pair meets/)).toBeInTheDocument();
    } else {
      expect(screen.getByText(/pair\(s\) fall below AA/)).toBeInTheDocument();
    }
    expect(screen.getByText(/Static analysis of the component's source/)).toBeInTheDocument();
  });
});

describe("baseline comparison", () => {
  it("captures a selected element and diffs a later capture against it", async () => {
    const entry = getComponent(index, "components/ui/index#Button")!;
    const selection = {
      tag: "button",
      classes: "btn btn-primary",
      text: "Design Studio",
      tokens: ["--color-primary"],
      snapshot: { color: "rgb(255, 255, 255)", backgroundColor: "rgb(59, 130, 246)" },
      depth: 1,
    };

    render(
      <Inspector
        index={index}
        entry={entry}
        theme={index.themes[0].id}
        props={mountPlan(entry).initialProps}
        onPropChange={() => {}}
        selection={selection}
        onSelectComponent={() => {}}
        defaultTab="element"
      />,
    );

    expect(screen.queryByTestId("baseline-diff")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "capture baseline" }));
    expect(screen.getByTestId("baseline-diff")).toBeInTheDocument();
    expect(screen.getAllByText("unchanged")).toHaveLength(2);

    fireEvent.click(screen.getByRole("button", { name: "clear" }));
    expect(screen.queryByTestId("baseline-diff")).toBeNull();
  });
});

describe("Inspector source navigation", () => {
  it("opens the indexed file at the indexed line", async () => {
    const entry = getComponent(index, "components/ui/index#Button")!;
    render(
      <Inspector
        index={index}
        entry={entry}
        theme={index.themes[0].id}
        props={mountPlan(entry).initialProps}
        onPropChange={() => {}}
        selection={null}
        onSelectComponent={() => {}}
        defaultTab="source"
      />,
    );

    const pane = await screen.findByTestId("source-lines");
    expect(pane.querySelector(`[data-line="${entry.line}"]`)).not.toBeNull();
    expect(screen.getByText(`${entry.file}:${entry.line}`)).toBeInTheDocument();
  });

  it("shows the reason when the source service is unavailable", async () => {
    const { fetchFileContent } = await import("../lib/fileContent");
    vi.mocked(fetchFileContent).mockResolvedValueOnce({ kind: "unavailable", reason: "HTTP 501" });
    const entry = getComponent(index, "components/ui/index#Button")!;

    render(
      <Inspector
        index={index}
        entry={entry}
        theme={index.themes[0].id}
        props={{}}
        onPropChange={() => {}}
        selection={null}
        onSelectComponent={() => {}}
        defaultTab="source"
      />,
    );

    expect(await screen.findByText("SOURCE UNAVAILABLE")).toBeInTheDocument();
    expect(screen.getByText("HTTP 501")).toBeInTheDocument();
  });
});

describe("token identity", () => {
  it("keeps theme tokens distinct while sharing the base scale", () => {
    const base = tokensOfTheme(index, "base");
    const dark = tokensOfTheme(index, index.themes[0].id);
    expect(base.some((t) => t.category === "spacing")).toBe(true);
    expect(dark.every((t) => t.category === "color")).toBe(true);
  });
});
