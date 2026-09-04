import { useEffect, useState, useCallback } from "react";

/**
 * Type declarations for Tauri IPC - these are available at runtime
 * when running inside a Tauri application. TypeScript cannot resolve
 * the module statically, so we use `any` types and `// @ts-ignore`
 * comments to bypass compile-time checking.
 */
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
 * @ts-ignore - Tauri module resolution is handled at runtime
 */
export function useTelemetryStream() {
  // @ts-ignore - Tauri module
  const tauri: any = typeof window?.__TAURI__ !== "undefined" ? window.__TAURI__ : null;
  const { invoke } = tauri?.useApp ? tauri.useApp() : { invoke: () => Promise.resolve() };

  const [state, setState] = useState<TelemetryStreamState>({
    metricsHistory: [],
    bridgeStatus: null,
    isConnected: false,
    lastUpdate: 0,
  });

  useEffect(() => {
    let metricsUnlisten: any = null;
    let bridgeUnlisten: any = null;

    if (tauri) {
      // Subscribe to telemetry:metrics events
      metricsUnlisten = tauri.listen(
        "telemetry:metrics",
        (event: any) => {
          setState((prev) => {
            const newHistory = [event.payload, ...prev.metricsHistory].slice(0, 20);
            return {
              ...prev,
              metricsHistory: newHistory,
              isConnected: true,
              lastUpdate: Date.now(),
            };
          });
        },
      );

      // Subscribe to telemetry:bridge-status events
      bridgeUnlisten = tauri.listen(
        "telemetry:bridge-status",
        (event: any) => {
          setState((prev) => ({
            ...prev,
            bridgeStatus: event.payload,
            isConnected: true,
            lastUpdate: Date.now(),
          }));
        },
      );
    }

    // Fallback: on initial mount, query bridge status via IPC invoke
    useEffect(() => {
      if (!tauri) {
        // Browser-only mode — no Tauri IPC available
        return;
      }
      // Prime the bridge status once so UI isn't blank on first render
      invoke("get_bridge_status")
        .then((bridges: any[]) => {
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
    }, [invoke]);

    // Cleanup event listeners on unmount
    return () => {
      metricsUnlisten?.();
      bridgeUnlisten?.();
    };
  }, [invoke]);

  return state;
}

/**
 * Convenience hook that provides IPC invoke access for
 * bridge control actions from the UI.
 * @ts-ignore - Tauri module
 */
export function useBridgeIpc() {
  // @ts-ignore - Tauri module
  const tauri: any = typeof window?.__TAURI__ !== "undefined" ? window.__TAURI__ : null;
  const { invoke } = tauri?.useApp ? tauri.useApp() : { invoke: () => Promise.resolve() };

  const resetCircuitBreaker = useCallback(
    async (bridgeId: string) => {
      if (!invoke) return;
      await invoke("reset_circuit_breaker", { bridge_id: bridgeId });
    },
    [invoke],
  );

  const getBridgeStatus = useCallback(async (): Promise<any> => {
    if (!invoke) return null;
    const bridges: any[] = await invoke("get_bridge_status");
    return bridges.length > 0 ? bridges[0] : null;
  }, [invoke]);

  return { resetCircuitBreaker, getBridgeStatus };
}

/**
 * Convenience hook that provides IPC invoke access for
 * audit log retrieval from the UI.
 * @ts-ignore - Tauri module
 */
export const useAuditIpc = (): {
  getRecentAuditLogs: () => Promise<any[]>;
} => {
  // @ts-ignore - Tauri module
  const tauri: any = typeof window?.__TAURI__ !== "undefined" ? window.__TAURI__ : null;
  const { invoke } = tauri?.useApp ? tauri.useApp() : { invoke: () => Promise.resolve() };

  const getRecentAuditLogs = useCallback(async (): Promise<any[]> => {
    if (!invoke) return [];
    try {
      const entries: any[] = await invoke("get_recent_audit_logs");
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