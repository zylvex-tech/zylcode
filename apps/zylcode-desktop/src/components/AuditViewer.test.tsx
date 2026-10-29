import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AuditViewer, type AuditEntry } from "./AuditViewer";

// ---------------------------------------------------------------------------
// Audit trail viewer (components/AuditViewer.tsx).
//
// The honesty rules under test:
//   * an empty trail says so instead of rendering an empty table;
//   * the filter matches tool name AND payload hash, case-insensitively;
//   * the footer's total is the whole recorded trail, not the filtered view —
//     a filter that silently rewrote "how much evidence exists" would be a
//     lie about the audit log;
//   * each status is styled from its own value.
// ---------------------------------------------------------------------------

const entries: AuditEntry[] = [
  {
    id: "1",
    timestamp: "2026-10-28T10:00:00Z",
    toolName: "apply_patch",
    durationMs: 42,
    status: "success",
    payloadHash: "a1b2c3",
  },
  {
    id: "2",
    timestamp: "2026-10-28T10:01:00Z",
    toolName: "execute_tool",
    durationMs: 120,
    status: "retried",
    payloadHash: "deadbeef",
  },
  {
    id: "3",
    timestamp: "2026-10-28T10:02:00Z",
    toolName: "apply_patch",
    durationMs: 7,
    status: "error",
    payloadHash: "ff00ff",
    // error path: same shape, different status styling
  } as AuditEntry,
];

function open() {
  const onClose = vi.fn();
  render(<AuditViewer entries={entries} isOpen onClose={onClose} />);
  return { onClose };
}

describe("AuditViewer", () => {
  it("renders nothing while closed", () => {
    const { container } = render(
      <AuditViewer entries={entries} isOpen={false} onClose={vi.fn()} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("says so when there is no trail to show", () => {
    render(<AuditViewer entries={[]} isOpen onClose={vi.fn()} />);
    expect(screen.getByText("No audit log entries recorded yet.")).toBeInTheDocument();
    expect(screen.getByText("0")).toBeInTheDocument();
  });

  it("lists every entry with status, duration and hash", () => {
    open();
    expect(screen.getAllByText("apply_patch")).toHaveLength(2);
    expect(screen.getByText("execute_tool")).toBeInTheDocument();
    expect(screen.getByText("42ms")).toBeInTheDocument();
    expect(screen.getByText("deadbeef")).toBeInTheDocument();
    expect(screen.getByText("a1b2c3")).toBeInTheDocument();
    // Footer counts the whole trail.
    expect(screen.getByText(/Total Recorded:/)).toHaveTextContent("3");
    // All three status styles are present.
    expect(screen.getAllByText("success")).toHaveLength(1);
    expect(screen.getAllByText("retried")).toHaveLength(1);
    expect(screen.getAllByText("error")).toHaveLength(1);
  });

  it("filters by tool name, case-insensitively", () => {
    open();
    const filter = screen.getByPlaceholderText("Filter by tool or SHA-256 hash...");
    fireEvent.change(filter, { target: { value: "APPLY" } });
    expect(screen.getAllByText("apply_patch")).toHaveLength(2);
    expect(screen.queryByText("execute_tool")).not.toBeInTheDocument();
    // The recorded total does not shrink with the filter.
    expect(screen.getByText(/Total Recorded:/)).toHaveTextContent("3");
  });

  it("filters by payload hash", () => {
    open();
    fireEvent.change(screen.getByPlaceholderText("Filter by tool or SHA-256 hash..."), {
      target: { value: "deadbe" },
    });
    expect(screen.getByText("execute_tool")).toBeInTheDocument();
    expect(screen.queryByText("apply_patch")).not.toBeInTheDocument();
  });

  it("shows the empty state when the filter matches nothing", () => {
    open();
    fireEvent.change(screen.getByPlaceholderText("Filter by tool or SHA-256 hash..."), {
      target: { value: "no-such-tool" },
    });
    expect(screen.getByText("No audit log entries recorded yet.")).toBeInTheDocument();
  });

  it("closes from the header ✕ and the footer Close", () => {
    const { onClose } = open();
    fireEvent.click(screen.getByText("✕"));
    expect(onClose).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
