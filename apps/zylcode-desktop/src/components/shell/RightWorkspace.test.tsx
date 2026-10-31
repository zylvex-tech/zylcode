import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RightWorkspace } from "./RightWorkspace";

// ---------------------------------------------------------------------------
// Right workspace (components/shell/RightWorkspace.tsx).
//
// Three tabs, each honest about what it is:
//   * Preview  — the real preview target in a sandboxed iframe.
//   * Design   — says outright that no visual builder is commissioned.
//   * Responsive — real width presets that actually resize the iframe.
// The collapse control must hide the preview without unmounting the agent
// workspace below it, and an embedded preview must not render a second iframe.
// ---------------------------------------------------------------------------

afterEach(() => {
  window.history.replaceState({}, "", "/");
});

describe("RightWorkspace", () => {
  it("starts on the preview tab with a sandboxed iframe", () => {
    render(<RightWorkspace />);

    const iframe = screen.getByTitle("Project preview");
    expect(iframe).toBeInTheDocument();
    expect(iframe).toHaveAttribute("sandbox", "allow-scripts allow-forms allow-same-origin");
    expect(screen.getByLabelText("Collapse preview")).toBeInTheDocument();
  });

  it("says the design canvas is not commissioned instead of faking one", () => {
    render(<RightWorkspace />);

    fireEvent.click(screen.getByRole("button", { name: "Design" }));

    expect(
      screen.getByText(/The visual design canvas is not commissioned in this build/),
    ).toBeInTheDocument();
    expect(screen.queryByTitle("Project preview")).not.toBeInTheDocument();
  });

  it("returns the preview when switching back off the design tab", () => {
    render(<RightWorkspace />);

    fireEvent.click(screen.getByRole("button", { name: "Design" }));
    expect(screen.queryByTitle("Project preview")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    expect(screen.getByTitle("Project preview")).toBeInTheDocument();
    // Switching away from design re-opens the preview even if it was collapsed.
    expect(screen.getByLabelText("Collapse preview")).toBeInTheDocument();
  });

  it("collapses and re-expands the preview on the disclosure control", () => {
    render(<RightWorkspace />);

    fireEvent.click(screen.getByLabelText("Collapse preview"));
    expect(screen.queryByTitle("Project preview")).not.toBeInTheDocument();

    fireEvent.click(screen.getByLabelText("Expand preview"));
    expect(screen.getByTitle("Project preview")).toBeInTheDocument();
  });

  it("offers the responsive width presets only on the responsive tab", () => {
    render(<RightWorkspace />);

    expect(screen.queryByRole("button", { name: "375" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Responsive" }));

    for (const preset of ["100%", "1280", "768", "375"]) {
      expect(screen.getByRole("button", { name: preset })).toBeInTheDocument();
    }
    expect(screen.getByText(/Responsive · 100%/)).toBeInTheDocument();
  });

  it("resizes the iframe to the selected preset and labels it", () => {
    render(<RightWorkspace />);

    fireEvent.click(screen.getByRole("button", { name: "Responsive" }));
    fireEvent.click(screen.getByRole("button", { name: "375" }));

    expect(screen.getByTitle("Project preview")).toHaveStyle({ width: "375px" });
    expect(screen.getByText(/Responsive · 375/)).toBeInTheDocument();
  });

  it("falls back to a full-width iframe for the 100% preset", () => {
    render(<RightWorkspace />);

    fireEvent.click(screen.getByRole("button", { name: "Responsive" }));

    expect(screen.getByTitle("Project preview")).toHaveStyle({ width: "100%" });
  });

  it("does not render a second shell when it is itself the embedded preview", () => {
    window.history.replaceState({}, "", "/?embed=1");

    render(<RightWorkspace />);

    expect(screen.queryByTitle("Project preview")).not.toBeInTheDocument();
    expect(screen.getByText("Already inside the embedded preview context.")).toBeInTheDocument();
    // The footer still labels what this pane is, without a second iframe.
    expect(screen.getByText("Workspace mirror")).toBeInTheDocument();
    expect(screen.getAllByText("LIMITED").length).toBeGreaterThan(0);
  });
});
