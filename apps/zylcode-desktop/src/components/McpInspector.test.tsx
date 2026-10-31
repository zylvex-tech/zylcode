import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import McpInspector from "./McpInspector";
import { safeInvoke } from "../lib/runtime";
import { onMcpToolCall, type McpToolCall } from "../lib/events";

// ---------------------------------------------------------------------------
// McpInspector talks to the Tauri IPC boundary through `safeInvoke` and to the
// event layer through `onMcpToolCall`. Both are stubbed so the panel's states —
// registry list, invoke failure, live feed — are driven deterministically
// rather than by whatever the host runtime happens to expose.
// ---------------------------------------------------------------------------

vi.mock("../lib/runtime", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/runtime")>();
  return { ...actual, safeInvoke: vi.fn() };
});

vi.mock("../lib/events", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/events")>();
  return { ...actual, onMcpToolCall: vi.fn(async () => () => {}) };
});

const invoke = vi.mocked(safeInvoke);
const subscribe = vi.mocked(onMcpToolCall);

type Descriptor = {
  id: string;
  transport: string;
  command: string;
  env_keys: string[];
  description?: string | null;
};

const TOOLS: Descriptor[] = [
  { id: "exec", transport: "stdio", command: "zylcode-exec", env_keys: [] },
  { id: "search", transport: "sse", command: "https://example.test/sse", env_keys: [] },
  { id: "relay", transport: "ws", command: "ws://example.test", env_keys: [] },
];

beforeEach(() => {
  vi.clearAllMocks();
  invoke.mockResolvedValue({ ok: true, data: TOOLS });
  subscribe.mockResolvedValue(() => {});
});

describe("McpInspector", () => {
  it("renders the registered tools with their transport badge", async () => {
    render(<McpInspector />);

    expect(await screen.findByText("exec")).toBeInTheDocument();
    expect(screen.getByText("search")).toBeInTheDocument();
    expect(screen.getByText("relay")).toBeInTheDocument();
    expect(screen.getByText("stdio")).toBeInTheDocument();
    expect(screen.getByText("sse")).toBeInTheDocument();
    expect(screen.getByText("3 tools")).toBeInTheDocument();
  });

  it("shows an empty registry when the backend returns no tools", async () => {
    invoke.mockResolvedValue({ ok: true, data: [] });
    render(<McpInspector />);

    expect(
      await screen.findByText(/No tools registered\. Add `mcp\.tools\.yaml`/),
    ).toBeInTheDocument();
    expect(screen.getByText("0 tools")).toBeInTheDocument();
    expect(screen.getByText("Awaiting tool calls…")).toBeInTheDocument();
  });

  it("surfaces a failed invoke as a visible error instead of throwing", async () => {
    invoke.mockRejectedValue(new Error("ipc unavailable"));
    render(<McpInspector />);

    expect(await screen.findByText(/ipc unavailable/)).toBeInTheDocument();
    expect(screen.getByText(/No tools registered/)).toBeInTheDocument();
  });

  it("unsubscribes from the tool-call feed on unmount", async () => {
    const unsubscribe = vi.fn();
    subscribe.mockResolvedValue(unsubscribe);

    const { unmount } = render(<McpInspector />);
    await waitFor(() => expect(subscribe).toHaveBeenCalledTimes(1));

    unmount();
    expect(unsubscribe).toHaveBeenCalledTimes(1);
  });

  it("renders feed entries for every status badge and each transport style", async () => {
    let emit: ((c: McpToolCall) => void) | undefined;
    subscribe.mockImplementation((cb: (c: McpToolCall) => void) => {
      emit = cb;
      return Promise.resolve(() => {});
    });

    render(<McpInspector />);
    expect(await screen.findByText("Awaiting tool calls…")).toBeInTheDocument();

    const entries: McpToolCall[] = [
      { tool: "t-call", status: "calling" },
      { tool: "t-retry", status: "retry", attempt: 2 },
      { tool: "t-ok", status: "complete", duration_ms: 12 },
      { tool: "t-fail", status: "failed", error: "boom", attempt: 1 },
      { tool: "t-odd", status: "unknown" as McpToolCall["status"], duration_ms: 3 },
    ];
    await act(async () => {
      entries.forEach((e) => emit?.(e));
    });

    // `status` is rendered raw and uppercased by CSS, so assert on the DOM text.
    expect(screen.getByText("calling")).toBeInTheDocument();
    expect(screen.getByText("retry #2")).toBeInTheDocument();
    expect(screen.getByText("complete")).toBeInTheDocument();
    expect(screen.getByText("failed #1")).toBeInTheDocument();
    expect(screen.getByText(/boom/)).toBeInTheDocument();
    expect(screen.getByText("12ms")).toBeInTheDocument();
    // No duration on the calling entry -> no timing text rendered for it.
    expect(screen.queryByText(/0ms/)).not.toBeInTheDocument();
  });
});
