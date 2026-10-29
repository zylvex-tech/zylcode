import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AboutModal } from "./AboutModal";

// ---------------------------------------------------------------------------
// About modal (components/AboutModal.tsx).
//
// It is a static facts panel, so the tests pin the facts (version, runtime,
// protocol, circuit state) and both dismissal affordances — an About dialog
// the user cannot close is a modal that traps them.
// ---------------------------------------------------------------------------

describe("AboutModal", () => {
  it("renders nothing while closed", () => {
    const { container } = render(<AboutModal isOpen={false} onClose={vi.fn()} />);
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByText(/Core Runtime/)).not.toBeInTheDocument();
  });

  it("states version, runtime, protocol and circuit state", () => {
    render(<AboutModal isOpen onClose={vi.fn()} />);
    expect(screen.getByAltText("ZylCode")).toBeInTheDocument();
    expect(screen.getByText("v0.2.3")).toBeInTheDocument();
    expect(screen.getByText("Rust (memchr / tokio)")).toBeInTheDocument();
    expect(
      screen.getByText("MCP (Model Context Protocol)"),
    ).toBeInTheDocument();
    expect(screen.getByText("CLOSED (Normal operation)")).toBeInTheDocument();
    expect(
      screen.getByText("Formally Verified AI Software Synthesis Engine"),
    ).toBeInTheDocument();
  });

  it("closes from the header ✕ and from the footer button", () => {
    const onClose = vi.fn();
    render(<AboutModal isOpen onClose={onClose} />);
    fireEvent.click(screen.getByText("✕"));
    expect(onClose).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});