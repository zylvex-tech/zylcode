import { describe, expect, it } from "vitest";
import { drainBacklog, onMcpToolCall, safeListen, type McpToolCall } from "./events";

// ---------------------------------------------------------------------------
// Environment-aware event layer: outside the desktop runtime every
// subscription resolves to a no-op unlistener and never touches Tauri.
// ---------------------------------------------------------------------------

describe("safeListen outside the desktop runtime", () => {
  it("resolves to a callable no-op unlistener", async () => {
    const unlisten = await safeListen("zylcode://test-event", () => {});
    expect(typeof unlisten).toBe("function");
    expect(() => unlisten()).not.toThrow();
  });
});

describe("typed subscription helpers outside the desktop runtime", () => {
  it("onMcpToolCall resolves without invoking Tauri", async () => {
    let calls = 0;
    const unlisten = await onMcpToolCall(() => {
      calls += 1;
    });
    unlisten();
    expect(calls).toBe(0);
  });
});

describe("backlog drain", () => {
  it("drains once then returns empty", () => {
    drainBacklog<McpToolCall>("zylcode://drain-test"); // clear any residue
    expect(drainBacklog<McpToolCall>("zylcode://drain-test")).toEqual([]);
  });
});
