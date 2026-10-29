import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ContextSidebar } from "./ContextSidebar";
import type { GitStatusState } from "../../lib/gitStatus";
import type { EvidenceState, SearchResult } from "../../lib/surfaces";

// ---------------------------------------------------------------------------
// Context sidebar (components/shell/ContextSidebar.tsx).
//
// The rules under test:
//   * each activity renders its own panel; `children` overrides everything;
//   * search runs nothing until asked, is disabled on an empty query, and
//     reports counts from the real index with the top hit's basename;
//   * source control and evidence summaries show real backend state with
//     honest loading/unavailable/empty branches;
//   * activities without their own panel say so instead of inventing rows.
// ---------------------------------------------------------------------------

vi.mock("../ExplorerTree", () => ({
  ExplorerTree: () => <div data-testid="explorer-stub" />,
}));
vi.mock("../../lib/gitStatus", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/gitStatus")>();
  return { ...actual, fetchGitStatus: vi.fn() };
});
vi.mock("../../lib/surfaces", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/surfaces")>();
  return {
    ...actual,
    fetchEvidence: vi.fn(),
    fetchSearch: vi.fn(),
  };
});

const { fetchGitStatus } = await import("../../lib/gitStatus");
const { fetchEvidence, fetchSearch } = await import("../../lib/surfaces");

const git = vi.mocked(fetchGitStatus);
const evi = vi.mocked(fetchEvidence);
const srch = vi.mocked(fetchSearch);

function gitReady(over: Partial<Extract<GitStatusState, { kind: "ready" }>["data"]> = {}) {
  return {
    kind: "ready" as const,
    via: "service" as const,
    data: {
      branch: "feat/home-mission-pipeline",
      upstream: null,
      ahead: 2,
      behind: 1,
      head: "abc1234",
      clean: false,
      counts: { total: 3, staged: 1, unstaged: 2, untracked: 0 },
      entries: [],
      diffstat: { files_changed: 0, insertions: 0, deletions: 0, rows: [] },
      ...over,
    },
  };
}

function evidence(over: Partial<Extract<EvidenceState, { kind: "ready" }>> = {}): EvidenceState {
  return {
    kind: "ready",
    chain_intact: true,
    session_count: 2,
    entry_count: 5,
    entries: [],
    ...over,
  };
}

function searchResult(results: SearchResult["results"]): SearchResult {
  return { query: "scanner", elapsed_ms: 3, indexed_files: 10, results };
}

beforeEach(() => {
  git.mockReset().mockResolvedValue({ kind: "loading" });
  evi.mockReset().mockResolvedValue({ kind: "loading" });
  srch.mockReset().mockResolvedValue({ kind: "unavailable", reason: "offline" });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("ContextSidebar", () => {
  it("renders children when provided, over the activity content", () => {
    render(
      <ContextSidebar activeActivity="explorer">
        <div data-testid="child">child wins</div>
      </ContextSidebar>,
    );
    expect(screen.getByTestId("child")).toBeInTheDocument();
    expect(screen.queryByTestId("explorer-stub")).not.toBeInTheDocument();
  });

  it("shows the explorer tree for the explorer activity", () => {
    render(<ContextSidebar activeActivity="explorer" />);
    expect(screen.getByTestId("explorer-stub")).toBeInTheDocument();
  });

  it("search: idle copy, disabled until a query exists, real results after", async () => {
    srch.mockResolvedValue({
      kind: "ready",
      data: searchResult([
        { resource: "src/intelligence/scanner.rs", resource_type: "file", relevance: 1.2, reason: "path" },
        { resource: "src/other.rs", resource_type: "file", relevance: 0.4, reason: "path" },
      ]),
    });
    render(<ContextSidebar activeActivity="search" />);

    expect(
      screen.getByText("Results come from the real index, ranked."),
    ).toBeInTheDocument();
    const button = screen.getByRole("button", { name: "Search" });
    expect(button).toBeDisabled();

    const input = screen.getByPlaceholderText("Search symbols, files…");
    fireEvent.change(input, { target: { value: "scanner" } });
    expect(button).toBeEnabled();
    fireEvent.click(button);

    expect(await screen.findByText(/2 results · top: scanner.rs/)).toBeInTheDocument();
    expect(srch).toHaveBeenCalledWith("scanner");
  });

  it("search: Enter runs it, one result reads singular, failures show the reason", async () => {
    srch.mockResolvedValue({
      kind: "ready",
      data: searchResult([
        { resource: "src/only.rs", resource_type: "file", relevance: 1, reason: "path" },
      ]),
    });
    const first = render(<ContextSidebar activeActivity="search" />);
    const input = screen.getByPlaceholderText("Search symbols, files…");
    fireEvent.change(input, { target: { value: "one" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(await screen.findByText(/1 result · top: only\.rs/)).toBeInTheDocument();
    first.unmount();

    srch.mockResolvedValue({ kind: "unavailable", reason: "serve-intel is down" });
    const second = render(<ContextSidebar activeActivity="search" />);
    const input2 = screen.getByPlaceholderText("Search symbols, files…");
    fireEvent.change(input2, { target: { value: "x" } });
    fireEvent.keyDown(input2, { key: "Enter" });
    expect(await screen.findByText("serve-intel is down")).toBeInTheDocument();
    second.unmount();
  });

  it("search refuses an empty query", () => {
    render(<ContextSidebar activeActivity="search" />);
    expect(screen.getByRole("button", { name: "Search" })).toBeDisabled();
    fireEvent.keyDown(screen.getByPlaceholderText("Search symbols, files…"), {
      key: "Enter",
    });
    expect(srch).not.toHaveBeenCalled();
  });

  it("source control summary reports real branch, divergence and change count", async () => {
    git.mockResolvedValue(gitReady());
    render(<ContextSidebar activeActivity="source-control" />);

    expect(await screen.findByText("feat/home-mission-pipeline")).toBeInTheDocument();
    expect(screen.getByText("↑2 ↓1")).toBeInTheDocument();
    expect(screen.getByText("3 changes")).toBeInTheDocument();
  });

  it("source control summary: clean worktree and unavailable states", async () => {
    git.mockResolvedValue(gitReady({ clean: true, ahead: 0, behind: 0, counts: { ...gitReady().data.counts, total: 0 } }));
    const first = render(<ContextSidebar activeActivity="source-control" />);
    expect(await screen.findByText("No changes")).toBeInTheDocument();
    expect(screen.queryByText(/↑/)).not.toBeInTheDocument();
    first.unmount();

    git.mockResolvedValue({ kind: "unavailable", reason: "not a git repository" });
    const second = render(<ContextSidebar activeActivity="source-control" />);
    expect(await screen.findByText("not a git repository")).toBeInTheDocument();
  });

  it("evidence summary renders the chain verdict and refreshes on demand", async () => {
    evi.mockResolvedValue(evidence());
    render(<ContextSidebar activeActivity="evidence" />);

    expect(await screen.findByText("5 entries · 2 sessions")).toBeInTheDocument();
    expect(screen.getByText("✓ hash chain intact")).toBeInTheDocument();

    evi.mockResolvedValue(evidence({ chain_intact: false, entry_count: 1, session_count: 1 }));
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    expect(await screen.findByText("1 entry · 1 session")).toBeInTheDocument();
    expect(screen.getByText("✗ chain broken")).toBeInTheDocument();
    expect(evi).toHaveBeenCalledTimes(2);
  });

  it("evidence summary: empty and unavailable states are honest", async () => {
    evi.mockResolvedValue({ kind: "empty", reason: "ledger has no entries" });
    const first = render(<ContextSidebar activeActivity="evidence" />);
    expect(await screen.findByText("No evidence recorded yet.")).toBeInTheDocument();
    first.unmount();

    evi.mockResolvedValue({ kind: "unavailable", reason: "serve-intel is down" });
    const second = render(<ContextSidebar activeActivity="evidence" />);
    expect(await screen.findByText("serve-intel is down")).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByText("Reading the ledger…")).not.toBeInTheDocument());
    second.unmount();
  });

  it("renders the static intel / missions / run / forge / extensions panels", () => {
    const cases: [Parameters<typeof ContextSidebar>[0]["activeActivity"], string, string][] = [
      ["intel", "CODE NAVIGATION", "References — definition plus every location"],
      ["missions", "MISSIONS", "Mission history will appear here"],
      ["run", "RUN", "Runtime targets will appear here"],
      ["forge", "FORGE", "Installed capability packs will appear here"],
      ["extensions", "EXTENSIONS", "Extension management coming soon"],
    ];
    for (const [activity, title, copy] of cases) {
      const view = render(<ContextSidebar activeActivity={activity} />);
      expect(screen.getByText(title)).toBeInTheDocument();
      expect(screen.getByText(copy)).toBeInTheDocument();
      view.unmount();
    }
    expect(
      screen.queryByText("Select an activity"),
    ).not.toBeInTheDocument();
  });

  it("activities without their own panel fall back to a labelled placeholder", () => {
    render(<ContextSidebar activeActivity="settings" />);
    expect(screen.getByText("SETTINGS")).toBeInTheDocument();
    expect(screen.getByText("Select an activity")).toBeInTheDocument();
  });
});