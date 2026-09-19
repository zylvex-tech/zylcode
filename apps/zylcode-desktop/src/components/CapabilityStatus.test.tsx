import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { StatusBadge, RungBadge, StatusLockedAction } from "./CapabilityStatus";

// ---------------------------------------------------------------------------
// Capability status language (v3 §26): the product must visually distinguish
// AVAILABLE / LIMITED / COMING SOON / BLOCKED / NOT INSTALLED.
// ---------------------------------------------------------------------------

describe("StatusBadge", () => {
  it.each([
    "AVAILABLE",
    "LIMITED",
    "COMING SOON",
    "BLOCKED",
    "NOT INSTALLED",
  ] as const)("renders the %s status text", (status) => {
    render(<StatusBadge status={status} />);
    expect(screen.getByText(status)).toBeInTheDocument();
  });

  it("renders both sizes without crashing", () => {
    const { rerender } = render(<StatusBadge status="AVAILABLE" size="sm" />);
    rerender(<StatusBadge status="AVAILABLE" size="xs" />);
    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();
  });
});

describe("RungBadge", () => {
  it("renders the rung label for developer contexts", () => {
    render(<RungBadge rung="R2" />);
    expect(screen.getByText("R2")).toBeInTheDocument();
  });
});

describe("StatusLockedAction", () => {
  it("renders a disabled control with an explanatory reason", () => {
    render(
      <StatusLockedAction status="NOT INSTALLED" reason="Not implemented yet.">
        Install pack
      </StatusLockedAction>,
    );
    expect(screen.getByText("Install pack")).toBeInTheDocument();
    expect(screen.getByText("Not implemented yet.")).toBeInTheDocument();
    expect(screen.getByText("NOT INSTALLED")).toBeInTheDocument();
  });
});
