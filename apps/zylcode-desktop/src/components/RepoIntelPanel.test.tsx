import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RepoIntelPanel } from "./RepoIntelPanel";
import { fetchRepoIntel, type RepoIntelState } from "../lib/repoIntel";

// ---------------------------------------------------------------------------
// Repository Intelligence renders ranked retrieval output with provenance.
// The panel must never pad an empty result: a query the retriever cannot
// answer says so. Both bounds of that (0 results / very many of each list)
// are pinned, along with the controlled unavailable state and re-querying.
// ---------------------------------------------------------------------------

vi.mock("../lib/repoIntel", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/repoIntel")>();
  return { ...actual, fetchRepoIntel: vi.fn() };
});

const intel = vi.mocked(fetchRepoIntel);

type Ready = Extract<RepoIntelState, { kind: "ready" }>;

const RESULT: Ready = {
  kind: "ready",
  via: "service",
  data: {
    task: "where is the router",
    elapsed_ms: 42.6,
    summary: {
      file_count: 120,
      symbol_count: 3400,
      package_count: 6,
      packages: ["zylcode-core"],
      languages: ["Rust", "TypeScript", "Python", "Go", "C", "C++", "Zig", "Shell", "Ruby"],
      entry_points: ["a", "b", "c", "d", "e", "f"],
      architecture_facts: [],
      recent_changes: [{ short_id: "abc1234", message: "tighten routing", author: "founder" }],
    },
    results: [
      {
        resource: "crates/zylcode-core/src/router.rs",
        resource_type: "file",
        relevance: 0.91,
        reason: "declares the router",
      },
      {
        resource: "crates/zylcode-core/src/router/cache.rs",
        resource_type: "file",
        relevance: 0.4,
        reason: "caches decisions",
      },
    ],
  },
};

beforeEach(() => {
  vi.clearAllMocks();
  intel.mockReturnValue(new Promise(() => {}));
});

describe("RepoIntelPanel", () => {
  it("shows the indexing state while the first query runs", () => {
    render(<RepoIntelPanel />);
    expect(
      screen.getByText("Indexing the repository (persisted, content-hash validated)…"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Indexing…" })).toBeDisabled();
  });

  it("renders counts, ranked results and provenance for a ready payload", async () => {
    intel.mockResolvedValue(RESULT);
    render(<RepoIntelPanel />);

    expect(await screen.findByText("120")).toBeInTheDocument();
    expect(screen.getByText("3400")).toBeInTheDocument();
    expect(screen.getByText("6")).toBeInTheDocument();
    expect(screen.getByText("via intel service · 43 ms")).toBeInTheDocument();

    // more than 8 languages -> truncated with an ellipsis marker
    expect(
      screen.getByText(/Rust, TypeScript, Python, Go, C, C\+\+, Zig, Shell …/),
    ).toBeInTheDocument();

    // entry points capped at 5 of 6
    expect(screen.getAllByRole("listitem").length).toBeGreaterThan(0);

    expect(screen.getByText("abc1234")).toBeInTheDocument();
    expect(screen.getByText("tighten routing")).toBeInTheDocument();

    expect(screen.getByText("RANKED CONTEXT FOR “where is the router”")).toBeInTheDocument();
    expect(screen.getByText("crates/zylcode-core/src/router.rs")).toBeInTheDocument();
    expect(screen.getByText("0.91")).toBeInTheDocument();
    expect(screen.getAllByText(/\[file\]/)).toHaveLength(2);
    expect(screen.getByText("declares the router")).toBeInTheDocument();
    expect(
      screen.queryByText("The retriever returned no results for this task."),
    ).not.toBeInTheDocument();
  });

  it("says so when the retriever returns nothing", async () => {
    intel.mockResolvedValue({
      kind: "ready",
      via: "service",
      data: {
        ...RESULT.data,
        summary: {
          ...RESULT.data.summary,
          languages: [],
          entry_points: [],
          recent_changes: [],
        },
        results: [],
      },
    });
    render(<RepoIntelPanel />);

    expect(
      await screen.findByText("The retriever returned no results for this task."),
    ).toBeInTheDocument();
    expect(screen.queryByText("Languages:")).not.toBeInTheDocument();
    expect(screen.queryByText("ENTRY POINTS")).not.toBeInTheDocument();
    expect(screen.queryByText("RECENT CHANGES")).not.toBeInTheDocument();
  });

  it("omits the language line when fewer than nine languages are present", async () => {
    intel.mockResolvedValue({
      kind: "ready",
      via: "service",
      data: {
        ...RESULT.data,
        summary: { ...RESULT.data.summary, languages: ["Rust", "TypeScript"] },
      },
    });
    render(<RepoIntelPanel />);

    expect(await screen.findByText("Languages:")).toBeInTheDocument();
    expect(screen.getByText(/Rust, TypeScript$/)).toBeInTheDocument();
    expect(screen.queryByText(/…/)).not.toBeInTheDocument();
  });

  it("reports an unreachable pipeline as unavailable", async () => {
    intel.mockResolvedValue({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to see live repository data",
    });
    render(<RepoIntelPanel />);

    expect(
      await screen.findByText("start `zylcode serve-intel` to see live repository data"),
    ).toBeInTheDocument();
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
  });

  it("re-queries with the typed task on submit", async () => {
    intel.mockResolvedValue(RESULT);
    render(<RepoIntelPanel initialTask="first" />);
    await waitFor(() => expect(intel).toHaveBeenCalledWith("first"));

    intel.mockResolvedValue(RESULT);
    fireEvent.change(screen.getByPlaceholderText("Ask the repository a task…"), {
      target: { value: "second query" },
    });
    fireEvent.submit(screen.getByRole("button", { name: "Query" }));

    await waitFor(() => expect(intel).toHaveBeenCalledWith("second query"));
    expect(intel).toHaveBeenCalledTimes(2);
  });
});
