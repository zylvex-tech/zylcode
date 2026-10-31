import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { TauriRuntime } from "../lib/runtime";

// ---------------------------------------------------------------------------
// Audit stream (hooks/useAuditStream.ts).
//
// The module reads `window.__TAURI__` ONCE at import time, so each scenario
// loads a fresh copy of the module with the global configured beforehand —
// that is the only way to exercise both branches in one file.
// ---------------------------------------------------------------------------

/** The injected global, typed through the shared `TauriRuntime` contract. */
const tauriScope = window as unknown as { __TAURI__?: TauriRuntime };

afterEach(() => {
  delete tauriScope.__TAURI__;
  vi.clearAllMocks();
});

async function load(options: { desktop: boolean; invoke?: ReturnType<typeof vi.fn> }) {
  vi.resetModules();
  const listen = vi.fn((_event: string, _handler: (e: { payload: unknown }) => void) => vi.fn());
  if (options.desktop) {
    tauriScope.__TAURI__ = {
      useApp: () => ({ invoke: options.invoke ?? vi.fn(async () => []) }),
      // The spy is deliberately not generic; it records (event, handler) pairs,
      // and the contract it must satisfy is asserted by the tests below.
      listen: listen as unknown as TauriRuntime["listen"],
    };
  } else {
    delete tauriScope.__TAURI__;
  }
  const mod = await import("./useAuditStream");
  return { mod, listen };
}

describe("outside the Tauri runtime", () => {
  it("settles into a standalone, non-loading empty log", async () => {
    const { mod } = await load({ desktop: false });
    const { result } = renderHook(() => mod.useAuditStream());
    // Effects flush inside act(), so the standalone branch has already
    // settled by the time the first snapshot is read.
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.source).toBe("standalone");
    expect(result.current.logs).toEqual([]);
  });
});

describe("in the Tauri runtime", () => {
  it("primes the log from invoke and reports the invoke source", async () => {
    const invoke = vi.fn(async (cmd: string) =>
      cmd === "get_recent_audit_logs"
        ? [{ tool_id: "execute_tool", event_type: "tool.call" }]
        : undefined,
    );
    const { mod } = await load({ desktop: true, invoke });
    const { result } = renderHook(() => mod.useAuditStream());

    // Wait for the final source, not just loading: the event-listener effect
    // sets "events" synchronously on mount, and the priming invoke only
    // overwrites it to "invoke" once its promise settles.
    await waitFor(() => expect(result.current.source).toBe("invoke"));
    expect(result.current.isLoading).toBe(false);
    expect(result.current.logs).toHaveLength(1);
    expect(invoke).toHaveBeenCalledWith("get_recent_audit_logs");
  });

  it("still settles when the priming invoke fails", async () => {
    const invoke = vi.fn(async () => {
      throw new Error("ipc down");
    });
    const { mod } = await load({ desktop: true, invoke });
    const { result } = renderHook(() => mod.useAuditStream());
    // Same ordering note as above: the failure path must win over the
    // synchronous "events" set from the listener effect.
    await waitFor(() => expect(result.current.source).toBe("invoke"));
    expect(result.current.isLoading).toBe(false);
    expect(result.current.logs).toEqual([]);
  });

  it("switches to the live event source, keeps 50 entries, cleans up", async () => {
    const { mod, listen } = await load({ desktop: true });
    const handlers = new Map<string, (e: { payload: unknown }) => void>();
    listen.mockImplementation((event, cb) => {
      handlers.set(event, cb);
      return vi.fn();
    });

    const { result, unmount } = renderHook(() => mod.useAuditStream());
    expect(listen).toHaveBeenCalledWith("telemetry:audit", expect.any(Function));

    act(() => {
      for (let i = 0; i < 60; i++) {
        handlers.get("telemetry:audit")!({ payload: { tool_id: `tool-${i}` } });
      }
    });
    expect(result.current.source).toBe("events");
    expect(result.current.isLoading).toBe(false);
    expect(result.current.logs).toHaveLength(50);
    // Newest first.
    expect(result.current.logs[0].tool_id).toBe("tool-59");

    unmount();
    for (const r of listen.mock.results) {
      expect(r.value).toHaveBeenCalledTimes(1);
    }
  });

  it("degrades gracefully when the runtime exposes no app IPC surface", async () => {
    // `useApp` is optional in the TauriRuntime contract, so a shell that
    // injects only the event bus must still subscribe and settle — the hook
    // must not throw just because app-scoped invoke is absent.
    vi.resetModules();
    const listen = vi.fn(() => vi.fn());
    tauriScope.__TAURI__ = { listen: listen as unknown as TauriRuntime["listen"] };
    const mod = await import("./useAuditStream");

    const { result } = renderHook(() => mod.useAuditStream());

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(listen).toHaveBeenCalledWith("telemetry:audit", expect.any(Function));
    expect(["events", "invoke"]).toContain(result.current.source);
  });
});
