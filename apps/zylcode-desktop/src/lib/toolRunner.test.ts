// Contract tests for the tool runner service (lib/toolRunner.ts).
//
// What is pinned: an id the catalogue says has no executor is declined
// *without* touching any transport (fail-closed is the product contract); a
// real engine failure is classified `failed` — never re-cast as an OK result
// or an empty list; a network-level failure is `unavailable` because nothing
// answered; and the null `error` path in the HTTP payload surface is
// reported from the engine's own output.error where present.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { invoke } from "@tauri-apps/api/core";
import { clearDiagnostics, getDiagnostics } from "./runtime";
import { runTool, type ToolRunState } from "./toolRunner";
import type { ToolRow } from "./service";

// The desktop transport is reached through a dynamic import, so the Tauri
// module has to be stubbed for that branch to be exercisable in jsdom.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const BOUND: ToolRow[] = [
  {
    id: "fs.read",
    transport: "stdio",
    description: "Read a file",
    enabled: true,
    executor_bound: true,
  },
  { id: "db.query", transport: "stdio", description: null, enabled: true, executor_bound: false },
];

describe("runTool — refusal before transport", () => {
  it("declines an id without a bound executor and never dispatches it", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    const state = await runTool("db.query", {}, BOUND);
    expect(state.kind).toBe("declined");
    if (state.kind === "declined") {
      expect(state.reason).toMatch(/no executor/);
    }
    expect(fetchMock).not.toHaveBeenCalled();
    vi.unstubAllGlobals();
  });

  it("declines non-object arguments", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    const state = await runTool("fs.read", "not-an-object", BOUND);
    expect(state.kind).toBe("declined");
    expect(fetchMock).not.toHaveBeenCalled();
    vi.unstubAllGlobals();
  });
});

describe("runTool — the engine's word is rendered verbatim", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        Promise.resolve({
          json: async () => ({ error: "no real executor is bound to 'x' — fails closed" }),
        }),
      ) as unknown as typeof fetch,
    );
  });
  afterEach(() => vi.unstubAllGlobals());

  it("a backend error payload is classified failed with the engine message", async () => {
    const state = await runTool("fs.read", {}, BOUND);
    expect(state.kind).toBe("failed");
    if (state.kind === "failed") {
      expect(state.via).toBe("service");
      expect(state.error).toMatch(/fails closed/);
    }
  });
});

describe("runTool — success and tool-level failure payloads", () => {
  it("a success=false result with output.error renders that error", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        Promise.resolve({
          json: async () => ({
            tool: "fs.read",
            executed: true,
            success: false,
            output: { error: "path must not contain '..'" },
            bound_operation: "fs.read",
          }),
        }),
      ) as unknown as typeof fetch,
    );
    const state: ToolRunState = await runTool("fs.read", {}, BOUND);
    expect(state.kind).toBe("failed");
    if (state.kind === "failed") expect(state.error).toMatch(/\.\./);
    vi.unstubAllGlobals();
  });

  it("a real success arrives as done with the payload intact", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        Promise.resolve({
          json: async () => ({
            tool: "fs.read",
            executed: true,
            success: true,
            output: { path: "README.md", content: "# ZylCode" },
            invocation_id: "ev-1",
          }),
        }),
      ) as unknown as typeof fetch,
    );
    const state: ToolRunState = await runTool("fs.read", {}, BOUND);
    expect(state.kind).toBe("done");
    if (state.kind === "done") {
      expect(state.via).toBe("service");
      expect(state.result.invocation_id).toBe("ev-1");
    }
    vi.unstubAllGlobals();
  });

  it("a transport-level failure resolves to unavailable, not failed", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        Promise.reject(new TypeError("Failed to fetch")),
      ) as unknown as typeof fetch,
    );
    const state: ToolRunState = await runTool("fs.read", {}, BOUND);
    expect(state.kind).toBe("unavailable");
    if (state.kind === "unavailable") expect(state.reason).toMatch(/serve-intel/);
    vi.unstubAllGlobals();
  });
});

describe("runTool — a payload the engine failed without a message", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("names the bound operation rather than inventing a reason", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        Promise.resolve({
          json: async () => ({
            tool: "fs.write",
            executed: true,
            success: false,
            bound_operation: "fs.write",
          }),
        }),
      ) as unknown as typeof fetch,
    );

    const state = await runTool("fs.write", {}, BOUND);

    expect(state.kind).toBe("failed");
    if (state.kind === "failed") {
      expect(state.error).toBe("the tool reported failure (operation: fs.write)");
    }
  });
});

// ---------------------------------------------------------------------------
// Desktop transport: `execute_tool` through Tauri IPC. The environment is
// forced to TAURI_DESKTOP by injecting Tauri's internals global, which is
// exactly the signal `detectEnvironment` looks for.
// ---------------------------------------------------------------------------
describe("runTool — desktop IPC transport", () => {
  const invokeMock = vi.mocked(invoke);
  const internals = globalThis as Record<string, unknown>;

  beforeEach(() => {
    clearDiagnostics();
    invokeMock.mockReset();
    internals.__TAURI_INTERNALS__ = {};
  });

  afterEach(() => {
    delete internals.__TAURI_INTERNALS__;
    vi.unstubAllGlobals();
  });

  it("returns the engine payload tagged desktop and skips HTTP entirely", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    invokeMock.mockResolvedValue({
      tool: "fs.read",
      executed: true,
      success: true,
      output: { content: "# ZylCode" },
      invocation_id: "ev-9",
    });

    const state = await runTool("fs.read", { path: "README.md" }, BOUND);

    expect(state.kind).toBe("done");
    if (state.kind === "done") {
      expect(state.via).toBe("desktop");
      expect(state.result.invocation_id).toBe("ev-9");
    }
    expect(invokeMock).toHaveBeenCalledWith("execute_tool", {
      id: "fs.read",
      params: { path: "README.md" },
    });
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("surfaces a desktop success=false with the tool's own error", async () => {
    invokeMock.mockResolvedValue({
      tool: "fs.read",
      executed: true,
      success: false,
      output: { error: "path escapes the workspace root" },
    });

    const state = await runTool("fs.read", {}, BOUND);

    expect(state.kind).toBe("failed");
    if (state.kind === "failed") {
      expect(state.via).toBe("desktop");
      expect(state.error).toMatch(/escapes the workspace root/);
    }
  });

  it("falls back to the engine's phrasing when IPC reports no message", async () => {
    invokeMock.mockResolvedValue({
      tool: "fs.read",
      executed: true,
      success: false,
      bound_operation: "fs.read",
    });

    const state = await runTool("fs.read", {}, BOUND);

    expect(state.kind).toBe("failed");
    if (state.kind === "failed") {
      expect(state.error).toBe("the tool reported failure (operation: fs.read)");
    }
  });

  it("classifies a rejected IPC call as a desktop failure with a diagnostic", async () => {
    invokeMock.mockRejectedValue(new Error("engine registry unavailable"));

    const state = await runTool("fs.read", {}, BOUND);

    expect(state.kind).toBe("failed");
    if (state.kind === "failed") {
      expect(state.via).toBe("desktop");
      expect(state.error).toMatch(/engine registry unavailable/);
    }
    expect(getDiagnostics().some((d) => d.source === "toolRunner")).toBe(true);
  });
});
