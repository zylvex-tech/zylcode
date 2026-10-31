import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SourceControlPanel } from "./SourceControlPanel";
import { fetchGitStatus, type GitStatusState } from "../lib/gitStatus";

// ---------------------------------------------------------------------------
// SourceControlPanel must never imply a cleaner tree than git reports. The
// tests drive every rendering branch: clean vs dirty, tracked vs untracked
// (empty status renders as `??`), ahead/behind divergences, the diff stat's
// singular/plural, and the controlled unavailable state.
// ---------------------------------------------------------------------------

vi.mock("../lib/gitStatus", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/gitStatus")>();
  return { ...actual, fetchGitStatus: vi.fn() };
});

const status = vi.mocked(fetchGitStatus);

const DIRTY: GitStatusState = {
  kind: "ready",
  via: "service",
  data: {
    branch: "feat/gates",
    upstream: "origin/feat/gates",
    ahead: 2,
    behind: 0,
    head: "abc1234",
    clean: false,
    counts: { total: 3, staged: 1, unstaged: 1, untracked: 1 },
    entries: [
      { status: "M ", path: "src/App.tsx" },
      { status: "A ", path: "src/new.ts" },
      { status: "", path: "notes.txt" },
    ],
    diffstat: {
      files_changed: 1,
      insertions: 12,
      deletions: 3,
      rows: [{ path: "src/App.tsx", added: 12, removed: 3 }],
    },
  },
};

const CLEAN: GitStatusState = {
  kind: "ready",
  via: "desktop",
  data: {
    ...(DIRTY.data as Extract<GitStatusState, { kind: "ready" }>["data"]),
    branch: "main",
    upstream: null,
    ahead: 0,
    behind: 0,
    clean: true,
    entries: [],
    diffstat: { files_changed: 0, insertions: 0, deletions: 0, rows: [] },
  },
};

beforeEach(() => {
  vi.clearAllMocks();
  status.mockResolvedValue({ kind: "loading" });
});

describe("SourceControlPanel", () => {
  it("shows the reading state while the first fetch is in flight", () => {
    status.mockReturnValue(new Promise(() => {}));
    render(<SourceControlPanel />);

    expect(screen.getByText("Reading the git state…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reading…" })).toBeDisabled();
  });

  it("renders a dirty tree exactly as git reported it", async () => {
    status.mockResolvedValue(DIRTY);
    render(<SourceControlPanel />);

    expect(await screen.findByText("feat/gates")).toBeInTheDocument();
    expect(screen.getAllByText(/origin\/feat\/gates/).length).toBeGreaterThan(0);
    expect(screen.getByText("@abc1234")).toBeInTheDocument();
    expect(screen.getAllByText("↑2").length).toBeGreaterThan(0);
    expect(screen.queryByText(/↓\d/)).not.toBeInTheDocument();

    // counts grid: total 3, staged/unstaged/untracked 1 each
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getAllByText("1")).toHaveLength(3);

    // entries, with an empty status rendered as untracked
    expect(screen.getAllByText("src/App.tsx").length).toBeGreaterThan(0);
    expect(screen.getByText("src/new.ts")).toBeInTheDocument();
    expect(screen.getByText("notes.txt")).toBeInTheDocument();
    expect(screen.getAllByText("??")).toHaveLength(1);

    // singular diff stat
    expect(screen.getByText(/1 file,/)).toBeInTheDocument();
    // header total and the per-file row both carry +12/−3
    expect(screen.getAllByText("+12")).toHaveLength(2);
    expect(screen.getAllByText("−3")).toHaveLength(2);

    expect(screen.queryByText(/working tree is clean/)).not.toBeInTheDocument();
    expect(screen.getByText("via intel service")).toBeInTheDocument();
  });

  it("renders a clean tree without a changes list", async () => {
    status.mockResolvedValue(CLEAN);
    render(<SourceControlPanel />);

    expect(await screen.findByText("No changes — the working tree is clean.")).toBeInTheDocument();
    expect(screen.queryByText("CHANGES")).not.toBeInTheDocument();
    expect(screen.queryByText(/UNCOMMITTED/)).not.toBeInTheDocument();
    expect(screen.getByText("via desktop engine")).toBeInTheDocument();
    expect(screen.queryByText(/↑/)).not.toBeInTheDocument();
    expect(screen.queryByText(/↓/)).not.toBeInTheDocument();
  });

  it("hides the diff stat when nothing is uncommitted", async () => {
    status.mockResolvedValue({
      kind: "ready",
      via: "service",
      data: {
        ...DIRTY.data,
        diffstat: { files_changed: 0, insertions: 0, deletions: 0, rows: [] },
      },
    });
    render(<SourceControlPanel />);

    expect(await screen.findByText("CHANGES")).toBeInTheDocument();
    expect(screen.queryByText(/UNCOMMITTED/)).not.toBeInTheDocument();
  });

  it("shows only the behind marker when the branch is behind", async () => {
    status.mockResolvedValue({
      kind: "ready",
      via: "service",
      data: { ...DIRTY.data, ahead: 0, behind: 3 },
    });
    render(<SourceControlPanel />);

    expect((await screen.findAllByText("↓3")).length).toBeGreaterThan(0);
    expect(screen.queryByText(/↑\d/)).not.toBeInTheDocument();
  });

  it("reports an unreadable git state as unavailable rather than inventing one", async () => {
    status.mockResolvedValue({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to see live source-control data",
    });
    render(<SourceControlPanel />);

    expect(
      await screen.findByText("start `zylcode serve-intel` to see live source-control data"),
    ).toBeInTheDocument();
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
  });

  it("re-reads on Refresh and returns the button to enabled", async () => {
    status.mockResolvedValue(DIRTY);
    render(<SourceControlPanel />);
    await waitFor(() => expect(status).toHaveBeenCalledTimes(1));

    // switch the service to a clean tree, then re-read
    status.mockResolvedValue(CLEAN);
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    expect(await screen.findByText("No changes — the working tree is clean.")).toBeInTheDocument();
    expect(status).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("button", { name: "Refresh" })).toBeEnabled();
  });
});
