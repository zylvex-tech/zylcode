// The References / Call hierarchy / Dependency graph / Impact surface.
//
// What is being guarded here is not styling: it is that the panel renders
// the engine's evidence label verbatim, that an unresolved selector is shown
// as unresolved rather than as an empty list, and that a refused query is
// shown as a refusal rather than as "no references found".

import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RepoNavPanel, NavErrorBoundary } from "./RepoNavPanel";
import {
  type NavCallHierarchyResult,
  type NavGraphResult,
  type NavImpactResult,
  type NavReferenceResult,
  type NavState,
} from "../lib/navIntel";

vi.mock("../lib/navIntel", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/navIntel")>();
  return { ...actual, queryNav: vi.fn() };
});

import { queryNav } from "../lib/navIntel";

const queryNavMock = vi.mocked(queryNav);

const RANGE = {
  start_byte: 0,
  end_byte: 6,
  start_line: 3,
  start_col: 0,
  end_line: 3,
  end_col: 6,
};

const REFERENCES: NavReferenceResult = {
  engine: "zylcode-nav",
  definition: {
    id: "src/lib.rs#crate::helper",
    file: "src/lib.rs",
    qualified_name: "crate::helper",
    name: "helper",
    kind: "function",
    range: RANGE,
    name_range: RANGE,
    container: "crate",
    signature: "pub fn helper() -> u32",
    exported: true,
    module: "crate",
  },
  references: [
    {
      file: "src/lib.rs",
      range: RANGE,
      kind: "definition",
      evidence: "DETERMINISTIC",
      excerpt: "pub fn helper() -> u32 {",
      via: "definition",
    },
    {
      file: "src/api.rs",
      range: { ...RANGE, start_line: 12 },
      kind: "call",
      evidence: "DETERMINISTIC",
      excerpt: "    helper()",
      via: "module scope",
    },
  ],
  possible: [
    {
      file: "src/notes.md",
      range: { ...RANGE, start_line: 40 },
      kind: "read",
      evidence: "HEURISTIC",
      excerpt: "see the helper",
      via: "name only",
    },
  ],
  unresolved: null,
  deterministic_count: 2,
  heuristic_count: 1,
};

const CALLERS: NavCallHierarchyResult = {
  engine: "zylcode-nav",
  target: REFERENCES.definition,
  relations: [
    {
      site: {
        file: "src/api.rs",
        range: { ...RANGE, start_line: 12 },
        callee_range: RANGE,
        callee_name: "helper",
        callee_path: "helper",
        shape: "plain",
        enclosing: "crate::api::run",
        excerpt: "    helper()",
      },
      resolved: REFERENCES.definition,
      evidence: "DETERMINISTIC",
      note: "resolved from the module scope chain",
    },
  ],
  unresolved: null,
  deterministic_count: 1,
  heuristic_count: 0,
  unsupported_count: 0,
};

const IMPACT: NavImpactResult = {
  engine: "zylcode-nav",
  subject: { kind: "file", path: "src/lib.rs" },
  items: [
    {
      class: "DIRECT_DEPENDENT",
      target_kind: "file",
      id: "src/api.rs",
      label: "src/api.rs",
      evidence: "DETERMINISTIC",
      provenance: "parsed import edge",
      via: ["src/lib.rs", "src/api.rs"],
    },
  ],
  direct_count: 1,
  transitive_count: 0,
  possible_count: 0,
  unresolved_count: 0,
  summary: "1 direct dependent within depth 2.",
  unresolved_subject: null,
};

const GRAPH: NavGraphResult = {
  engine: "zylcode-nav",
  nodes: [],
  edges: [
    {
      from: { kind: "file", id: "src/lib.rs", label: "src/lib.rs" },
      to: { kind: "file", id: "src/util.rs", label: "src/util.rs" },
      kind: "imports",
      provenance: "parsed",
      evidence: "DETERMINISTIC",
      detail: "src/lib.rs --imports--> src/util.rs",
      source_file: "src/lib.rs",
      excerpt: "use util::*;",
    },
    {
      from: { kind: "file", id: "src/lib.rs", label: "src/lib.rs" },
      to: { kind: "symbol", id: "src/other.rs#helper", label: "helper" },
      kind: "references",
      provenance: "inference",
      evidence: "HEURISTIC",
      detail: "src/lib.rs --references--> helper",
    },
  ],
  cycles: [["src/a.ts", "src/b.ts"]],
  truncated: false,
  total_edges_matched: 2,
};

async function flush() {
  await act(async () => {
    for (let i = 0; i < 10; i++) await Promise.resolve();
  });
}

function runQuery() {
  fireEvent.click(screen.getByRole("button", { name: /Analyze|Querying/ }));
}

async function submitSymbol(symbol: string) {
  fireEvent.change(screen.getByLabelText("Symbol name"), { target: { value: symbol } });
  runQuery();
  await flush();
}

/**
 * A promise whose resolution the test controls, so overlapping queries can be
 * resolved in *any* order — including the order that used to blank the window.
 */
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

type NavOutcome = NavState<unknown>;

beforeEach(() => {
  queryNavMock.mockReset();
  queryNavMock.mockImplementation(async (tool: string) => {
    if (tool === "repository_graph_query") {
      return { kind: "ready", data: GRAPH, via: "service" } as const;
    }
    if (tool === "find_callers" || tool === "find_callees") {
      return { kind: "ready", data: CALLERS, via: "service" } as const;
    }
    return { kind: "ready", data: REFERENCES, via: "service" } as const;
  });
});

describe("RepoNavPanel", () => {
  it("does not query the index until the user asks it to", () => {
    render(<RepoNavPanel onNavigateToFile={vi.fn()} />);

    expect(screen.getByText(/answers come from the parsed repository index/)).toBeInTheDocument();
    expect(queryNavMock).not.toHaveBeenCalled();
  });

  it("refuses to run — rather than silently doing nothing — when no selector is given", async () => {
    render(<RepoNavPanel onNavigateToFile={vi.fn()} />);
    runQuery();
    await flush();

    expect(queryNavMock).not.toHaveBeenCalled();
    expect(screen.getByText(/The engine refused this query/)).toBeInTheDocument();
    expect(screen.queryByText(/DETERMINISTIC/)).not.toBeInTheDocument();
  });

  it("renders references with the engine's evidence labels and click-to-navigate", async () => {
    const openFile = vi.fn();
    render(<RepoNavPanel onNavigateToFile={openFile} />);

    await submitSymbol("helper");

    expect(queryNavMock).toHaveBeenCalledWith(
      "find_references",
      expect.objectContaining({ name: "helper", limit: 500 }),
    );
    expect(screen.getByText("2 deterministic")).toBeInTheDocument();
    expect(screen.getByText("1 heuristic")).toBeInTheDocument();
    expect(screen.getAllByText("DETERMINISTIC")).toHaveLength(2);
    expect(screen.getByText("HEURISTIC")).toBeInTheDocument();

    fireEvent.click(screen.getByTitle("Open src/api.rs"));
    expect(openFile).toHaveBeenCalledWith("src/api.rs");
  });

  it("lets the reviewer point at an exact file/line and narrow the rendered rows", async () => {
    const openFile = vi.fn();
    render(<RepoNavPanel onNavigateToFile={openFile} />);

    // The file/line selectors belong to the user, and stay editable.
    fireEvent.change(screen.getByLabelText("Repository-relative file"), {
      target: { value: "src/lib.rs" },
    });
    fireEvent.change(screen.getByLabelText("Line number"), { target: { value: "42" } });

    await submitSymbol("helper");

    // The rendered rows can be narrowed by a free-text filter.
    const filter = screen.getByLabelText("Filter results");
    fireEvent.change(filter, { target: { value: "api" } });
    expect(screen.queryByText("HEURISTIC")).not.toBeInTheDocument();
    fireEvent.change(filter, { target: { value: "" } });
    expect(screen.getByText("HEURISTIC")).toBeInTheDocument();

    // The definition itself is a navigation target, not just a heading.
    const definition = screen.getByText(/src\/lib\.rs:L\d+:C\d+/).closest("button");
    expect(definition).not.toBeNull();
    fireEvent.click(definition as HTMLElement);
    expect(openFile).toHaveBeenCalledWith("src/lib.rs");
  });

  it("navigates from a call in the hierarchy back to the file that holds it", async () => {
    const openFile = vi.fn();
    render(<RepoNavPanel onNavigateToFile={openFile} />);
    await submitSymbol("helper");

    fireEvent.click(screen.getByRole("tab", { name: "Call hierarchy" }));
    await flush();

    expect(screen.getByText("1 deterministic")).toBeInTheDocument();
    fireEvent.click(screen.getByTitle("Open src/api.rs"));
    expect(openFile).toHaveBeenCalledWith("src/api.rs");
  });

  it("opens a source edge in the editor and refuses a non-source edge", async () => {
    const openFile = vi.fn();
    render(<RepoNavPanel onNavigateToFile={openFile} />);
    await submitSymbol("helper");

    fireEvent.click(screen.getByRole("tab", { name: "Dependency graph" }));
    await flush();

    expect(screen.getByText("2 of 2 edges")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Edge kind filter"), {
      target: { value: "references" },
    });
    expect(screen.getByText("1 of 2 edges")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Edge kind filter"), { target: { value: "all" } });

    fireEvent.click(screen.getByTitle("Open src/util.rs"));
    expect(openFile).toHaveBeenCalledWith("src/util.rs");

    fireEvent.click(screen.getByTitle("Not a source file — no navigation"));
    expect(openFile).toHaveBeenCalledTimes(1);
  });

  it("navigates from an impact row back to the dependent file", async () => {
    const openFile = vi.fn();
    render(<RepoNavPanel onNavigateToFile={openFile} />);
    await submitSymbol("helper");

    queryNavMock.mockResolvedValueOnce({ kind: "ready", data: IMPACT, via: "service" } as const);
    fireEvent.click(screen.getByRole("tab", { name: "Impact" }));
    await flush();

    expect(screen.getByText("1 direct dependent within depth 2.")).toBeInTheDocument();
    fireEvent.click(screen.getByTitle("Open src/api.rs"));
    expect(openFile).toHaveBeenCalledWith("src/api.rs");
  });

  it("narrows to a single evidence class when the reviewer asks for one", async () => {
    render(<RepoNavPanel onNavigateToFile={vi.fn()} />);
    await submitSymbol("helper");

    fireEvent.change(screen.getByLabelText("Evidence filter"), {
      target: { value: "DETERMINISTIC" },
    });

    expect(screen.queryByText("HEURISTIC")).not.toBeInTheDocument();
    expect(screen.getAllByText("DETERMINISTIC")).toHaveLength(2);
  });

  it("shows an unresolved selector as unresolved, never as an empty result", async () => {
    queryNavMock.mockResolvedValueOnce({
      kind: "ready",
      data: {
        ...REFERENCES,
        definition: null,
        references: [],
        possible: [],
        unresolved: "no symbol named `nope` in this repository",
      } satisfies NavReferenceResult,
      via: "service",
    } as const);

    render(<RepoNavPanel onNavigateToFile={vi.fn()} />);
    await submitSymbol("nope");

    expect(
      screen.getByText("Unresolved: no symbol named `nope` in this repository"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/DETERMINISTIC/)).not.toBeInTheDocument();
    expect(screen.queryByText(/deterministic\)/)).not.toBeInTheDocument();
  });

  it("shows a refused query as a refusal, never as 'no references found'", async () => {
    queryNavMock.mockResolvedValueOnce({
      kind: "rejected",
      reason: "the repository could not answer this query with the given selector",
    } as const);

    render(<RepoNavPanel onNavigateToFile={vi.fn()} />);
    await submitSymbol("helper");

    expect(screen.getByText(/The engine refused this query/)).toBeInTheDocument();
    expect(screen.queryByText(/2 deterministic/)).not.toBeInTheDocument();
    expect(screen.queryByText(/No reference matched/)).not.toBeInTheDocument();
  });

  it("shows an unreachable index as unavailable, never as an empty list", async () => {
    queryNavMock.mockResolvedValueOnce({
      kind: "unavailable",
      reason: "the repository index could not be built for this workspace",
    } as const);

    render(<RepoNavPanel onNavigateToFile={vi.fn()} />);
    await submitSymbol("helper");

    expect(
      screen.getByText("the repository index could not be built for this workspace"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/No reference matched/)).not.toBeInTheDocument();
  });

  it("re-runs the query against the newly selected view", async () => {
    render(<RepoNavPanel onNavigateToFile={vi.fn()} />);
    await submitSymbol("helper");
    expect(queryNavMock).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("tab", { name: "Call hierarchy" }));
    await flush();

    expect(queryNavMock).toHaveBeenCalledTimes(2);
    expect(queryNavMock).toHaveBeenLastCalledWith(
      "find_callers",
      expect.objectContaining({ name: "helper" }),
    );
    expect(screen.getByText("1 deterministic")).toBeInTheDocument();
  });

  it("scopes and collapses the dependency graph and reports cycles", async () => {
    render(<RepoNavPanel onNavigateToFile={vi.fn()} />);
    await submitSymbol("helper");

    fireEvent.click(screen.getByRole("tab", { name: "Dependency graph" }));
    await flush();

    expect(queryNavMock).toHaveBeenLastCalledWith(
      "repository_graph_query",
      expect.objectContaining({ limit: 400, include_cycles: true }),
    );
    expect(screen.getByText("2 of 2 edges")).toBeInTheDocument();
    expect(screen.getByText("1 import cycle detected")).toBeInTheDocument();
    expect(screen.getByTitle("Open src/util.rs")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Edge kind filter"), {
      target: { value: "imports" },
    });
    expect(screen.getByText("1 of 2 edges")).toBeInTheDocument();
    expect(screen.queryByText("HEURISTIC")).not.toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// G11 — overlapping queries
//
// The race that used to blank the window: request A is in flight, the user
// moves to another view, A resolves, and A's payload is rendered under view B
// (`undefined.filter` → React root unmounts). These tests hold the invariant
// that a response only ever updates the view/request that owns it.
// ---------------------------------------------------------------------------

describe("RepoNavPanel — G11 overlapping queries", () => {
  const readyWith = (data: unknown): NavOutcome => ({
    kind: "ready",
    data,
    via: "service",
  });

  /** Resolve a test-controlled promise and let React process the result. */
  async function settle(fn: () => void) {
    await act(async () => {
      fn();
      for (let i = 0; i < 20; i++) await Promise.resolve();
    });
  }

  it("discards an answer that resolves after the view has moved on", async () => {
    const references = deferred<NavOutcome>();
    const impact = deferred<NavOutcome>();
    queryNavMock
      .mockImplementationOnce(() => references.promise)
      .mockImplementationOnce(() => impact.promise);

    const { container } = render(<RepoNavPanel onNavigateToFile={vi.fn()} />);

    await submitSymbol("helper");
    expect(screen.getByText(/Querying the repository index/)).toBeInTheDocument();

    // Impatient user: switch to Impact while the references query is running.
    fireEvent.click(screen.getByRole("tab", { name: "Impact" }));
    await flush();
    expect(queryNavMock).toHaveBeenCalledTimes(2);

    // The references answer now arrives — for a view nobody is on any more.
    await settle(() => references.resolve(readyWith(REFERENCES)));

    // It is dropped whole: the panel stays in flight for Impact, the
    // mismatched-payload fallback never has to fire, and nothing from the
    // abandoned query is drawn.
    expect(screen.getByText(/Querying the repository index/)).toBeInTheDocument();
    expect(screen.queryByText(/belongs to a different view/)).not.toBeInTheDocument();
    expect(screen.queryByText("2 deterministic")).not.toBeInTheDocument();

    // The application is still mounted.
    expect(screen.getByRole("tab", { name: "Impact" })).toBeInTheDocument();
    expect(container.firstElementChild).not.toBeNull();

    // The Impact answer lands and is drawn as Impact.
    await settle(() => impact.resolve(readyWith(IMPACT)));
    expect(screen.getByText("1 direct")).toBeInTheDocument();
    expect(screen.queryByText("2 deterministic")).not.toBeInTheDocument();
  });

  it("keeps the newest request authoritative when an older one resolves later", async () => {
    const references = deferred<NavOutcome>();
    const callers = deferred<NavOutcome>();
    queryNavMock
      .mockImplementationOnce(() => references.promise)
      .mockImplementationOnce(() => callers.promise);

    render(<RepoNavPanel onNavigateToFile={vi.fn()} />);
    await submitSymbol("helper"); // request 1 — references
    fireEvent.click(screen.getByRole("tab", { name: "Call hierarchy" }));
    await flush(); // request 2 — callers

    // The newer request answers first…
    await settle(() => callers.resolve(readyWith(CALLERS)));
    expect(screen.getByText("1 deterministic")).toBeInTheDocument();

    // …and the older one arriving afterwards must not displace it.
    await settle(() => references.resolve(readyWith(REFERENCES)));
    expect(screen.getByText("1 deterministic")).toBeInTheDocument();
    expect(screen.queryByText("2 deterministic")).not.toBeInTheDocument();
    expect(screen.queryByText(/belongs to a different view/)).not.toBeInTheDocument();
  });

  it("survives A → B → A switching, drawing only the answer for the view asked last", async () => {
    const refsFirst = deferred<NavOutcome>();
    const calls = deferred<NavOutcome>();
    const refsAgain = deferred<NavOutcome>();
    queryNavMock
      .mockImplementationOnce(() => refsFirst.promise)
      .mockImplementationOnce(() => calls.promise)
      .mockImplementationOnce(() => refsAgain.promise);

    render(<RepoNavPanel onNavigateToFile={vi.fn()} />);
    await submitSymbol("helper"); // A
    fireEvent.click(screen.getByRole("tab", { name: "Call hierarchy" })); // B
    await flush();
    fireEvent.click(screen.getByRole("tab", { name: "References" })); // A again
    await flush();
    expect(queryNavMock).toHaveBeenCalledTimes(3);

    await settle(() => refsFirst.resolve(readyWith(REFERENCES)));
    expect(screen.getByText(/Querying the repository index/)).toBeInTheDocument();

    await settle(() => calls.resolve(readyWith(CALLERS)));
    expect(screen.getByText(/Querying the repository index/)).toBeInTheDocument();

    await settle(() => refsAgain.resolve(readyWith(REFERENCES)));
    expect(screen.getByText("2 deterministic")).toBeInTheDocument();
    expect(screen.queryByText("1 deterministic")).not.toBeInTheDocument();
    expect(screen.queryByText("1 direct")).not.toBeInTheDocument();
  });

  it("refuses an engine payload that does not answer the requested mode", async () => {
    queryNavMock.mockResolvedValueOnce(readyWith(CALLERS) as never);

    render(<RepoNavPanel onNavigateToFile={vi.fn()} />);
    await submitSymbol("helper");

    expect(screen.getByText(/unrelated payload/)).toBeInTheDocument();
    expect(screen.queryByText(/No reference matched/)).not.toBeInTheDocument();
    expect(screen.queryByText("1 deterministic")).not.toBeInTheDocument();
  });

  it("contains a rendering failure instead of unmounting the rest of ZylCode", () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    let shouldThrow = true;
    function Boom() {
      if (shouldThrow) throw new Error("G11 probe");
      return <p>navigation panel recovered</p>;
    }

    render(
      <div>
        <p>the rest of ZylCode</p>
        <NavErrorBoundary>
          <Boom />
        </NavErrorBoundary>
      </div>,
    );

    expect(screen.getByText(/Repository Intelligence could not be rendered/)).toBeInTheDocument();
    expect(screen.getByText("G11 probe")).toBeInTheDocument();
    expect(screen.getByText("the rest of ZylCode")).toBeInTheDocument();

    shouldThrow = false;
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(screen.getByText("navigation panel recovered")).toBeInTheDocument();
    expect(screen.queryByText(/could not be rendered/)).not.toBeInTheDocument();

    consoleError.mockRestore();
  });
});
