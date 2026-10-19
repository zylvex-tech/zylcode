// ---------------------------------------------------------------------------
// Repository context freshness (lib/repoContext.ts) — P15 W4
//
// The observation signal behind `GET /api/repo-context` and its SSE twin
// `GET /api/repo-context-stream`: is the persisted index answering for the
// tree as it is right now — `fresh`, `stale` (files really changed since the
// index was written), or `reindexing` (a scan is running this moment)?
//
// One payload shape, two transports:
//   * Tauri desktop  -> `repo_context_status` IPC, polled — the engine runs
//                       in another process, so the desktop samples instead
//                       of subscribing;
//   * browser preview -> `GET /api/repo-context`, or the SSE stream for
//                       live transitions (initial event, then only real
//                       state/fingerprint changes).
//
// Honesty rules, same as every lib in this folder: an unreachable engine is
// `unavailable` with a reason — never rendered as `fresh`; a payload the
// engine shaped is rendered as-is; an unrecognised payload is `unavailable`,
// never guessed into a state.
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic, safeInvoke } from "./runtime";

export type RepoContextKind = "fresh" | "stale" | "reindexing";

export type RepoContextSignal =
  | { kind: "connecting" }
  | {
      kind: RepoContextKind;
      /** Short (12-hex) fingerprint measured right now. */
      fingerprint: string;
      /** Short fingerprint the index was built at — null when no index. */
      indexedFingerprint: string | null;
      /** Content changes since the index — null when no baseline exists. */
      changedFiles: number | null;
    }
  | { kind: "unavailable"; reason: string };

type RawPayload = {
  state?: unknown;
  fingerprint?: unknown;
  indexed_fingerprint?: unknown;
  changed_files?: unknown;
  error?: unknown;
};

/** Map an engine payload to a signal. Unknown shapes and engine errors become
 *  `unavailable` with the message — never a guessed state. */
export function signalFromPayload(payload: unknown): RepoContextSignal {
  if (!payload || typeof payload !== "object") {
    return {
      kind: "unavailable",
      reason: "the freshness endpoint returned no payload",
    };
  }
  const raw = payload as RawPayload;
  if (typeof raw.error === "string" && raw.error.length > 0) {
    return { kind: "unavailable", reason: raw.error };
  }
  const state = raw.state;
  if (state !== "fresh" && state !== "stale" && state !== "reindexing") {
    return {
      kind: "unavailable",
      reason: `unrecognised freshness state: ${JSON.stringify(state ?? null)}`,
    };
  }
  if (typeof raw.fingerprint !== "string" || raw.fingerprint.length === 0) {
    return {
      kind: "unavailable",
      reason: "the freshness payload carries no fingerprint",
    };
  }
  return {
    kind: state,
    fingerprint: raw.fingerprint,
    indexedFingerprint:
      typeof raw.indexed_fingerprint === "string"
        ? raw.indexed_fingerprint
        : null,
    changedFiles:
      typeof raw.changed_files === "number" ? raw.changed_files : null,
  };
}

/** One-shot freshness observation on the current transport. */
export async function fetchRepoContextStatus(): Promise<RepoContextSignal> {
  const env = detectEnvironment();

  if (env === "TAURI_DESKTOP") {
    const result = await safeInvoke<unknown>(env, "repo_context_status");
    return result.ok
      ? signalFromPayload(result.data)
      : { kind: "unavailable", reason: result.reason };
  }

  try {
    const response = await fetch("/api/repo-context", {
      headers: { Accept: "application/json" },
    });
    const payload: unknown = await response.json();
    if (!response.ok && payload && typeof payload === "object") {
      // Error bodies still carry the engine's own message — render that.
      return signalFromPayload(payload);
    }
    return signalFromPayload(payload);
  } catch (e) {
    recordDiagnostic("repoContext", String(e));
    return {
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to observe repository freshness",
    };
  }
}

function parse(data: string): unknown {
  try {
    return JSON.parse(data) as unknown;
  } catch {
    return null;
  }
}

/**
 * Watch freshness for as long as the caller stays mounted; returns the
 * unsubscribe function (call it from the effect cleanup).
 *
 * Browser: an EventSource on the stream — its initial `state` event, then
 * transitions only. A dropped connection reports `unavailable` but leaves the
 * source open, so EventSource's own retry recovers when the service returns.
 * Desktop: the same payload polled — one engine, one shape, two transports.
 */
export function watchRepoContext(
  onSignal: (signal: RepoContextSignal) => void,
): () => void {
  const env = detectEnvironment();

  if (env === "TAURI_DESKTOP") {
    let cancelled = false;
    void fetchRepoContextStatus().then((signal) => {
      if (!cancelled) onSignal(signal);
    });
    const timer = setInterval(() => {
      if (cancelled) return;
      void fetchRepoContextStatus().then((signal) => {
        if (!cancelled) onSignal(signal);
      });
    }, 1500);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }

  if (typeof EventSource === "undefined") {
    onSignal({
      kind: "unavailable",
      reason: "this runtime cannot open the repository freshness stream",
    });
    return () => {};
  }

  const source = new EventSource("/api/repo-context-stream");
  source.addEventListener("state", (event) => {
    onSignal(signalFromPayload(parse((event as MessageEvent).data)));
  });
  source.addEventListener("unavailable", (event) => {
    onSignal(signalFromPayload(parse((event as MessageEvent).data)));
  });
  source.onerror = () => {
    onSignal({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to stream repository freshness",
    });
  };
  return () => source.close();
}
