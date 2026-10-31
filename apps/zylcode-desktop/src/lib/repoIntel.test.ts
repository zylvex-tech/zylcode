import { beforeEach, describe, expect, it, vi } from "vitest";
import { fetchRepoIntel } from "./repoIntel";
import { detectEnvironment } from "./runtime";

// ---------------------------------------------------------------------------
// repoIntel has exactly two transports and one honesty rule: a failure never
// invents data, it resolves to a controlled `unavailable` state. Each transport
// and each failure mode is pinned here, because the alternative — a panel that
// silently renders fabricated repository data — is the failure this module
// exists to prevent.
// ---------------------------------------------------------------------------

vi.mock("./runtime", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./runtime")>();
  return { ...actual, detectEnvironment: vi.fn() };
});

const env = vi.mocked(detectEnvironment);
const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

const PAYLOAD = {
  task: "map the repo",
  elapsed_ms: 12,
  summary: {
    file_count: 1,
    symbol_count: 2,
    package_count: 1,
    packages: ["zylcode-core"],
    languages: ["Rust"],
    entry_points: ["crates/zylcode-cli/src/main.rs"],
    architecture_facts: [{ fact: "workspace", value: "cargo" }],
    recent_changes: [{ short_id: "abc1234", message: "wip", author: "founder" }],
  },
  results: [
    { resource: "crates/zylcode-core", resource_type: "dir", relevance: 0.9, reason: "core" },
  ],
};

function jsonResponse(body: unknown, ok = true, status = 200) {
  return {
    ok,
    status,
    json: async () => body,
  } as Response;
}

beforeEach(() => {
  vi.clearAllMocks();
  invoke.mockResolvedValue(PAYLOAD);
  vi.stubGlobal("fetch", vi.fn());
});

describe("fetchRepoIntel", () => {
  it("uses the in-process desktop engine when running under Tauri", async () => {
    env.mockReturnValue("TAURI_DESKTOP");
    const state = await fetchRepoIntel("map the repo");

    expect(invoke).toHaveBeenCalledWith("repo_context", { task: "map the repo" });
    expect(state).toEqual({ kind: "ready", data: PAYLOAD, via: "desktop" });
    expect(globalThis.fetch).not.toHaveBeenCalled();
  });

  it("degrades to unavailable when the desktop engine fails", async () => {
    env.mockReturnValue("TAURI_DESKTOP");
    invoke.mockRejectedValue(new Error("engine offline"));

    const state = await fetchRepoIntel("map the repo");

    expect(state).toEqual({
      kind: "unavailable",
      reason: "desktop engine could not build the intelligence payload",
    });
  });

  it("falls back to the serve-intel service in browser preview", async () => {
    env.mockReturnValue("BROWSER_PREVIEW");
    vi.mocked(globalThis.fetch).mockResolvedValue(jsonResponse(PAYLOAD));

    const state = await fetchRepoIntel("map the repo");

    expect(globalThis.fetch).toHaveBeenCalledWith("/api/repo-intel?task=map%20the%20repo", {
      headers: { Accept: "application/json" },
    });
    expect(state).toEqual({ kind: "ready", data: PAYLOAD, via: "service" });
    expect(invoke).not.toHaveBeenCalled();
  });

  it("treats a non-2xx service response as unavailable, not as data", async () => {
    env.mockReturnValue("BROWSER_PREVIEW");
    vi.mocked(globalThis.fetch).mockResolvedValue(jsonResponse({}, false, 503));

    const state = await fetchRepoIntel("t");

    expect(state.kind).toBe("unavailable");
    expect(state).toEqual({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to see live repository data",
    });
  });

  it("unwraps the service's controlled error envelope", async () => {
    env.mockReturnValue("BROWSER_PREVIEW");
    vi.mocked(globalThis.fetch).mockResolvedValue(jsonResponse({ error: "index is stale" }));

    const state = await fetchRepoIntel("t");

    expect(state.kind).toBe("unavailable");
    expect(state).toEqual({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to see live repository data",
    });
  });

  it("handles a rejected fetch (service not running at all)", async () => {
    env.mockReturnValue("BROWSER_PREVIEW");
    vi.mocked(globalThis.fetch).mockRejectedValue(new TypeError("Failed to fetch"));

    const state = await fetchRepoIntel("t");

    expect(state.kind).toBe("unavailable");
  });

  it("percent-encodes the task so query metacharacters cannot break the URL", async () => {
    env.mockReturnValue("BROWSER_PREVIEW");
    vi.mocked(globalThis.fetch).mockResolvedValue(jsonResponse(PAYLOAD));

    await fetchRepoIntel("a&b=c?d#e");

    expect(vi.mocked(globalThis.fetch).mock.calls[0][0]).toBe(
      "/api/repo-intel?task=a%26b%3Dc%3Fd%23e",
    );
  });
});
