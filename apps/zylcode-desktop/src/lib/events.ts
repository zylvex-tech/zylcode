// ---------------------------------------------------------------------------
// ZylCode event layer — environment-aware (Product Architecture v3 §11)
//
// Outside the Tauri desktop runtime, NO Tauri event IPC is touched: the
// listeners below resolve to no-ops and the hook reports the environment so
// UI can render controlled states ("Desktop runtime required") instead of
// raw exceptions. Exception details, when they do occur in the desktop
// runtime, go to the diagnostics sink in lib/runtime.ts.
// ---------------------------------------------------------------------------

import { useEffect, useRef, useState, useCallback } from "react";
import {
  detectEnvironment,
  isDesktopRuntime,
  recordDiagnostic,
  type RuntimeEnvironment,
} from "./runtime";
import type { UnlistenFn } from "@tauri-apps/api/event";

// ---------------------------------------------------------------------------
// Types — match Tauri emits in src-tauri/src/main.rs
// ---------------------------------------------------------------------------

export type StreamDelta = {
  index: number;
  delta: string;
  done: boolean;
  phase?: string;
};

export type McpToolCall = {
  tool: string;
  transport?: string;
  status: "calling" | "retry" | "complete" | "failed";
  attempt?: number;
  duration_ms?: number;
  error?: string;
};

export type IntentDone = {
  summary: string;
  artifacts: { kind: string; label: string; content: string }[];
  success: boolean;
};

// ---------------------------------------------------------------------------
// Environment — detected once per process
// ---------------------------------------------------------------------------

export const ENVIRONMENT: RuntimeEnvironment = detectEnvironment();
export const IS_DESKTOP = isDesktopRuntime(ENVIRONMENT);

// ---------------------------------------------------------------------------
// Low-level listeners with backlog buffering
// ---------------------------------------------------------------------------

const backlog: Map<string, unknown[]> = new Map();

function pushBacklog(event: string, payload: unknown) {
  const arr = backlog.get(event) ?? [];
  arr.push(payload);
  // cap at 200 to avoid unbounded growth
  if (arr.length > 200) arr.shift();
  backlog.set(event, arr);
}

export function drainBacklog<T>(event: string): T[] {
  const arr = (backlog.get(event) ?? []) as T[];
  backlog.delete(event);
  return arr;
}

const noopUnlisten: UnlistenFn = () => {};

/**
 * Subscribe to a Tauri event safely. Outside the desktop runtime this
 * resolves immediately to a no-op unlistener — `listen` is never called, so
 * no "Cannot read properties of undefined" exceptions ever occur.
 */
async function listenWhenSafe<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (!IS_DESKTOP) return noopUnlisten;
  try {
    const { listen } = await import("@tauri-apps/api/event");
    return await listen<T>(event, (ev) => handler(ev.payload));
  } catch (e) {
    recordDiagnostic(`events.listen:${event}`, String(e));
    return noopUnlisten;
  }
}

function bufferedHandler<T>(event: string, cb: (e: T) => void) {
  return (payload: T) => {
    pushBacklog(event, payload);
    cb(payload);
  };
}

export function onIntentChunk(cb: (e: StreamDelta) => void): Promise<UnlistenFn> {
  return listenWhenSafe<StreamDelta>(
    "intent:chunk",
    bufferedHandler("intent:chunk", cb),
  );
}

export function onIntentDone(cb: (e: IntentDone) => void): Promise<UnlistenFn> {
  return listenWhenSafe<IntentDone>(
    "intent:done",
    bufferedHandler("intent:done", cb),
  );
}

export function onStreamDelta(cb: (e: StreamDelta) => void): Promise<UnlistenFn> {
  return listenWhenSafe<StreamDelta>(
    "zylcode://stream-delta",
    bufferedHandler("zylcode://stream-delta", cb),
  );
}

export function onMcpToolCall(cb: (e: McpToolCall) => void): Promise<UnlistenFn> {
  return listenWhenSafe<McpToolCall>(
    "zylcode://mcp-tool-call",
    bufferedHandler("zylcode://mcp-tool-call", cb),
  );
}

// ---------------------------------------------------------------------------
// React hook — subscribes to all stream channels, returns state + backlog
// ---------------------------------------------------------------------------

export type StreamState = {
  deltas: StreamDelta[];
  done: IntentDone | null;
  mcpCalls: McpToolCall[];
  isStreaming: boolean;
};

export function useStreamSubscription(enabled = true) {
  const [state, setState] = useState<StreamState>({
    deltas: [],
    done: null,
    mcpCalls: [],
    isStreaming: false,
  });
  const unlistens = useRef<UnlistenFn[]>([]);

  const reset = useCallback(() => {
    setState({ deltas: [], done: null, mcpCalls: [], isStreaming: false });
  }, []);

  useEffect(() => {
    if (!enabled || !IS_DESKTOP) return;
    let cancelled = false;

    // Drain backlog from before mount (e.g. immediate synthetic emit)
    const pendingDeltas = drainBacklog<StreamDelta>("intent:chunk");
    const pendingStream = drainBacklog<StreamDelta>("zylcode://stream-delta");
    const pendingMcp = drainBacklog<McpToolCall>("zylcode://mcp-tool-call");
    if (pendingDeltas.length > 0 || pendingStream.length > 0 || pendingMcp.length > 0) {
      setState((s) => ({
        ...s,
        deltas: [...s.deltas, ...pendingDeltas, ...pendingStream],
        mcpCalls: [...s.mcpCalls, ...pendingMcp],
        isStreaming: true,
      }));
    }

    (async () => {
      const u1 = await onIntentChunk((payload) => {
        if (cancelled) return;
        setState((s) => ({
          ...s,
          deltas: [...s.deltas, payload],
          isStreaming: !payload.done,
        }));
      });
      const u2 = await listenWhenSafe<StreamDelta>(
        "zylcode://stream-delta",
        (payload) => {
          if (cancelled) return;
          setState((s) => ({
            ...s,
            deltas: [...s.deltas, payload],
            isStreaming: !payload.done,
          }));
        },
      );
      const u3 = await onMcpToolCall((payload) => {
        if (cancelled) return;
        setState((s) => ({ ...s, mcpCalls: [...s.mcpCalls, payload] }));
      });
      const u4 = await onIntentDone((payload) => {
        if (cancelled) return;
        setState((s) => ({ ...s, done: payload, isStreaming: false }));
      });
      unlistens.current = [u1, u2, u3, u4];
    })();

    return () => {
      cancelled = true;
      unlistens.current.forEach((fn) => fn());
      unlistens.current = [];
    };
  }, [enabled]);

  return { ...state, reset, environment: ENVIRONMENT, isDesktop: IS_DESKTOP };
}
