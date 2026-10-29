import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { BottomPanel, type BottomTab } from "./BottomPanel";
import { clearDiagnostics, recordDiagnostic } from "../../lib/runtime";
import type { EvidenceState } from "../../lib/surfaces";
import type { Mission } from "../../lib/missions";
import type { GitStatusState } from "../../lib/gitStatus";
import type { ProvidersState } from "../../lib/service";

// ---------------------------------------------------------------------------
// Bottom panel (components/shell/BottomPanel.tsx).
//
// Every tab must be honest: Output shows only diagnostics actually recorded
// at runtime, Problems reports only real untracked files / failed missions,
// Ports lists ports genuinely served, Tests/Evidence come straight from the
// ledger, and Debug Console states plainly that no debugger is commissioned.
// Heavy leaf panels are replaced with stubs here; they have their own tests.
// ---------------------------------------------------------------------------

vi.mock("../TerminalPanel", () => ({
  default: () => <div data-testid="terminal-stub" />,
}));
vi.mock("../ToolsCataloguePanel", () => ({
  default: () => <div data-testid="tools-stub" />,
}));
vi.mock("../FoundationPanel", () => ({
  FoundationPanel: () => <div data-testid="foundation-stub" />,
}));
vi.mock("../DeliveryPanel", () => ({
  DeliveryPanel: () => <div data-testid="delivery-stub" />,
}));
vi.mock("../../components", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../components")>();
  return {
    ...actual,
    McpInspector: () => <div data-testid="mcp-stub" />,
    ProviderSettings: () => <div data-testid="provider-settings-stub" />,
    DiagnosticsPanel: () => <div data-testid="diagnostics-stub" />,
  };
});
vi.mock("../../lib/missions", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/missions")>();
  return { ...actual, listMissions: vi.fn() };
});
vi.mock("../../lib/surfaces", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/surfaces")>();
  return { ...actual, fetchEvidence: vi.fn() };
});
vi.mock("../../lib/service", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/service")>();
  return { ...actual, fetchProviders: vi.fn() };
});
vi.mock("../../lib/gitStatus", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/gitStatus")>();
  return { ...actual, fetchGitStatus: vi.fn() };
});

// jsdom has no PointerEvent; fall back to MouseEvent so clientY survives.
if (typeof window.PointerEvent === "undefined") {
  (window as unknown as { PointerEvent: typeof MouseEvent }).PointerEvent = class PointerEvent
    extends MouseEvent {
    pointerId: number;
    constructor(type: string, init: PointerEventInit = {}) {
      super(type, init);
      this.pointerId = init.pointerId ?? 0;
    }
  };
}

const { listMissions } = await import("../../lib/missions");
const { fetchEvidence } = await import("../../lib/surfaces");
const { fetchProviders } = await import("../../lib/service");
const { fetchGitStatus } = await import("../../lib/gitStatus");

function evidence(overrides: Partial<Extract<EvidenceState, { kind: "ready" }>> = {}): Extract<
  EvidenceState,
  { kind: "ready" }
> {
  return {
    kind: "ready",
    chain_intact: true,
    session_count: 1,
    entry_count: 2,
    entries: [
      {
        id: "e1",
        session_id: "sess-0123456789ab",
        action_id: "mission.started",
        state: "Started",
        timestamp: "2026-10-28T10:00:00.000Z",
        error: null,
        payload: null,
      },
      {
        id: "e2",
        session_id: "sess-0123456789ab",
        action_id: "selection.final",
        state: "Recorded",
        timestamp: "2026-10-28T10:00:05.000Z",
        error: null,
        payload: {
          outcomes: [{ candidate_index: 0, passed: true, passing_checks: 7 }],
          reason: "all checks passed",
        },
      },
    ],
    ...overrides,
  };
}

function gitReady(entries: { status: string; path: string }[]): GitStatusState {
  return {
    kind: "ready",
    via: "service",
    data: {
      branch: "main",
      upstream: null,
      ahead: 0,
      behind: 0,
      head: "abc1234",
      clean: entries.length === 0,
      counts: {
        total: entries.length,
        staged: 0,
        unstaged: 0,
        untracked: entries.filter((e) => e.status === "??").length,
      },
      entries,
      diffstat: { files_changed: 0, insertions: 0, deletions: 0, rows: [] },
    },
  };
}

const mission = (over: Partial<Mission> = {}): Mission => ({
  id: "12345678-abcd-efgh",
  task: "add a login form",
  mode: "build",
  state: "failed",
  createdAt: "2026-10-28T10:00:00.000Z",
  updatedAt: "2026-10-28T10:00:00.000Z",
  ...over,
});

function Harness({
  initial = "terminal",
  isOpen = true,
  onClose = vi.fn(),
}: {
  initial?: BottomTab;
  isOpen?: boolean;
  onClose?: () => void;
}) {
  const [tab, setTab] = useState<BottomTab>(initial);
  return (
    <BottomPanel isOpen={isOpen} onToggle={onClose} activeTab={tab} onTabChange={setTab} />
  );
}

const evi = vi.mocked(fetchEvidence);
const mis = vi.mocked(listMissions);
const prov = vi.mocked(fetchProviders);
const git = vi.mocked(fetchGitStatus);

beforeEach(() => {
  clearDiagnostics();
  evi.mockReset().mockResolvedValue({ kind: "unavailable", reason: "ledger offline" });
  mis.mockReset().mockResolvedValue([]);
  prov.mockReset().mockResolvedValue({ kind: "unavailable", reason: "providers offline" });
  git.mockReset().mockResolvedValue(gitReady([]));
});

afterEach(() => {
  vi.useRealTimers(); // belt-and-braces: never leak fake timers into the next test
  vi.clearAllMocks();
});

describe("BottomPanel", () => {
  it("renders nothing while closed and toggles from the close button", () => {
    const onClose = vi.fn();
    const { rerender } = render(<Harness isOpen={false} onClose={onClose} />);
    expect(screen.queryByRole("separator")).not.toBeInTheDocument();
    expect(screen.queryByText("Terminal")).not.toBeInTheDocument();

    rerender(<Harness isOpen={true} onClose={onClose} />);
    expect(screen.getByRole("separator")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Close panel" }));
    expect(onClose).toHaveBeenCalled();
  });

  it("offers all ten tabs and swaps content per tab", async () => {
    render(<Harness />);

    for (const label of [
      "Terminal",
      "Output",
      "Problems",
      "Debug Console",
      "Ports",
      "Tests",
      "Evidence",
      "Dev Tools",
      "Foundation",
      "Delivery",
    ]) {
      expect(screen.getByRole("button", { name: label })).toBeInTheDocument();
    }

    fireEvent.click(screen.getByRole("button", { name: "Terminal" }));
    expect(screen.getByTestId("terminal-stub")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Foundation" }));
    expect(screen.getByTestId("foundation-stub")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Delivery" }));
    expect(screen.getByTestId("delivery-stub")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Debug Console" }));
    expect(screen.getByText("Debug Console — not available.")).toBeInTheDocument();
    expect(
      screen.getByText(/No debugger integration is commissioned in this build/),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Ports" }));
    expect(
      screen.getByText("zylcode serve-intel (repo intelligence API)"),
    ).toBeInTheDocument();
    expect(screen.getByText("This app (Vite dev server)")).toBeInTheDocument();
    expect(
      screen.getByText("Ports actually served by this workspace. Nothing here is invented."),
    ).toBeInTheDocument();
  });

  it("resizes with the drag handle, clamped between 120px and 70% of the viewport", () => {
    render(<Harness />);
    const handle = screen.getByRole("separator");
    Object.assign(handle, {
      setPointerCapture: vi.fn(),
      releasePointerCapture: vi.fn(),
    });
    const panel = document.querySelector('div[class*="panel-h"]') as HTMLElement;
    expect(panel.style.height).toBe("240px");

    fireEvent.pointerDown(handle, { clientY: 500, pointerId: 1 });
    fireEvent.pointerMove(handle, { clientY: 400, pointerId: 1 });
    expect(panel.style.height).toBe("340px");

    // Clamped at the 120px floor.
    fireEvent.pointerMove(handle, { clientY: 5000, pointerId: 1 });
    expect(panel.style.height).toBe("120px");

    fireEvent.pointerUp(handle, { clientY: 5000, pointerId: 1 });
    expect(handle.releasePointerCapture).toHaveBeenCalled();
    // No drag state left: further moves do nothing.
    fireEvent.pointerMove(handle, { clientY: 0, pointerId: 1 });
    expect(panel.style.height).toBe("120px");
  });

  it("Output shows only diagnostics actually recorded at runtime", async () => {
    recordDiagnostic("git", "status fetch failed");
    render(<Harness initial="output" />);

    expect(await screen.findByText("status fetch failed")).toBeInTheDocument();
    expect(document.body.textContent).toContain("[git]");
  });

  it("Problems reports real untracked files and failed missions", async () => {
    git.mockResolvedValue(gitReady([{ status: "??", path: "scratch.md" }]));
    mis.mockResolvedValue([mission()]);
    render(<Harness initial="problems" />);

    expect(
      await screen.findByText("Untracked file — not part of any commit or verification run"),
    ).toBeInTheDocument();
    expect(screen.getByText("Mission failed: add a login form")).toBeInTheDocument();
    expect(screen.getByText("mission 12345678")).toBeInTheDocument();
    expect(screen.getByText("scratch.md")).toBeInTheDocument();
  });

  it("Problems is honestly empty when the workspace is clean", async () => {
    git.mockResolvedValue(gitReady([]));
    mis.mockResolvedValue([mission({ state: "done" })]);
    render(<Harness initial="problems" />);
    expect(await screen.findByText(/No problems detected/)).toBeInTheDocument();
  });

  it("Problems surfaces the backend's own reason when git status is unavailable", async () => {
    git.mockResolvedValue({ kind: "unavailable", reason: "not a git repository" });
    render(<Harness initial="problems" />);
    expect(await screen.findByText("not a git repository")).toBeInTheDocument();
  });

  it("Tests tab reads verification runs out of the evidence ledger", async () => {
    evi.mockResolvedValue(evidence());
    render(<Harness initial="tests" />);

    expect(await screen.findByText("all checks passed")).toBeInTheDocument();
    expect(screen.getByText("sess-012")).toBeInTheDocument();
    expect(screen.getByText("7 checks")).toBeInTheDocument();
    expect(screen.getByText("✓")).toBeInTheDocument();
  });

  it("Tests tab states loading, unavailable and empty honestly", async () => {
    // loading
    evi.mockResolvedValue({ kind: "loading" });
    const first = render(<Harness initial="tests" />);
    expect(await screen.findByText("Reading the evidence ledger…")).toBeInTheDocument();
    first.unmount();

    // empty → honest "nothing recorded yet", not a fake pass
    evi.mockResolvedValue({ kind: "empty", reason: "ledger has no entries yet" });
    const second = render(<Harness initial="tests" />);
    expect(
      await screen.findByText(/No verification runs recorded yet/),
    ).toBeInTheDocument();
    second.unmount();

    // unavailable → the backend's own reason
    evi.mockResolvedValue({ kind: "unavailable", reason: "serve-intel is down" });
    render(<Harness initial="tests" />);
    expect(await screen.findByText("serve-intel is down")).toBeInTheDocument();
  });

  it("Evidence tab renders the chain verdict from real ledger state", async () => {
    evi.mockResolvedValue(evidence({ chain_intact: false, entry_count: 5, session_count: 2 }));
    render(<Harness initial="evidence" />);

    expect(await screen.findByText("✕ CHAIN BROKEN")).toBeInTheDocument();
    expect(screen.getByText("— 5 entries across 2 sessions")).toBeInTheDocument();
    expect(screen.getByText("mission.started")).toBeInTheDocument();
  });

  it("Dev Tools renders catalogue, providers and diagnostics with the real provider chain", async () => {
    prov.mockResolvedValue({
      kind: "ready",
      primary_model: "claude-sonnet",
      fallback_model: "gpt-4o",
      chain: [
        {
          kind: "anthropic",
          model: "claude-sonnet",
          enabled: true,
          fallback_order: 0,
          requires_api_key: true,
          timeout_ms: 60_000,
        },
      ],
      measured_ranking: [
        {
          provider: "anthropic",
          successes: 9,
          failures: 1,
          score: 0.833,
          avg_latency_ms: 1250.4,
        },
      ],
    });
    render(<Harness initial="dev-tools" />);

    expect(screen.getByText("MCP Tool Catalogue")).toBeInTheDocument();
    expect(screen.getByText("Providers")).toBeInTheDocument();
    expect(screen.getByText("Diagnostics")).toBeInTheDocument();
    expect(screen.getByTestId("diagnostics-stub")).toBeInTheDocument();

    expect((await screen.findAllByText("anthropic"))).toHaveLength(2);
    expect(screen.getByText("key required")).toBeInTheDocument();
    expect(screen.getByText("enabled")).toBeInTheDocument();
    expect(screen.getByText("9✓ / 1✕")).toBeInTheDocument();
    expect(screen.getByText("score 0.833 · 1250ms avg")).toBeInTheDocument();
  });

  it("Providers view says so when no measured outcomes exist yet", async () => {
    prov.mockResolvedValue({
      kind: "ready",
      primary_model: "claude-sonnet",
      fallback_model: "gpt-4o",
      chain: [],
      measured_ranking: [],
    });
    render(<Harness initial="dev-tools" />);
    expect(
      await screen.findByText(/No measured outcomes yet/),
    ).toBeInTheDocument();
  });

  it("Providers view shows the backend reason when the chain cannot be read", async () => {
    prov.mockResolvedValue({ kind: "unavailable", reason: "engine not running" });
    render(<Harness initial="dev-tools" />);
    expect(await screen.findByText("engine not running")).toBeInTheDocument();
  });

  it("recovers Output polling state across interval ticks", async () => {
    vi.useFakeTimers();
    render(<Harness initial="output" />);
    expect(screen.getByText(/No output yet/)).toBeInTheDocument();

    recordDiagnostic("ipc", "invoke rejected");
    // RTL's waitFor cannot poll under vitest fake timers (no global `jest`),
    // so drive the 2s interval manually and assert synchronously.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2100);
    });
    expect(screen.getByText("invoke rejected")).toBeInTheDocument();
  });

  it("keeps only the first-level tabs mounted per selection", async () => {
    evi.mockResolvedValue(evidence());
    render(<Harness initial="evidence" />);
    await screen.findByText("mission.started");
    fireEvent.click(screen.getByRole("button", { name: "Ports" }));
    expect(screen.queryByText("mission.started")).not.toBeInTheDocument();
    expect(
      screen.getByText("zylcode serve-intel (repo intelligence API)"),
    ).toBeInTheDocument();
    await waitFor(() => expect(evi).toHaveBeenCalled());
  });
});