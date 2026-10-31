import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AgentWorkspace } from "./AgentWorkspace";
import { enqueueMission, listMissions, runNextMission, type Mission } from "../lib/missions";
import { fetchEvidence, type EvidenceState } from "../lib/surfaces";

// ---------------------------------------------------------------------------
// AgentWorkspace is where a human authorises real work. Three properties are
// load-bearing and each is pinned here:
//   * nothing auto-submits — Queue is disabled until a task is typed, and a
//     running mission blocks the Run control;
//   * running queued work requires an explicit confirmation, and declining it
//     must not dispatch anything;
//   * the status strip reports the real ledger state, never an optimistic one.
// ---------------------------------------------------------------------------

vi.mock("../lib/missions", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/missions")>();
  return { ...actual, enqueueMission: vi.fn(), listMissions: vi.fn(), runNextMission: vi.fn() };
});

vi.mock("../lib/surfaces", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/surfaces")>();
  return { ...actual, fetchEvidence: vi.fn() };
});

const enqueue = vi.mocked(enqueueMission);
const list = vi.mocked(listMissions);
const runNext = vi.mocked(runNextMission);
const evidence = vi.mocked(fetchEvidence);

function mission(overrides: Partial<Mission> = {}): Mission {
  return {
    id: "m-1",
    task: "ship the fix",
    mode: "build",
    state: "queued",
    createdAt: "2026-10-30T10:00:00.000Z",
    updatedAt: new Date().toISOString(),
    summary: null,
    ledgerSession: null,
    ...overrides,
  } as Mission;
}

const LEDGER: EvidenceState = {
  kind: "ready",
  chain_intact: true,
  session_count: 1,
  entry_count: 4,
  entries: [
    {
      id: "e1",
      session_id: "sess-abcd1234",
      action_id: "tool.exec",
      state: "Complete",
      timestamp: "2026-10-30T10:00:00.000Z",
      error: null,
      payload: null,
    },
  ],
};

function renderWs(props: Partial<Parameters<typeof AgentWorkspace>[0]> = {}) {
  const onTabChange = vi.fn();
  const ui = render(<AgentWorkspace activeTab="agent" onTabChange={onTabChange} {...props} />);
  return { ...ui, onTabChange };
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.useFakeTimers({ shouldAdvanceTime: true });
  list.mockResolvedValue([]);
  enqueue.mockResolvedValue(mission());
  runNext.mockResolvedValue(mission({ state: "done" }) as unknown as Record<string, unknown>);
  evidence.mockResolvedValue({ kind: "empty", reason: "ledger empty — run a build mission" });
});

describe("AgentWorkspace", () => {
  it("exposes all four tabs and reports tab changes", () => {
    const { onTabChange } = renderWs();
    const tabs: [label: string, id: string][] = [
      ["AI Agent", "agent"],
      ["Tasks", "tasks"],
      ["Changes", "changes"],
      ["Evidence", "evidence"],
    ];
    for (const [label, id] of tabs) {
      fireEvent.click(screen.getByText(label));
      expect(onTabChange).toHaveBeenCalledWith(id);
    }
  });

  it("keeps Queue disabled until a task is typed", () => {
    renderWs();
    const queue = screen.getByRole("button", { name: "Queue" });
    expect(queue).toBeDisabled();

    fireEvent.change(screen.getByPlaceholderText(/Describe a task/), {
      target: { value: "   " },
    });
    expect(queue).toBeDisabled();

    fireEvent.change(screen.getByPlaceholderText(/Describe a task/), {
      target: { value: "real work" },
    });
    expect(queue).toBeEnabled();
  });

  it("queues the typed task in the selected mode and clears the composer", async () => {
    renderWs();
    fireEvent.change(screen.getByPlaceholderText(/Describe a task/), {
      target: { value: "real work" },
    });
    fireEvent.click(screen.getByText("🗺 Plan"));
    fireEvent.click(screen.getByRole("button", { name: "Queue" }));

    await waitFor(() => expect(enqueue).toHaveBeenCalledWith("real work", "plan"));
    expect(screen.getByPlaceholderText(/Describe a task/)).toHaveValue("");
  });

  it("queues on Ctrl+Enter from the composer", async () => {
    renderWs();
    const box = screen.getByPlaceholderText(/Describe a task/);
    fireEvent.change(box, { target: { value: "via shortcut" } });
    fireEvent.keyDown(box, { key: "Enter", ctrlKey: true });

    await waitFor(() => expect(enqueue).toHaveBeenCalledWith("via shortcut", "build"));
  });

  it("never queues an empty task", () => {
    renderWs();
    fireEvent.click(screen.getByRole("button", { name: "Queue" }));
    expect(enqueue).not.toHaveBeenCalled();
  });

  it("reports an empty ledger in the status strip", async () => {
    renderWs();
    expect(await screen.findByText("ledger empty — run a build mission")).toBeInTheDocument();
  });

  it("reports a verified chain in the status strip", async () => {
    evidence.mockResolvedValue(LEDGER);
    renderWs();
    expect(await screen.findByText("4 entries · chain intact")).toBeInTheDocument();
  });

  it("reports a broken chain and a reason verbatim when the ledger fails", async () => {
    evidence.mockResolvedValue({ ...LEDGER, chain_intact: false });
    renderWs({ activeTab: "evidence" });
    expect(await screen.findByText("✕ CHAIN BROKEN")).toBeInTheDocument();
    expect(screen.queryByText("✓ hash chain intact")).not.toBeInTheDocument();
  });

  it("renders the empty mission queue honestly", async () => {
    renderWs({ activeTab: "tasks" });
    expect(await screen.findByText("Mission queue · 0")).toBeInTheDocument();
    expect(
      screen.getByText("No missions yet — compose one in the AI Agent tab."),
    ).toBeInTheDocument();
  });

  it("renders queued work and the run control", async () => {
    list.mockResolvedValue([
      mission({ id: "m-1", state: "queued", task: "first" }),
      mission({ id: "m-2", state: "queued", task: "second", mode: "plan" }),
    ]);
    renderWs({ activeTab: "tasks" });

    expect(await screen.findByText("Mission queue · 2")).toBeInTheDocument();
    expect(screen.getByText("first")).toBeInTheDocument();
    expect(screen.getByText("second")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Run 2 queued" })).toBeEnabled();
  });

  it("dispatches the next queued mission only after explicit confirmation", async () => {
    list.mockResolvedValue([mission({ state: "queued", task: "first" })]);
    vi.spyOn(window, "confirm").mockReturnValue(true);
    renderWs({ activeTab: "tasks" });

    fireEvent.click(await screen.findByRole("button", { name: "Run 1 queued" }));
    await waitFor(() => expect(runNext).toHaveBeenCalledTimes(1));
    expect(window.confirm).toHaveBeenCalled();
  });

  it("dispatches nothing when the operator declines the confirmation", async () => {
    list.mockResolvedValue([mission({ state: "queued", task: "first" })]);
    vi.spyOn(window, "confirm").mockReturnValue(false);
    renderWs({ activeTab: "tasks" });

    fireEvent.click(await screen.findByRole("button", { name: "Run 1 queued" }));
    await waitFor(() => expect(window.confirm).toHaveBeenCalled());
    expect(runNext).not.toHaveBeenCalled();
  });

  it("blocks the run control while a mission is already running", async () => {
    // a running mission suppresses the control even though work is queued
    list.mockResolvedValue([
      mission({ id: "r", state: "running", task: "in flight" }),
      mission({ id: "q", state: "queued", task: "waiting" }),
    ]);
    renderWs({ activeTab: "tasks" });

    const btn = await screen.findByRole("button", { name: "Run in progress…" });
    expect(btn).toBeDisabled();
    fireEvent.click(btn);
    expect(runNext).not.toHaveBeenCalled();
  });

  it("shows the running mission on the composer tab", async () => {
    list.mockResolvedValue([mission({ state: "running", task: "in flight" })]);
    renderWs({ activeTab: "agent" });
    expect(await screen.findByText("in flight")).toBeInTheDocument();
    expect(screen.getByText("running:")).toBeInTheDocument();
  });

  it("renders mission state markers, summaries and ledger sessions", async () => {
    list.mockResolvedValue([
      mission({ id: "a", state: "done", task: "done task", summary: "all green" }),
      mission({ id: "b", state: "failed", task: "failed task" }),
      mission({
        id: "c",
        state: "queued",
        task: "queued task",
        ledgerSession: "session-12345678",
        updatedAt: new Date(Date.now() - 7200_000).toISOString(),
      }),
    ]);
    renderWs({ activeTab: "tasks" });

    expect(await screen.findByText("● done")).toBeInTheDocument();
    expect(screen.getByText("✕ failed")).toBeInTheDocument();
    expect(screen.getByText("○ queued")).toBeInTheDocument();
    expect(screen.getByText("all green")).toBeInTheDocument();
    expect(screen.getByText("ledger: session-")).toBeInTheDocument();
    // two hours old -> hours bucket
    expect(screen.getByText("2h")).toBeInTheDocument();
  });

  it("renders the evidence detail for a ready ledger", async () => {
    evidence.mockResolvedValue(LEDGER);
    renderWs({ activeTab: "evidence" });

    expect(await screen.findByText("tool.exec")).toBeInTheDocument();
    // session id is truncated to its first 8 characters
    expect(screen.getByText(/sess-abc/)).toBeInTheDocument();
    expect(screen.queryByText("loading…")).not.toBeInTheDocument();
  });

  it("renders the unavailable evidence reason verbatim", async () => {
    evidence.mockResolvedValue({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to read the evidence ledger",
    });
    renderWs({ activeTab: "evidence" });

    // shown once in the status strip and once in the evidence tab
    expect(
      await screen.findAllByText("start `zylcode serve-intel` to read the evidence ledger"),
    ).toHaveLength(2);
    expect(screen.queryByText("✓ hash chain intact")).not.toBeInTheDocument();
  });

  it("reports BLOCKED status when the ledger is unreachable", async () => {
    evidence.mockResolvedValue({ kind: "unavailable", reason: "backend down" });
    renderWs();
    await waitFor(() => expect(screen.getByText("BLOCKED")).toBeInTheDocument());
    expect(screen.getByText("backend down")).toBeInTheDocument();
  });

  it("releases its poll interval on unmount", async () => {
    const clearIntervalSpy = vi.spyOn(globalThis, "clearInterval");
    const { unmount } = renderWs();
    await waitFor(() => expect(list).toHaveBeenCalled());

    unmount();
    expect(clearIntervalSpy).toHaveBeenCalled();
    clearIntervalSpy.mockRestore();
  });
});
