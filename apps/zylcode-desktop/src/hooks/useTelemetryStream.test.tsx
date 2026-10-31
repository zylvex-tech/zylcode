import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { TauriRuntime } from "../lib/runtime";
import { useAuditIpc, useBridgeIpc, useTelemetryStream } from "./useTelemetryStream";

// ---------------------------------------------------------------------------
// Telemetry hooks (hooks/useTelemetryStream.ts).
//
// Two runtimes, two behaviours, both under test:
//   * browser/jsdom — no `window.__TAURI__`, so the hook must settle into a
//     usable empty state instead of throwing (this is the shape that used to
//     hit the rules-of-hooks defect documented in the source);
//   * desktop — bridge status is primed via invoke, both telemetry channels
//     are subscribed, the metric history is a 20-entry window (newest first),
//     and unmounting unsubscribes.
// ---------------------------------------------------------------------------

type Invoke = ReturnType<typeof vi.fn>;
type Listen = ReturnType<typeof vi.fn>;

/** The injected global, typed through the shared `TauriRuntime` contract. */
const tauriScope = window as unknown as { __TAURI__?: TauriRuntime };

function installTauri(invoke: Invoke, listen: Listen) {
  tauriScope.__TAURI__ = {
    useApp: () => ({ invoke }),
    listen,
  };
}

afterEach(() => {
  delete tauriScope.__TAURI__;
  vi.clearAllMocks();
});

describe("outside the Tauri runtime", () => {
  it("settles into an empty, disconnected state", async () => {
    const { result } = renderHook(() => useTelemetryStream());
    expect(result.current.metricsHistory).toEqual([]);
    expect(result.current.bridgeStatus).toBeNull();
    expect(result.current.isConnected).toBe(false);
    await waitFor(() => expect(result.current.lastUpdate).toBe(0));
  });

  it("useBridgeIpc degrades to a no-op circuit-breaker reset", async () => {
    const { result } = renderHook(() => useBridgeIpc());
    await expect(result.current.resetCircuitBreaker("anthropic")).resolves.toBeUndefined();
  });

  it("useAuditIpc returns no entries without IPC", async () => {
    const { result } = renderHook(() => useAuditIpc());
    await expect(result.current.getRecentAuditLogs()).resolves.toEqual([]);
  });
});

describe("in the Tauri runtime", () => {
  it("primes bridge status from invoke and reports connected", async () => {
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === "get_bridge_status") {
        return [
          { id: "anthropic", status: "connected", latency_ms: 12 },
          { id: "openai", status: "degraded", latency_ms: 40 },
        ];
      }
      return undefined;
    });
    const listen = vi.fn(() => vi.fn());
    installTauri(invoke, listen);

    const { result } = renderHook(() => useTelemetryStream());
    await waitFor(() => expect(result.current.isConnected).toBe(true));
    // Only the first bridge becomes `bridgeStatus`.
    expect(result.current.bridgeStatus).toMatchObject({ id: "anthropic" });
    expect(invoke).toHaveBeenCalledWith("get_bridge_status");
  });

  it("keeps disconnected when the priming invoke rejects", async () => {
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === "get_bridge_status") throw new Error("bridge down");
      return undefined;
    });
    installTauri(
      invoke,
      vi.fn(() => vi.fn()),
    );

    const { result } = renderHook(() => useTelemetryStream());
    // The rejection is swallowed by design: no status, still not connected.
    await waitFor(() => expect(invoke).toHaveBeenCalled());
    expect(result.current.bridgeStatus).toBeNull();
    expect(result.current.isConnected).toBe(false);
  });

  it("applies metric events into a 20-entry newest-first window", async () => {
    const handlers = new Map<string, (e: { payload: unknown }) => void>();
    const listen = vi.fn((event: string, cb: (e: { payload: unknown }) => void) => {
      handlers.set(event, cb);
      return vi.fn();
    });
    installTauri(
      vi.fn(async () => undefined),
      listen,
    );

    const { result, unmount } = renderHook(() => useTelemetryStream());
    expect(listen).toHaveBeenCalledWith("telemetry:metrics", expect.any(Function));
    expect(listen).toHaveBeenCalledWith("telemetry:bridge-status", expect.any(Function));

    act(() => {
      for (let i = 0; i < 25; i++) {
        handlers.get("telemetry:metrics")!({
          payload: {
            input_tokens: i,
            output_tokens: 0,
            verification_saved_tokens: 0,
            fallback_count: 0,
          },
        });
      }
    });
    expect(result.current.metricsHistory).toHaveLength(20);
    expect(result.current.metricsHistory[0].input_tokens).toBe(24);
    expect(result.current.isConnected).toBe(true);

    act(() => {
      handlers.get("telemetry:bridge-status")!({
        payload: {
          id: "mcp",
          transport: "stdio",
          status: "degraded",
          latency_ms: 90,
          circuit_breaker: "open",
        },
      });
    });
    expect(result.current.bridgeStatus).toMatchObject({ status: "degraded" });

    unmount();
    for (const unlisten of vi.mocked(listen).mock.results.map((r) => r.value)) {
      expect(unlisten).toHaveBeenCalledTimes(1);
    }
  });

  it("ignores a bridge-status payload of none", async () => {
    const invoke = vi.fn(async () => []);
    installTauri(
      invoke,
      vi.fn(() => vi.fn()),
    );
    const { result } = renderHook(() => useTelemetryStream());
    // Empty bridge list: stays disconnected, no crash.
    await waitFor(() => expect(invoke).toHaveBeenCalled());
    expect(result.current.bridgeStatus).toBeNull();
  });

  it("useBridgeIpc invokes the backend with the bridge id", async () => {
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === "reset_circuit_breaker") return null;
      if (cmd === "get_bridge_status") return [{ id: "first" }];
      return undefined;
    });
    installTauri(
      invoke,
      vi.fn(() => vi.fn()),
    );

    const { result } = renderHook(() => useBridgeIpc());
    await act(async () => {
      await result.current.resetCircuitBreaker("anthropic");
    });
    expect(invoke).toHaveBeenCalledWith("reset_circuit_breaker", {
      bridge_id: "anthropic",
    });

    let status: unknown = undefined;
    await act(async () => {
      status = await result.current.getBridgeStatus();
    });
    expect(status).toEqual({ id: "first" });
  });

  it("getBridgeStatus returns null for an empty bridge list", async () => {
    installTauri(
      vi.fn(async () => []),
      vi.fn(() => vi.fn()),
    );
    const { result } = renderHook(() => useBridgeIpc());
    let status: unknown = "unset";
    await act(async () => {
      status = await result.current.getBridgeStatus();
    });
    expect(status).toBeNull();
  });

  it("useAuditIpc unwraps the entries returned by the backend", async () => {
    installTauri(
      vi.fn(async (cmd: string) =>
        cmd === "get_recent_audit_logs" ? [{ tool_id: "apply_patch" }] : undefined,
      ),
      vi.fn(() => vi.fn()),
    );
    const { result } = renderHook(() => useAuditIpc());
    await expect(result.current.getRecentAuditLogs()).resolves.toEqual([
      { tool_id: "apply_patch" },
    ]);
  });

  it("useAuditIpc defaults to an empty list for a null response", async () => {
    installTauri(
      vi.fn(async () => null),
      vi.fn(() => vi.fn()),
    );
    const { result } = renderHook(() => useAuditIpc());
    await expect(result.current.getRecentAuditLogs()).resolves.toEqual([]);
  });

  it("useAuditIpc swallows a failing invoke instead of rejecting", async () => {
    installTauri(
      vi.fn(async () => {
        throw new Error("nope");
      }),
      vi.fn(() => vi.fn()),
    );
    const { result } = renderHook(() => useAuditIpc());
    await expect(result.current.getRecentAuditLogs()).resolves.toEqual([]);
  });
});
