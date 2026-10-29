import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PulsePanel } from "./PulsePanel";
import type { Mission, MissionMode } from "../lib/missions";
import type { EvidenceState } from "../lib/surfaces";

// ---------------------------------------------------------------------------
// Mission console (components/PulsePanel.tsx).
//
// The rules under test:
//   * Build vs Plan is explicit — the composer says which one runs the real
//     pipeline and which one only plans;
//   * an empty task can never be queued, Enter queues, and the input clears;
//   * the connection badge is derived from real ledger state;
//   * the drain loop runs queued missions one by one until none remain;
//   * the queue and evidence feed render backend data as-is.
// ---------------------------------------------------------------------------

vi.mock("../lib/missions", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/missions")>();
  return {
    ...actual,
    enqueueMission: vi.fn(),
    listMissions: vi.fn(),
    runNextMission: vi.fn(),
    clearMissions: vi.fn(),
  };
});
vi.mock("../lib/surfaces", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/surfaces")>();
  return { ...actual, fetchEvidence: vi.fn() };
});

const { enqueueMission, listMissions, runNextMission, clearMissions } = await import(
  "../lib/missions"
);
const { fetchEvidence } = await import("../lib/surfaces");

const mis = vi.mocked(listMissions);
const enq = vi.mocked(enqueueMission);
const run = vi.mocked(runNextMission);
const clr = vi.mocked(clearMissions);
const evi = vi.mocked(fetchEvidence);

const mission = (over: Partial<Mission> = {}): Mission => ({
  id: "m1",
  task: "add rate limiting",
  mode: "build",
  state: "queued",
  createdAt: "2026-10-28T10:00:00.000Z",
  updatedAt: new Date().toISOString(),
  ...over,
});

function evidence(overrides: Partial<Extract<EvidenceState, { kind: "ready" }>> = {}) {
  return {
    kind: "ready" as const,
    chain_intact: true,
    session_count: 1,
    entry_count: 1,
    entries: [
      {
        id: "e1",
        session_id: "sess-abcdef0123456789",
        action_id: "mission.finished",
        state: "Recorded",
        timestamp: "2026-10-28T10:00:00.000Z",
        error: null,
        payload: null,
      },
    ],
    ...overrides,
  };
}

beforeEach(() => {
  mis.mockReset().mockResolvedValue([]);
  enq.mockReset().mockResolvedValue(mission());
  run.mockReset().mockResolvedValue(null);
  clr.mockReset().mockResolvedValue(true);
  evi.mockReset().mockResolvedValue({ kind: "unavailable", reason: "ledger offline" });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("PulsePanel", () => {
  it("makes the Build/Plan distinction explicit in the composer", () => {
    render(<PulsePanel />);
    expect(
      screen.getByText("runs the real pipeline · verified by the test suite"),
    ).toBeInTheDocument();
    expect(
      screen.getByPlaceholderText("Describe the change to build…"),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Plan/ }));
    expect(screen.getByText("design only · nothing executes")).toBeInTheDocument();
    expect(
      screen.getByPlaceholderText("Describe what to plan…"),
    ).toBeInTheDocument();
  });

  it("refuses to queue an empty task and clears the input after queuing", async () => {
    render(<PulsePanel />);
    const queue = screen.getByRole("button", { name: "Queue" });
    expect(queue).toBeDisabled();

    const input = screen.getByPlaceholderText("Describe the change to build…");
    fireEvent.change(input, { target: { value: "add rate limiting" } });
    expect(queue).toBeEnabled();

    fireEvent.click(queue);
    await waitFor(() =>
      expect(enq).toHaveBeenCalledWith("add rate limiting", "build"),
    );
    expect(input).toHaveValue("");

    // Plan mode is passed through, not coerced.
    fireEvent.click(screen.getByRole("button", { name: /Plan/ }));
    fireEvent.change(input, { target: { value: "design the queue" } });
    fireEvent.click(screen.getByRole("button", { name: "Queue" }));
    await waitFor(() =>
      expect(enq).toHaveBeenLastCalledWith("design the queue", "plan"),
    );
  });

  it("queues on Enter as well as the button", async () => {
    render(<PulsePanel />);
    const input = screen.getByPlaceholderText("Describe the change to build…");
    fireEvent.change(input, { target: { value: "typed task" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(enq).toHaveBeenCalledWith("typed task", "build"));
  });

  it("derives the status badge from real ledger state", async () => {
    evi.mockResolvedValue(evidence({ entry_count: 42, chain_intact: true }));
    const first = render(<PulsePanel />);
    expect(
      await screen.findByText("42 ledger entries · chain intact"),
    ).toBeInTheDocument();
    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();
    first.unmount();

    evi.mockResolvedValue(evidence({ chain_intact: false }));
    const second = render(<PulsePanel />);
    expect(
      await screen.findByText("1 ledger entries · chain BROKEN"),
    ).toBeInTheDocument();
    second.unmount();

    evi.mockResolvedValue({ kind: "empty", reason: "ledger empty — run a build mission to record evidence" });
    const third = render(<PulsePanel />);
    expect(
      (await screen.findAllByText("ledger empty — run a build mission to record evidence"))
        .length,
    ).toBeGreaterThan(0);
    expect(screen.getByText("LIMITED")).toBeInTheDocument();
    third.unmount();

    evi.mockResolvedValue({ kind: "unavailable", reason: "serve-intel is down" });
    render(<PulsePanel />);
    // Reason appears both as the status note and in the evidence feed.
    expect((await screen.findAllByText("serve-intel is down")).length).toBe(2);
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
  });

  it("shows the honest empty state for both panels", async () => {
    evi.mockResolvedValue({ kind: "loading" });
    render(<PulsePanel />);
    expect(
      await screen.findByText(/Empty\. Queue a task above/),
    ).toBeInTheDocument();
    expect(
      await screen.findByText(/No evidence yet/),
    ).toBeInTheDocument();
    expect(screen.getByText("MISSION QUEUE")).toBeInTheDocument();
  });

  it("renders missions with state, mode, task and ledger session", async () => {
    mis.mockResolvedValue([
      mission({ state: "failed", mode: "plan", summary: "plan stopped", ledgerSession: "feedfacefeedface" }),
      mission({ id: "m2", state: "done", task: "ship it" }),
    ]);
    render(<PulsePanel />);

    expect(await screen.findByText("MISSION QUEUE · 2")).toBeInTheDocument();
    expect(screen.getByText("failed")).toBeInTheDocument();
    expect(screen.getByText("plan")).toBeInTheDocument();
    expect(screen.getByText("plan stopped")).toBeInTheDocument();
    expect(screen.getByText("ledger: feedface")).toBeInTheDocument();
    expect(screen.getByText("ship it")).toBeInTheDocument();
  });

  it("shows the running banner while a mission is active", async () => {
    mis.mockResolvedValue([mission({ state: "running" })]);
    render(<PulsePanel />);
    expect(await screen.findByText("running:")).toBeInTheDocument();
    // The task shows in the running banner and in the queue row.
    expect(screen.getAllByText("add rate limiting").length).toBeGreaterThan(0);
  });

  it("drains the whole queue mission by mission", async () => {
    // mount refresh → [queued] (drain button visible); loop pre-check →
    // [queued] (run it); post-run refresh and every later call → [done],
    // so the loop terminates after exactly one run.
    mis
      .mockResolvedValueOnce([mission()])
      .mockResolvedValueOnce([mission()])
      .mockResolvedValue([mission({ state: "done" })]);
    render(<PulsePanel />);

    const drain = await screen.findByRole("button", { name: /Run 1 queued/ });
    fireEvent.click(drain);

    await waitFor(() => expect(run).toHaveBeenCalledTimes(1));
    // Queue is drained: the drain button disappears (no queued missions left).
    await waitFor(() =>
      expect(
        screen.queryByRole("button", { name: /Run \d+ queued/ }),
      ).not.toBeInTheDocument(),
    );
  });

  it("clears the queue through the backend", async () => {
    mis.mockResolvedValue([mission()]);
    render(<PulsePanel />);
    fireEvent.click(await screen.findByRole("button", { name: "Clear" }));
    await waitFor(() => expect(clr).toHaveBeenCalledTimes(1));
  });

  it("renders the evidence feed with the chain verdict", async () => {
    evi.mockResolvedValue(evidence({ chain_intact: true }));
    render(<PulsePanel />);
    expect(
      await screen.findByText(/EVIDENCE ACTIVITY · ✓ chain intact/),
    ).toBeInTheDocument();
    expect(screen.getByText("mission.finished")).toBeInTheDocument();
    expect(screen.getByText(/session sess-abc/)).toBeInTheDocument();
  });

  it("states the ledger's own reason when evidence is unavailable", async () => {
    evi.mockResolvedValue({ kind: "unavailable", reason: "serve-intel is down" });
    render(<PulsePanel />);
    expect((await screen.findAllByText("serve-intel is down")).length).toBeGreaterThan(0);
  });
});