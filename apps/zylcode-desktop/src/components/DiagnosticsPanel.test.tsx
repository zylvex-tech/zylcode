import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { DiagnosticsPanel } from "./DiagnosticsPanel";
import { clearDiagnostics, getDiagnostics, recordDiagnostic } from "../lib/runtime";

// ---------------------------------------------------------------------------
// DiagnosticsPanel reads the module-level diagnostic ring buffer directly (no
// props, no async), so the real buffer is used rather than a stub — that keeps
// the Clear button's actual side effect under test.
// ---------------------------------------------------------------------------

afterEach(() => {
  clearDiagnostics();
});

describe("DiagnosticsPanel", () => {
  it("shows an empty state when nothing has been recorded", () => {
    render(<DiagnosticsPanel />);
    expect(screen.getByText("No diagnostics recorded.")).toBeInTheDocument();
  });

  it("lists each recorded entry with timestamp, source and message", () => {
    recordDiagnostic("ipc", "invoke failed");
    recordDiagnostic("stream", "reconnected");
    render(<DiagnosticsPanel />);

    expect(screen.getByText(/\[.*\] ipc: invoke failed/)).toBeInTheDocument();
    expect(screen.getByText(/\[.*\] stream: reconnected/)).toBeInTheDocument();
  });

  it("Clear empties the shared buffer and re-renders the empty state", () => {
    recordDiagnostic("ipc", "boom");
    render(<DiagnosticsPanel />);
    expect(screen.getByText(/boom/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Clear" }));

    expect(screen.getByText("No diagnostics recorded.")).toBeInTheDocument();
    expect(getDiagnostics()).toHaveLength(0);
  });
});
