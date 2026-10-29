import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ChangesPanel } from "./ChangesPanel";
import type { GitStatusResult, GitStatusState } from "../lib/gitStatus";

// ---------------------------------------------------------------------------
// Source-control surface (components/ChangesPanel.tsx).
//
// The honesty rules under test:
//   * Staged changes are listed separately from — and above — unstaged ones,
//     because that is how commits are actually built.
//   * The porcelain XY pair is classified with the staged column dominating;
//     an untracked file shows as U, not as whatever the worktree column says.
//   * A transport failure surfaces the reason the user can act on, and the
//     panel never renders a clean tree it did not read.
// ---------------------------------------------------------------------------

vi.mock("../lib/gitStatus", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/gitStatus")>();
  return { ...actual, fetchGitStatus: vi.fn() };
});

const { fetchGitStatus } = await import("../lib/gitStatus");

function ready(overrides: Partial<GitStatusResult> = {}): GitStatusState {
  return {
    kind: "ready",
    via: "service",
    data: {
      branch: "feat/home-mission-pipeline",
      upstream: "origin/feat/home-mission-pipeline",
      ahead: 2,
      behind: 1,
      head: "43f11ed",
      clean: false,
      counts: { total: 3, staged: 1, unstaged: 2, untracked: 0 },
      entries: [],
      diffstat: { files_changed: 0, insertions: 0, deletions: 0, rows: [] },
      ...overrides,
    },
  };
}

beforeEach(() => {
  vi.mocked(fetchGitStatus).mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("ChangesPanel", () => {
  it("reports BLOCKED with an actionable reason when the service is down", async () => {
    vi.mocked(fetchGitStatus).mockResolvedValue({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to see live source-control data",
    });

    render(<ChangesPanel />);

    await waitFor(() =>
      expect(
        screen.getByText("start `zylcode serve-intel` to see live source-control data"),
      ).toBeInTheDocument(),
    );
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
    // No branch is rendered: the panel did not read a git state.
    expect(screen.queryByText(/feat\/home-mission-pipeline/)).not.toBeInTheDocument();
  });

  it("shows the branch, its tracking relation and the change summary", async () => {
    vi.mocked(fetchGitStatus).mockResolvedValue(
      ready({
        entries: [{ status: "M ", path: "README.md" }],
      }),
    );

    render(<ChangesPanel />);

    await waitFor(() =>
      expect(screen.getByText(/feat\/home-mission-pipeline/)).toBeInTheDocument(),
    );
    expect(screen.getByText("↑2")).toBeInTheDocument();
    expect(screen.getByText("↓1")).toBeInTheDocument();
    expect(screen.getByText("3 changes · 0 untracked")).toBeInTheDocument();
  });

  it("lists staged entries under a Staged heading, ahead of unstaged ones", async () => {
    vi.mocked(fetchGitStatus).mockResolvedValue(
      ready({
        entries: [
          { status: "M ", path: "staged.ts" },
          { status: " M", path: "unstaged.ts" },
        ],
      }),
    );

    render(<ChangesPanel />);

    await waitFor(() => expect(screen.getByText("staged.ts")).toBeInTheDocument());

    // Leaf text nodes in document order: the staged row must precede both the
    // "Staged" heading's counterpart and the unstaged heading.
    const order = Array.from(document.body.querySelectorAll("span, p"))
      .map((el) => el.textContent ?? "")
      .filter((t) => ["staged.ts", "unstaged.ts", "Staged", "Unstaged & untracked"].includes(t));

    expect(order).toEqual(["Staged", "staged.ts", "Unstaged & untracked", "unstaged.ts"]);
  });

  it("labels an untracked file U from the '?' worktree column", async () => {
    vi.mocked(fetchGitStatus).mockResolvedValue(
      ready({ entries: [{ status: "??", path: "new-file.ts" }] }),
    );

    render(<ChangesPanel />);

    await waitFor(() => expect(screen.getByText("new-file.ts")).toBeInTheDocument());
    expect(screen.getByText("U")).toBeInTheDocument();
  });

  it("prefers the staged column when both columns carry a letter", async () => {
    vi.mocked(fetchGitStatus).mockResolvedValue(
      ready({ entries: [{ status: "RM", path: "renamed.ts" }] }),
    );

    render(<ChangesPanel />);

    await waitFor(() => expect(screen.getByText("renamed.ts")).toBeInTheDocument());
    // R (staged rename) dominates M (worktree modify).
    expect(screen.getByText("R")).toBeInTheDocument();
  });

  it("renders the diffstat only when something actually changed", async () => {
    vi.mocked(fetchGitStatus).mockResolvedValue(
      ready({
        entries: [],
        diffstat: { files_changed: 2, insertions: 40, deletions: 7, rows: [] },
      }),
    );

    render(<ChangesPanel />);

    await waitFor(() => expect(screen.getByText("2 files changed")).toBeInTheDocument());
    expect(screen.getByText("+40")).toBeInTheDocument();
    expect(screen.getByText("−7")).toBeInTheDocument();
  });

  it("says the tree is clean rather than showing an empty list", async () => {
    vi.mocked(fetchGitStatus).mockResolvedValue(
      ready({
        clean: true,
        entries: [],
        counts: { total: 0, staged: 0, unstaged: 0, untracked: 0 },
      }),
    );

    render(<ChangesPanel />);

    await waitFor(() =>
      expect(screen.getByText("No changes — working tree clean.")).toBeInTheDocument(),
    );
    expect(screen.getByText("working tree clean")).toBeInTheDocument();
  });

  it("reads in LIMITED state before the first response arrives", async () => {
    vi.mocked(fetchGitStatus).mockResolvedValue(ready());

    render(<ChangesPanel />);

    await waitFor(() => expect(screen.getByText("AVAILABLE")).toBeInTheDocument());
  });
});
