import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ENVIRONMENT,
  IS_DESKTOP,
  drainBacklog,
  onIntentChunk,
  onIntentDone,
  onMcpToolCall,
  safeListen,
  useStreamSubscription,
} from "./events";

// ---------------------------------------------------------------------------
// Event layer (lib/events.ts).
//
// The contract has two halves:
//   * Outside the desktop runtime — which is what jsdom is — no Tauri IPC is
//     ever touched. `listen` must stay uncalled; the hook must still return a
//     usable object so UI can render a controlled state instead of throwing.
//   * In the desktop runtime the same calls subscribe to the four channels
//     Tauri emits, buffer payloads that arrive before anyone subscribes
//     (capped), and unsubscribe every listener on unmount.
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
  emit: vi.fn(),
}));

afterEach(() => {
  vi.clearAllMocks();
});

describe("outside the desktop runtime", () => {
  it("classifies jsdom as non-desktop", () => {
    expect(IS_DESKTOP).toBe(false);
    expect(ENVIRONMENT).not.toBe("tauri-desktop");
  });

  it("never reaches the Tauri listener", async () => {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await safeListen("intent:chunk", () => {});
    expect(typeof unlisten).toBe("function");
    unlisten(); // must be safe to call
    expect(listen).not.toHaveBeenCalled();
  });

  it("the channel helpers resolve to no-op unlisteners", async () => {
    const { listen } = await import("@tauri-apps/api/event");
    const unlistens = await Promise.all([
      onIntentChunk(() => {}),
      onIntentDone(() => {}),
      onMcpToolCall(() => {}),
    ]);
    for (const u of unlistens) expect(typeof u).toBe("function");
    expect(listen).not.toHaveBeenCalled();
  });

  it("drains nothing for a channel nobody wrote to", () => {
    expect(drainBacklog("never-emitted")).toEqual([]);
  });

  it("returns a usable hook object and reset() clears it", () => {
    const { result } = renderHook(() => useStreamSubscription());
    expect(result.current.isDesktop).toBe(false);
    expect(result.current.isStreaming).toBe(false);
    expect(result.current.deltas).toEqual([]);
    expect(result.current.done).toBeNull();
    expect(result.current.mcpCalls).toEqual([]);
    act(() => result.current.reset());
    expect(result.current.deltas).toEqual([]);
  });

  it("subscribes to nothing when disabled", () => {
    const { result } = renderHook(() => useStreamSubscription(false));
    expect(result.current.isDesktop).toBe(false);
    expect(result.current.environment).toBe(ENVIRONMENT);
  });
});

/** Load a fresh copy of the module with the desktop runtime faked. */
async function loadDesktopEvents() {
  vi.resetModules();
  vi.doMock("./runtime", () => ({
    detectEnvironment: () => "tauri-desktop",
    isDesktopRuntime: () => true,
    recordDiagnostic: vi.fn(),
  }));
  const handlers = new Map<string, (ev: { payload: unknown }) => void>();
  const unlisten = vi.fn();
  const { listen } = await import("@tauri-apps/api/event");
  vi.mocked(listen).mockImplementation(async (event, cb) => {
    handlers.set(event, cb as (ev: { payload: unknown }) => void);
    return unlisten;
  });
  const mod = await import("./events");
  return { mod, handlers, unlisten };
}

describe("in the desktop runtime", () => {
  it("subscribes to the named channel and delivers the payload", async () => {
    const { mod, handlers } = await loadDesktopEvents();
    expect(mod.IS_DESKTOP).toBe(true);

    const received: unknown[] = [];
    await mod.onIntentChunk((e) => received.push(e));
    expect(handlers.has("intent:chunk")).toBe(true);

    handlers.get("intent:chunk")!({
      payload: { index: 0, delta: "hello", done: false },
    });
    expect(received).toHaveLength(1);
    expect(received[0]).toMatchObject({ delta: "hello" });
  });

  it("buffers payloads for a late subscriber and caps the buffer at 200", async () => {
    const { mod, handlers } = await loadDesktopEvents();
    await mod.onIntentChunk(() => {});

    for (let i = 0; i < 205; i++) {
      handlers.get("intent:chunk")!({ payload: { index: i, delta: "x", done: false } });
    }

    const drained = mod.drainBacklog<{ index: number }>("intent:chunk");
    expect(drained).toHaveLength(200);
    // Oldest payloads were dropped, not the newest.
    expect(drained[drained.length - 1].index).toBe(204);
    // Draining is destructive.
    expect(mod.drainBacklog("intent:chunk")).toEqual([]);
  });

  it("routes every channel through its own subscription", async () => {
    const { mod, handlers } = await loadDesktopEvents();
    await mod.onIntentDone(() => {});
    await mod.onMcpToolCall(() => {});
    expect(handlers.has("intent:done")).toBe(true);
    expect(handlers.has("zylcode://mcp-tool-call")).toBe(true);
    expect(handlers.has("zylcode://stream-delta")).toBe(false);
  });

  it("drives the hook's stream state and unsubscribes on unmount", async () => {
    const { mod, handlers, unlisten } = await loadDesktopEvents();
    const { result, unmount } = renderHook(() => mod.useStreamSubscription());

    await waitFor(() => expect(handlers.size).toBeGreaterThanOrEqual(4));

    act(() => {
      handlers.get("intent:chunk")!({
        payload: { index: 0, delta: "plan", done: false },
      });
    });
    expect(result.current.deltas).toHaveLength(1);
    expect(result.current.isStreaming).toBe(true);

    act(() => {
      handlers.get("zylcode://mcp-tool-call")!({
        payload: { tool: "apply_patch", status: "complete" },
      });
    });
    expect(result.current.mcpCalls).toHaveLength(1);

    act(() => {
      handlers.get("intent:done")!({
        payload: { summary: "done", artifacts: [], success: true },
      });
    });
    expect(result.current.done).toMatchObject({ summary: "done" });
    expect(result.current.isStreaming).toBe(false);

    act(() => result.current.reset());
    expect(result.current.deltas).toEqual([]);
    expect(result.current.done).toBeNull();

    unmount();
    expect(unlisten).toHaveBeenCalled();
  });

  it("does not subscribe when the hook is disabled", async () => {
    const { mod, handlers } = await loadDesktopEvents();
    renderHook(() => mod.useStreamSubscription(false));
    await new Promise((r) => setTimeout(r, 10));
    expect(handlers.size).toBe(0);
  });
});