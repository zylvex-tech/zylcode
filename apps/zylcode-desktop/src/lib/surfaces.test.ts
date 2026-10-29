import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchEvidence, fetchFileTree, fetchSearch, type EvidenceState } from "./surfaces";

// ---------------------------------------------------------------------------
// Search / file tree / evidence ledger services (lib/surfaces.ts).
//
// Guarantees under test:
//   * Transport parity — desktop invokes the engine command, browser fetches
//     the matching /api route, with the query URI-encoded so a search term
//     containing `&` cannot inject a second parameter.
//   * Honest failure — every transport failure becomes an actionable
//     `unavailable` reason naming the command to start, and a diagnostic, not
//     an exception that would take the surface down.
//   * The evidence ledger keeps its four distinct states apart: `empty` is
//     not `unavailable`, and neither is `ready`.
// ---------------------------------------------------------------------------

const env = { value: "TEST" };

vi.mock("./runtime", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./runtime")>();
  return { ...actual, detectEnvironment: () => env.value };
});

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

const SEARCH = {
  query: "process_intent",
  elapsed_ms: 4,
  indexed_files: 94,
  results: [
    { resource: "src/lib/agentTurn.ts", resource_type: "file", relevance: 0.9, reason: "cites" },
  ],
};

beforeEach(() => {
  env.value = "TEST";
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.doUnmock("@tauri-apps/api/core");
});

describe("fetchSearch", () => {
  it("URI-encodes the query so metacharacters cannot inject a parameter", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(SEARCH));
    vi.stubGlobal("fetch", fetchMock);

    const state = await fetchSearch("a & b=c");

    expect(fetchMock.mock.calls[0][0]).toBe("/api/search?q=a%20%26%20b%3Dc");
    expect(state.kind).toBe("ready");
  });

  it("returns ready with the service payload", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse(SEARCH)));

    const state = await fetchSearch("process_intent");

    expect(state).toEqual({ kind: "ready", data: SEARCH });
  });

  it("reports an HTTP failure with a command the user can run", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({}, 500)));

    expect(await fetchSearch("x")).toEqual({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to search the repository",
    });
  });

  it("reports the service's error envelope as unavailable", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({ error: "index building" })));

    expect((await fetchSearch("x")).kind).toBe("unavailable");
  });

  it("reports an unreachable service as unavailable", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("ECONNREFUSED")));

    expect((await fetchSearch("x")).kind).toBe("unavailable");
  });

  it("invokes the repo_search engine command in the desktop transport", async () => {
    env.value = "TAURI_DESKTOP";
    const invoke = vi.fn().mockResolvedValue(SEARCH);
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));

    const state = await fetchSearch("process_intent");

    expect(invoke).toHaveBeenCalledWith("repo_search", { q: "process_intent" });
    expect(state).toEqual({ kind: "ready", data: SEARCH });
  });

  it("turns a desktop engine failure into a controlled unavailable state", async () => {
    env.value = "TAURI_DESKTOP";
    const invoke = vi.fn().mockRejectedValue("index corrupt");
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));

    expect(await fetchSearch("x")).toEqual({
      kind: "unavailable",
      reason: "desktop engine search failed",
    });
  });
});

describe("fetchFileTree", () => {
  it("returns ready with the scanned tree", async () => {
    const tree = {
      total_scanned: 94,
      truncated: false,
      elapsed_ms: 12,
      rows: [{ path: "src/App.tsx", language: "tsx" }],
    };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse(tree)));

    expect(await fetchFileTree()).toEqual({ kind: "ready", data: tree });
  });

  it("reports failure with an actionable reason", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("down")));

    expect(await fetchFileTree()).toEqual({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to browse the repository tree",
    });
  });

  it("invokes repo_file_tree with no arguments in the desktop transport", async () => {
    env.value = "TAURI_DESKTOP";
    const invoke = vi.fn().mockResolvedValue({ total_scanned: 1, rows: [] });
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));

    const state = await fetchFileTree();

    expect(invoke).toHaveBeenCalledWith("repo_file_tree", undefined);
    expect(state.kind).toBe("ready");
  });

  it("reports a desktop engine failure without invoking the browser fallback", async () => {
    env.value = "TAURI_DESKTOP";
    const invoke = vi.fn().mockRejectedValue("scanner died");
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    expect(await fetchFileTree()).toEqual({
      kind: "unavailable",
      reason: "desktop engine file tree failed",
    });
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

describe("fetchEvidence", () => {
  it("preserves the ready state including chain integrity", async () => {
    const payload = {
      kind: "ready",
      chain_intact: true,
      session_count: 2,
      entry_count: 5,
      entries: [],
    };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse(payload)));

    expect(await fetchEvidence()).toEqual(payload);
  });

  it("preserves an empty ledger as `empty`, not as an error", async () => {
    const payload = { kind: "empty", reason: "no evidence recorded yet" };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse(payload)));

    expect(await fetchEvidence()).toEqual(payload);
  });

  it("distinguishes a broken chain from a transport failure", async () => {
    const payload = {
      kind: "ready",
      chain_intact: false,
      session_count: 1,
      entry_count: 3,
      entries: [{ id: "e1", session_id: "s1" }],
    };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse(payload)));

    const state: EvidenceState = await fetchEvidence();
    expect(state.kind).toBe("ready");
    if (state.kind === "ready") expect(state.chain_intact).toBe(false);
  });

  it("reports an unreachable service as unavailable", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("down")));

    expect(await fetchEvidence()).toEqual({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to read the evidence ledger",
    });
  });

  it("reports an error envelope as unavailable rather than returning it as a state", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({ error: "ledger corrupt" })));

    expect((await fetchEvidence()).kind).toBe("unavailable");
  });

  it("passes the error envelope through in the desktop transport as unavailable", async () => {
    env.value = "TAURI_DESKTOP";
    const invoke = vi.fn().mockRejectedValue("hash chain broken");
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));

    expect(await fetchEvidence()).toEqual({
      kind: "unavailable",
      reason: "desktop engine could not read the evidence ledger",
    });
  });
});
