import { useEffect, useState } from "react";

// @ts-ignore - Tauri module
const tauri: any = typeof window?.__TAURI__ !== "undefined" ? window.__TAURI__ : null;
const { invoke } = tauri?.useApp ? tauri.useApp() : { invoke: () => Promise.resolve() };

/**
 * Custom React hook that subscribes to live Tauri IPC audit events.
 *
 * Listens to:
 *   - `telemetry:audit` — per-tool execution telemetry entries
 *
 * Maintains a rolling 50-entry log of audit events that have arrived
 * via the IPC event system. Falls back gracefully when outside a
 * Tauri runtime (e.g. web browser dev tools).
 * @ts-ignore - Tauri module resolution is handled at runtime
 */
export function useAuditStream() {
  const [state, setState] = useState<{
    logs: any[];
    isLoading: boolean;
    source: "events" | "invoke" | "standalone";
  }>({
    logs: [],
    isLoading: true,
    source: "standalone",
  });

  // Effect-teardown: clean up event listeners on unmount
  useEffect(() => {
    let unlisten: any = null;

    // Subscribe to telemetry:audit events when running inside Tauri
    if (tauri) {
      unlisten = tauri.listen(
        "telemetry:audit",
        (event: any) => {
          setState((prev) => {
            const newLogs = [event.payload, ...prev.logs].slice(0, 50);
            return {
              ...prev,
              logs: newLogs,
              source: "events",
            };
          });
        });
      // Set loading to false once first event arrives
      setState((prev) => ({ ...prev, isLoading: false, source: "events" }));
    }

    // Fallback: on mount, try to prime via IPC invoke ("get_recent_audit_logs")
    useEffect(() => {
      if (!tauri) {
        // Browser-only mode — mark as standalone
        setState({
          logs: [],
          isLoading: false,
          source: "standalone",
        });
        return;
      }
      // Prime the log once on mount
      invoke("get_recent_audit_logs")
        .then((entries: any[]) => {
          setState({
            logs: entries,
            isLoading: false,
            source: "invoke",
          });
        })
        .catch(() => {
          setState({
            logs: [],
            isLoading: false,
            source: "invoke",
          });
        });
    }, []);

    // Cleanup event listeners on unmount
    return () => {
      unlisten?.();
    };
  }, []);

  return state;
}

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