// Repository-navigation data path: the two transports, the refusal /
// unavailable split, and the pure helpers the panel leans on.
//
// The rule under test: an engine that could not answer must never surface as
// an empty result set, because "nothing references this" and "we could not
// ask" are different facts.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  byEvidenceThen,
  classifyNavFailure,
  filterByEvidence,
  formatLocation,
  groupByFile,
  matchesQuery,
  queryNav,
  type Evidence,
  type NavReferenceResult,
} from "./navIntel";

// `queryNav` imports `@tauri-apps/api/core` dynamically, so the module is
// mocked once here and asserted per transport below.
const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const RANGE = {
  start_byte: 0,
  end_byte: 4,
  start_line: 3,
  start_col: 0,
  end_line: 3,
  end_col: 4,
};

const OK_PAYLOAD: NavReferenceResult = {
  engine: "zylcode-nav",
  definition: null,
  references: [],
  possible: [],
  unresolved: null,
  deterministic_count: 0,
  heuristic_count: 0,
};

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  fetchMock = vi.fn();
  vi.stubGlobal("fetch", fetchMock);
  invokeMock.mockReset();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------
// queryNav
// ---------------------------------------------------------------------------

describe("queryNav (service transport)", () => {
  it("returns ready with the engine payload verbatim", async () => {
    fetchMock.mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => OK_PAYLOAD,
    });

    const state = await queryNav<NavReferenceResult>("find_references", { name: "helper" });

    expect(state.kind).toBe("ready");
    if (state.kind !== "ready") throw new Error("unreachable");
    expect(state.via).toBe("service");
    expect(state.data.engine).toBe("zylcode-nav");

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("/api/nav");
    expect(init.method).toBe("POST");
    expect(JSON.parse(String(init.body))).toEqual({
      tool: "find_references",
      args: { name: "helper" },
    });
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("treats an engine refusal as `rejected`, never as an empty answer", async () => {
    fetchMock.mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        error: "repository navigation failed: request rejected: a selector is required",
        engine: "zylcode-nav",
      }),
    });

    const state = await queryNav("find_references", {});

    expect(state.kind).toBe("rejected");
    if (state.kind !== "rejected") throw new Error("unreachable");
    expect(state.reason).toContain("could not answer this query");
  });

  it("treats an unreachable engine as `unavailable`, never as an empty answer", async () => {
    fetchMock.mockRejectedValue(new Error("connection refused"));

    const state = await queryNav("find_references", { name: "helper" });

    expect(state.kind).toBe("unavailable");
    if (state.kind !== "unavailable") throw new Error("unreachable");
    expect(state.reason).toContain("serve-intel");
  });

  it("treats a non-2xx from the service as `unavailable`", async () => {
    fetchMock.mockResolvedValue({ ok: false, status: 503, json: async () => ({}) });

    const state = await queryNav("repository_graph_query", {});

    expect(state.kind).toBe("unavailable");
  });
});

describe("classifyNavFailure", () => {
  it("separates the engine's own two failure vocabularies", () => {
    expect(classifyNavFailure("repository navigation failed: request rejected: no selector")).toBe(
      "rejected",
    );
    expect(
      classifyNavFailure("repository navigation failed: repository index unavailable: no such dir"),
    ).toBe("unavailable");
    // Anything unrecognised is treated as infrastructure, not as a refusal —
    // the conservative reading: an unknown failure was not a deliberate answer.
    expect(classifyNavFailure("spawn failed")).toBe("unavailable");
  });
});

// ---------------------------------------------------------------------------
// Presentation helpers
// ---------------------------------------------------------------------------

describe("formatLocation", () => {
  it("renders 1-based line:col the way the editor does", () => {
    expect(formatLocation(RANGE)).toBe("L3:C0");
  });
});

describe("matchesQuery", () => {
  it("is case-insensitive and treats a blank filter as 'everything'", () => {
    expect(matchesQuery("src/Lib.RS", "lib.rs")).toBe(true);
    expect(matchesQuery("src/lib.rs", "   ")).toBe(true);
    expect(matchesQuery("src/lib.rs", "api")).toBe(false);
  });
});

describe("evidence ordering", () => {
  const rows = [
    { id: "h", evidence: "HEURISTIC" as Evidence },
    { id: "u", evidence: "UNSUPPORTED" as Evidence },
    { id: "d", evidence: "DETERMINISTIC" as Evidence },
  ];

  it("always leads with the strongest evidence", () => {
    expect([...rows].sort(byEvidenceThen).map((r) => r.id)).toEqual(["d", "h", "u"]);
  });

  it("narrows to exactly one label without reordering the rest", () => {
    expect(filterByEvidence(rows, "DETERMINISTIC").map((r) => r.id)).toEqual(["d"]);
    expect(filterByEvidence(rows, "all")).toHaveLength(3);
    expect(filterByEvidence(rows, "HEURISTIC")).toHaveLength(1);
  });

  it("does not mutate the input when filtering", () => {
    const input = [...rows];
    filterByEvidence(input, "DETERMINISTIC");
    expect(input.map((r) => r.id)).toEqual(["h", "u", "d"]);
  });
});

describe("groupByFile", () => {
  it("keeps first-seen order and every row exactly once", () => {
    const grouped = groupByFile([{ file: "b.rs" }, { file: "a.rs" }, { file: "b.rs" }]);
    expect(grouped.map((g) => g.file)).toEqual(["b.rs", "a.rs"]);
    expect(grouped[0].rows).toHaveLength(2);
    expect(grouped[1].rows).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// Transport selection (G2)
//
// Two transports, one data path. A suite that exercises only one of them
// cannot notice a change that collapses the two, so each branch asserts the
// *other* transport was not used: a Tauri desktop query reaches the engine
// through `nav_query` IPC and never through HTTP; a non-desktop query reaches
// it through `fetch("/api/nav")` and never through IPC.
// ---------------------------------------------------------------------------

function asTauriDesktop(): void {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
    transformCallback: () => 0,
  };
}

function asNonDesktop(): void {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
}

describe("queryNav (transport selection)", () => {
  it("routes a Tauri desktop query through `nav_query` IPC and never through HTTP", async () => {
    asTauriDesktop();
    try {
      invokeMock.mockResolvedValue(OK_PAYLOAD);

      const state = await queryNav<NavReferenceResult>("find_references", { name: "helper" });

      expect(state.kind).toBe("ready");
      if (state.kind !== "ready") throw new Error("unreachable");
      expect(state.via).toBe("desktop");
      expect(invokeMock).toHaveBeenCalledWith("nav_query", {
        tool: "find_references",
        args: { name: "helper" },
      });
      expect(fetchMock).not.toHaveBeenCalled();
    } finally {
      asNonDesktop();
    }
  });

  it("classifies an IPC refusal as `rejected` rather than an empty answer", async () => {
    asTauriDesktop();
    try {
      invokeMock.mockRejectedValue(
        "repository navigation failed: request rejected: a selector is required",
      );

      const state = await queryNav("find_references", {});

      expect(state.kind).toBe("rejected");
      if (state.kind !== "rejected") throw new Error("unreachable");
      expect(state.reason).toContain("could not answer this query");
      expect(fetchMock).not.toHaveBeenCalled();
    } finally {
      asNonDesktop();
    }
  });

  it('routes a non-desktop query through fetch("/api/nav") and never through IPC', async () => {
    asNonDesktop();
    fetchMock.mockResolvedValue({ ok: true, status: 200, json: async () => OK_PAYLOAD });

    const state = await queryNav<NavReferenceResult>("find_callers", { name: "helper" });

    expect(state.kind).toBe("ready");
    if (state.kind !== "ready") throw new Error("unreachable");
    expect(state.via).toBe("service");
    expect(fetchMock.mock.calls[0]?.[0]).toBe("/api/nav");
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
