import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { EvidenceTimeline } from "./EvidenceTimeline";
import { fetchEvidence, type EvidenceState } from "../lib/surfaces";

// ---------------------------------------------------------------------------
// The ledger is the audit surface. The hash-chain verdict is the single most
// load-bearing thing on this panel: a broken chain must be impossible to miss,
// and an empty ledger must read as "nothing happened yet", never as a failure.
// Both are pinned, along with the failed-entry styling.
// ---------------------------------------------------------------------------

vi.mock("../lib/surfaces", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/surfaces")>();
  return { ...actual, fetchEvidence: vi.fn() };
});

const evidence = vi.mocked(fetchEvidence);

type Ready = Extract<EvidenceState, { kind: "ready" }>;

const READY: Ready = {
  kind: "ready",
  chain_intact: true,
  session_count: 2,
  entry_count: 3,
  entries: [
    {
      id: "e1",
      session_id: "session-0001-abcdef",
      action_id: "tool.exec",
      state: '"Complete"',
      timestamp: "2026-10-30T10:00:00.000Z",
      error: null,
      payload: null,
    },
    {
      id: "e2",
      session_id: "session-0002-fedcba",
      action_id: "tool.shell",
      state: '"Failed"',
      timestamp: "2026-10-30T10:05:00.000Z",
      error: "exit code 1",
      payload: null,
    },
    {
      id: "e3",
      session_id: "session-0002-fedcba",
      action_id: "mission.verify",
      state: '"Complete"',
      timestamp: "2026-10-30T10:06:00.000Z",
      error: null,
      payload: null,
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  evidence.mockReturnValue(new Promise(() => {}));
});

describe("EvidenceTimeline", () => {
  it("shows the reading state while the ledger is being fetched", () => {
    render(<EvidenceTimeline />);
    expect(screen.getByText("Reading the ledger…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reading…" })).toBeDisabled();
  });

  it("renders a verified chain with the entry and session counts", async () => {
    evidence.mockResolvedValue(READY);
    render(<EvidenceTimeline />);

    expect(await screen.findByText("✓ hash chain intact")).toBeInTheDocument();
    expect(screen.getByText(/3 entries · 2 sessions/)).toBeInTheDocument();
    expect(screen.getByText("tool.exec")).toBeInTheDocument();
    expect(screen.getByText("tool.shell")).toBeInTheDocument();
    // quotes are stripped from the raw state (first quote only)
    expect(screen.getAllByText(/Complete/).length).toBe(2);
    expect(screen.getAllByText(/Failed/).length).toBeGreaterThan(0);
    // failed entries carry their error
    expect(screen.getByText("exit code 1")).toBeInTheDocument();
    expect(screen.getAllByText(/session session-/)).toHaveLength(3);
  });

  it("makes a tampered chain impossible to miss", async () => {
    evidence.mockResolvedValue({
      ...READY,
      chain_intact: false,
      entries: [READY.entries[0]],
      entry_count: 1,
      session_count: 1,
    });
    render(<EvidenceTimeline />);

    expect(
      await screen.findByText("✗ chain broken — evidence has been tampered with or corrupted"),
    ).toBeInTheDocument();
    expect(screen.queryByText("✓ hash chain intact")).not.toBeInTheDocument();
    // singular forms
    expect(screen.getByText(/1 entry · 1 session\b/)).toBeInTheDocument();
  });

  it("renders an empty ledger as 'nothing yet', not as an error", async () => {
    evidence.mockResolvedValue({ kind: "empty", reason: "The ledger is empty." });
    render(<EvidenceTimeline />);

    expect(await screen.findByText("The ledger is empty.")).toBeInTheDocument();
    expect(screen.getByText(/Evidence lands here automatically/)).toBeInTheDocument();
    // an empty ledger is not ready and not loading -> BLOCKED badge
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
  });

  it("distinguishes a ready-but-empty result list", async () => {
    evidence.mockResolvedValue({ ...READY, entries: [], entry_count: 0, session_count: 0 });
    render(<EvidenceTimeline />);

    expect(await screen.findByText("No entries.")).toBeInTheDocument();
    expect(screen.getByText(/0 entries · 0 sessions/)).toBeInTheDocument();
    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();
  });

  it("reports an unreachable ledger as unavailable", async () => {
    evidence.mockResolvedValue({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to read the evidence ledger",
    });
    render(<EvidenceTimeline />);

    expect(
      await screen.findByText("start `zylcode serve-intel` to read the evidence ledger"),
    ).toBeInTheDocument();
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
  });

  it("re-reads on Refresh", async () => {
    evidence.mockResolvedValue(READY);
    render(<EvidenceTimeline />);
    await waitFor(() => expect(evidence).toHaveBeenCalledTimes(1));

    // re-read on Refresh: point the service at a new state first
    evidence.mockResolvedValue({ kind: "empty", reason: "clean slate" });
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    expect(await screen.findByText("clean slate")).toBeInTheDocument();
    expect(evidence).toHaveBeenCalledTimes(2);
  });
});
