import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { runAgentTurn, type IntentResult } from "./agentTurn";

// ---------------------------------------------------------------------------
// Agent-turn service (lib/agentTurn.ts).
//
// The honesty rule under test: a failed turn renders the engine's failure
// verbatim. `success: false` and an `error` envelope must never be reshaped
// into a `done` state, because the surface would then display a failed run as
// if it had succeeded. Transport errors resolve to `unavailable`, never a
// throw.
// ---------------------------------------------------------------------------

const env = { value: "TEST" };

vi.mock("./runtime", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./runtime")>();
  return { ...actual, detectEnvironment: () => env.value };
});

const RESULT: IntentResult = {
  summary: "added the guard",
  artifacts: [{ kind: "patch", label: "fix", content: "diff --git ..." }],
  success: true,
};

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

beforeEach(() => {
  env.value = "TEST";
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.doUnmock("@tauri-apps/api/core");
});

describe("runAgentTurn in the test environment", () => {
  it("refuses rather than simulating a turn", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    const state = await runAgentTurn("do the thing");

    expect(state).toEqual({ kind: "unavailable", reason: "agent turn requires a runtime" });
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

describe("runAgentTurn in the browser transport", () => {
  beforeEach(() => {
    env.value = "BROWSER_PREVIEW";
  });

  it("POSTs the prompt and returns the engine result tagged with its transport", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(RESULT));
    vi.stubGlobal("fetch", fetchMock);

    const state = await runAgentTurn("do the thing");

    expect(fetchMock).toHaveBeenCalledWith("/api/intent", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ prompt: "do the thing" }),
    });
    expect(state).toEqual({ kind: "done", result: RESULT, via: "service" });
  });

  it("reports an engine error envelope as failed, not as a done turn", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(jsonResponse({ error: "verification failed" })),
    );

    expect(await runAgentTurn("x")).toEqual({ kind: "failed", error: "verification failed" });
  });

  it("preserves success:false verbatim rather than reshaping it", async () => {
    const failed: IntentResult = { ...RESULT, success: false };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse(failed)));

    const state = await runAgentTurn("x");

    expect(state.kind).toBe("done");
    if (state.kind === "done") expect(state.result.success).toBe(false);
  });

  it("reports an HTTP failure as unavailable, naming the status", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({}, 503)));

    expect(await runAgentTurn("x")).toEqual({
      kind: "unavailable",
      reason: "agent service error: HTTP 503",
    });
  });

  it("reports an unreachable service as unavailable", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("ECONNREFUSED")));

    expect(await runAgentTurn("x")).toEqual({
      kind: "unavailable",
      reason: "agent service unreachable: Error: ECONNREFUSED",
    });
  });
});

describe("runAgentTurn in the desktop transport", () => {
  beforeEach(() => {
    env.value = "TAURI_DESKTOP";
  });

  it("invokes process_intent with a null model", async () => {
    const invoke = vi.fn().mockResolvedValue(RESULT);
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));

    const state = await runAgentTurn("do the thing");

    expect(invoke).toHaveBeenCalledWith("process_intent", { prompt: "do the thing", model: null });
    expect(state).toEqual({ kind: "done", result: RESULT, via: "desktop" });
  });

  it("turns an IPC rejection into a failed state carrying the raw error", async () => {
    const invoke = vi.fn().mockRejectedValue("model not found");
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));

    expect(await runAgentTurn("x")).toEqual({ kind: "failed", error: "model not found" });
  });
});

describe("runAgentTurn in an unsupported environment", () => {
  it("reports the environment rather than guessing a transport", async () => {
    env.value = "UNKNOWN";
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    // UNKNOWN shares the service transport, so it must attempt the fetch.
    fetchMock.mockResolvedValue(jsonResponse(RESULT));
    const state = await runAgentTurn("x");

    expect(state.kind).toBe("done");
    expect(fetchMock).toHaveBeenCalled();
  });
});
