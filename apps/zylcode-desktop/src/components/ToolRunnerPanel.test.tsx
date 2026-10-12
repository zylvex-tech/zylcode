// Tool runner panel regression tests.
//
// Guards: only executor-bound ids are offered; invalid JSON arguments are
// declined *before* any execution ("declined" reads as a fix-it message, not
// an exception); a failed execution renders its engine message as a failure;
// a successful execution renders the engine payload (risk/operation/output).

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ToolRunnerPanel } from "./ToolRunnerPanel";
import { fetchTools, type ToolRow } from "../lib/service";
import type { ToolRunState } from "../lib/toolRunner";

vi.mock("../lib/service", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/service")>();
  return { ...actual, fetchTools: vi.fn() };
});
vi.mock("../lib/toolRunner", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/toolRunner")>();
  return { ...actual, runTool: vi.fn() };
});

import { runTool } from "../lib/toolRunner";

const fetchToolsMock = vi.mocked(fetchTools);
const runToolMock = vi.mocked(runTool);

const ROWS: ToolRow[] = [
  { id: "fs.read", transport: "stdio", description: "Read a file", enabled: true, executor_bound: true },
  { id: "db.query", transport: "stdio", description: null, enabled: true, executor_bound: false },
];

beforeEach(() => {
  runToolMock.mockReset();
});

describe("ToolRunnerPanel", () => {
  it("offers executor-bound ids only", async () => {
    fetchToolsMock.mockResolvedValue({ kind: "ready", source: "mcp.tools.yaml", tools: ROWS });
    render(<ToolRunnerPanel />);
    await waitFor(() => expect(screen.getByTestId("tool-runner-ids")).toBeTruthy());
    expect(screen.queryByText("db.query")).toBeNull();
  });

  it("declines invalid JSON arguments without executing", async () => {
    fetchToolsMock.mockResolvedValue({ kind: "ready", source: "mcp.tools.yaml", tools: ROWS });
    render(<ToolRunnerPanel />);
    await waitFor(() => expect(screen.getByTestId("tool-runner-form")).toBeTruthy());
    act(() => {
      fireEvent.change(screen.getByRole("textbox"), { target: { value: "{ not json" } });
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("tool-runner-execute"));
    });
    expect(runToolMock).not.toHaveBeenCalled();
    expect(screen.getByRole("alert").textContent).toMatch(/not valid JSON/);
  });

  it("renders a failed execution as a failure, with the engine's message", async () => {
    fetchToolsMock.mockResolvedValue({ kind: "ready", source: "mcp.tools.yaml", tools: ROWS });
    runToolMock.mockResolvedValue({
      kind: "failed",
      id: "fs.read",
      via: "service",
      error: "the tool reported failure (operation: fs.read)",
    } satisfies ToolRunState);
    render(<ToolRunnerPanel />);
    await waitFor(() => expect(screen.getByTestId("tool-runner-form")).toBeTruthy());
    await act(async () => {
      fireEvent.click(screen.getByTestId("tool-runner-execute"));
    });
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toMatch(/failure/);
    // Prefilled example args for fs.read, passed through untouched.
    expect(runToolMock).toHaveBeenCalledWith("fs.read", { path: "README.md" }, ROWS);
  });

  it("renders a successful execution with the engine payload", async () => {
    fetchToolsMock.mockResolvedValue({ kind: "ready", source: "mcp.tools.yaml", tools: ROWS });
    runToolMock.mockResolvedValue({
      kind: "done",
      via: "service",
      result: {
        tool: "fs.read",
        executed: true,
        success: true,
        output: { path: "README.md", content: "# ZylCode" },
        risk: "READ",
        bound_operation: "fs.read",
        invocation_id: "ev-9",
      },
    } satisfies ToolRunState);
    render(<ToolRunnerPanel />);
    await waitFor(() => expect(screen.getByTestId("tool-runner-form")).toBeTruthy());
    await act(async () => {
      fireEvent.click(screen.getByTestId("tool-runner-execute"));
    });
    expect(screen.getByTestId("tool-runner-output").textContent).toMatch(/fs\.read/);
    expect(screen.getByTestId("tool-runner-output").textContent).toMatch(/ev-9/);
  });
});
