import { useEffect, useState } from "react";
import { getTauriRuntime, type Unlisten } from "../lib/runtime";

// Read once at module load: the test suite installs `window.__TAURI__` before
// importing this module, and the desktop shell injects it before any bundle
// evaluates. The shape of that global lives in `lib/runtime.ts`.
const tauri = getTauriRuntime();
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
 */
export function useAuditStream() {
  const [state, setState] = useState<{
    logs: AuditLogEntry[];
    isLoading: boolean;
    source: "events" | "invoke" | "standalone";
  }>({
    logs: [],
    isLoading: true,
    source: "standalone",
  });

  // Fallback: on mount, try to prime via IPC invoke ("get_recent_audit_logs")
  //
  // This used to be nested INSIDE the effect below, which is a rules-of-hooks
  // violation: React cannot register a hook during the commit phase, so any
  // component that actually called this hook threw
  //     "Error: Invalid hook call. Hooks can only be called inside of the
  //      body of a function component."
  // Reproduced 2026-10-21 by rendering a probe component in jsdom.
  //
  // Hoisted to a sibling effect. The trigger condition is unchanged: `invoke`
  // is a module-level constant, so `[]` and `[invoke]` are equivalent here.
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
      .then((raw) => {
        // IPC boundary: the engine answers with `AuditLogEntry[]`.
        const entries = raw as AuditLogEntry[];
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

  // Effect-teardown: clean up event listeners on unmount
  useEffect(() => {
    let unlisten: Unlisten | null = null;

    // Subscribe to telemetry:audit events when running inside Tauri
    if (tauri) {
      unlisten = tauri.listen<AuditLogEntry>("telemetry:audit", (event) => {
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
