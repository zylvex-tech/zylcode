import { useEffect, useState, useCallback } from "react";
import { getTauriRuntime, type Unlisten } from "../lib/runtime";

export interface TelemetryMetrics {
  input_tokens: number;
  output_tokens: number;
  verification_saved_tokens: number;
  fallback_count: number;
}

export interface BridgeStatus {
  id: string;
  name: string;
  transport: string;
  status: "connected" | "degraded" | "disconnected";
  latency_ms: number;
  circuit_breaker: "open" | "closed";
}

export interface TelemetryStreamState {
  metricsHistory: TelemetryMetrics[];
  bridgeStatus: BridgeStatus | null;
  isConnected: boolean;
  lastUpdate: number;
}

/**
 * Custom React hook that subscribes to live Tauri IPC telemetry events.
 *
 * Listens to:
 *   - `telemetry:metrics` — token usage and verification savings
 *   - `telemetry:bridge-status` — MCP bridge health indicators
 *
 * Maintains a rolling history window of the latest 20 metric data points
 * for ResourceGraph consumption, and tracks the latest bridge status.
 *
 * Falls back gracefully when outside a Tauri runtime (e.g. web browser dev).
 * The injected global's shape lives in `lib/runtime.ts` as `TauriRuntime`.
 */
export function useTelemetryStream() {
  const tauri = getTauriRuntime();
  const { invoke } = tauri?.useApp ? tauri.useApp() : { invoke: () => Promise.resolve() };

  const [state, setState] = useState<TelemetryStreamState>({
    metricsHistory: [],
    bridgeStatus: null,
    isConnected: false,
    lastUpdate: 0,
  });

  // Fallback: on initial mount, query bridge status via IPC invoke
  //
  // This used to be nested INSIDE the listener effect below, which is a
  // rules-of-hooks violation: React cannot register a hook during the commit
  // phase, so any component that actually called this hook threw
  //     "Error: Invalid hook call. Hooks can only be called inside of the
  //      body of a function component."
  // Reproduced 2026-10-21 by rendering a probe component in jsdom (same shape
  // as the identical defect fixed in useAuditStream.ts).
  //
  // Hoisted to a sibling effect with the same `[invoke]` trigger, so the
  // before/after behaviour matches: prime whenever `invoke` changes.
  useEffect(() => {
    if (!tauri) {
      // Browser-only mode — no Tauri IPC available
      return;
    }
    // Prime the bridge status once so UI isn't blank on first render
    invoke("get_bridge_status")
      .then((raw) => {
        // IPC boundary: the engine answers with `BridgeStatus[]`.
        const bridges = raw as BridgeStatus[];
        if (bridges.length > 0) {
          setState((prev) => ({
            ...prev,
            bridgeStatus: bridges[0],
            isConnected: true,
            lastUpdate: Date.now(),
          }));
        }
      })
      .catch(() => {
        // Graceful fallback — keep isConnected false
      });
    // `tauri` is derived from `window.__TAURI__` at render time and is stable,
    // but the effect does read it, so it is declared rather than assumed.
  }, [invoke, tauri]);

  useEffect(() => {
    let metricsUnlisten: Unlisten | null = null;
    let bridgeUnlisten: Unlisten | null = null;

    if (tauri) {
      // Subscribe to telemetry:metrics events
      metricsUnlisten = tauri.listen<TelemetryMetrics>("telemetry:metrics", (event) => {
        setState((prev) => {
          const newHistory = [event.payload, ...prev.metricsHistory].slice(0, 20);
          return {
            ...prev,
            metricsHistory: newHistory,
            isConnected: true,
            lastUpdate: Date.now(),
          };
        });
      });

      // Subscribe to telemetry:bridge-status events
      bridgeUnlisten = tauri.listen<BridgeStatus>("telemetry:bridge-status", (event) => {
        setState((prev) => ({
          ...prev,
          bridgeStatus: event.payload,
          isConnected: true,
          lastUpdate: Date.now(),
        }));
      });
    }

    // Cleanup event listeners on unmount
    return () => {
      metricsUnlisten?.();
      bridgeUnlisten?.();
    };
  }, [invoke, tauri]);

  return state;
}

/**
 * Convenience hook that provides IPC invoke access for
 * bridge control actions from the UI.
 */
export function useBridgeIpc() {
  const tauri = getTauriRuntime();
  const { invoke } = tauri?.useApp ? tauri.useApp() : { invoke: () => Promise.resolve() };

  const resetCircuitBreaker = useCallback(
    async (bridgeId: string) => {
      if (!invoke) return;
      await invoke("reset_circuit_breaker", { bridge_id: bridgeId });
    },
    [invoke],
  );

  const getBridgeStatus = useCallback(async (): Promise<BridgeStatus | null> => {
    if (!invoke) return null;
    // IPC boundary: the engine answers with `BridgeStatus[]`.
    const bridges = (await invoke("get_bridge_status")) as BridgeStatus[];
    return bridges.length > 0 ? bridges[0] : null;
  }, [invoke]);

  return { resetCircuitBreaker, getBridgeStatus };
}

/**
 * Convenience hook that provides IPC invoke access for
 * audit log retrieval from the UI.
 */
export const useAuditIpc = (): {
  getRecentAuditLogs: () => Promise<AuditLogEntry[]>;
} => {
  const tauri = getTauriRuntime();
  const { invoke } = tauri?.useApp ? tauri.useApp() : { invoke: () => Promise.resolve() };

  const getRecentAuditLogs = useCallback(async (): Promise<AuditLogEntry[]> => {
    if (!invoke) return [];
    try {
      // IPC boundary: the engine answers with `AuditLogEntry[]`.
      const entries = (await invoke("get_recent_audit_logs")) as AuditLogEntry[];
      return entries || [];
    } catch {
      return [];
    }
  }, [invoke]);

  return { getRecentAuditLogs };
};

/** Audit log entry shape matching what IPC returns. */
export interface AuditLogEntry {
  timestamp: string;
  tool_id: string;
  event_type: string;
  input_hash?: string;
  output_hash?: string;
  duration_ms?: number;
  error?: string | null;
}
