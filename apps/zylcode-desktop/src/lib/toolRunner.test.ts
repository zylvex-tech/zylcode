// Contract tests for the tool runner service (lib/toolRunner.ts).
//
// What is pinned: an id the catalogue says has no executor is declined
// *without* touching any transport (fail-closed is the product contract); a
// real engine failure is classified `failed` — never re-cast as an OK result
// or an empty list; a network-level failure is `unavailable` because nothing
// answered; and the null `error` path in the HTTP payload surface is
// reported from the engine's own output.error where present.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { runTool, type ToolRunState } from "./toolRunner";
import type { ToolRow } from "./service";

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
