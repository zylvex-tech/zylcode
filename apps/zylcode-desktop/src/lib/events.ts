import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useRef, useState, useCallback } from "react";

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

export async function onIntentChunk(cb: (e: StreamDelta) => void): Promise<UnlistenFn> {
  return listen<StreamDelta>("intent:chunk", (ev) => {
    pushBacklog("intent:chunk", ev.payload);
    cb(ev.payload);
  });
}

export async function onIntentDone(cb: (e: IntentDone) => void): Promise<UnlistenFn> {
  return listen<IntentDone>("intent:done", (ev) => {
    pushBacklog("intent:done", ev.payload);
    cb(ev.payload);
  });
}

export async function onStreamDelta(cb: (e: StreamDelta) => void): Promise<UnlistenFn> {
  return listen<StreamDelta>("zylcode://stream-delta", (ev) => {
    pushBacklog("zylcode://stream-delta", ev.payload);
    cb(ev.payload);
  });
}

export async function onMcpToolCall(cb: (e: McpToolCall) => void): Promise<UnlistenFn> {
  return listen<McpToolCall>("zylcode://mcp-tool-call", (ev) => {
    pushBacklog("zylcode://mcp-tool-call", ev.payload);
    cb(ev.payload);
  });
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
    if (!enabled) return;
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
      const u2 = await listen<StreamDelta>("zylcode://stream-delta", (ev) => {
        if (cancelled) return;
        setState((s) => ({
          ...s,
          deltas: [...s.deltas, ev.payload],
          isStreaming: !ev.payload.done,
        }));
      });
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

  return { ...state, reset };
}
