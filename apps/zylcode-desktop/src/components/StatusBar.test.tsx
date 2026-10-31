import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { StatusBar } from "./StatusBar";
import { themes } from "./ui/theme";

// ---------------------------------------------------------------------------
// Status bar (components/StatusBar.tsx).
//
// It carries three live facts (circuit state, bridge count, version) and the
// global theme control. What matters: the bridge count is rendered verbatim
// from the prop, and the theme <select> is a *controlled* component — its
// value is the `theme` prop and every change reports the new theme id back,
// never a display name (the option's label differs from its value).
// ---------------------------------------------------------------------------

function renderBar(overrides: Partial<Parameters<typeof StatusBar>[0]> = {}) {
  const onThemeChange = vi.fn();
  const props = {
    theme: "midnight-pro" as const,
    onThemeChange,
    mcpBridgeCount: 3,
    ...overrides,
  };
  render(<StatusBar {...props} />);
  return { onThemeChange, props };
}

describe("StatusBar", () => {
  it("reports bridge count, circuit state and version", () => {
    renderBar({ mcpBridgeCount: 5 });
    expect(screen.getByText("Bridges:")).toBeInTheDocument();
    expect(screen.getByText("5")).toBeInTheDocument();
    expect(screen.getByText("Circuit: CLOSED")).toBeInTheDocument();
    expect(screen.getByText("v0.2.3")).toBeInTheDocument();
  });

  it("offers every theme, with the current one selected", () => {
    renderBar({ theme: "dracula" });
    const select = screen.getByRole("combobox");
    expect(select).toHaveValue("dracula");
    expect(screen.getAllByRole("option")).toHaveLength(themes.length);
    // Option value is the id, label is the human name.
    const dracula = screen.getByRole("option", { name: "Dracula" });
    expect(dracula).toHaveAttribute("value", "dracula");
  });

  it("reports a theme change as the theme id", () => {
    const { onThemeChange } = renderBar();
    fireEvent.change(screen.getByRole("combobox"), {
      target: { value: "nord" },
    });
    expect(onThemeChange).toHaveBeenCalledWith("nord");
    expect(onThemeChange).not.toHaveBeenCalledWith("Nord");
  });

  it("renders a zero bridge count rather than hiding the row", () => {
    renderBar({ mcpBridgeCount: 0 });
    expect(screen.getByText("0")).toBeInTheDocument();
  });
});
