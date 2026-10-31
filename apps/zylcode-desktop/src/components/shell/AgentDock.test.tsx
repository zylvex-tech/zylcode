import { render, screen, fireEvent } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AgentDock } from "./AgentDock";
import { ThemeProvider } from "../ui";
import { DESKTOP_REQUIRED_MESSAGE, clearDiagnostics } from "../../lib/runtime";
import type { MissionRecord } from "../../lib/mission";

// ---------------------------------------------------------------------------
// AgentDock is a pure router: nine modules, one active slot, one callback.
// The test drives every module through the same harness so the routing table
// can never silently drop a module, and pins the desktop-only fallback —
// MCP and Providers must NOT pretend to work in a browser preview.
// ---------------------------------------------------------------------------

const BUTTONS: { id: string; label: string }[] = [
  { id: "pulse", label: "⚡ Pulse" },
  { id: "agent", label: "Agent" },
  { id: "changes", label: "Changes" },
  { id: "files", label: "Files" },
  { id: "preview", label: "Preview" },
  { id: "proof", label: "Proof" },
  { id: "mcp", label: "MCP" },
  { id: "providers", label: "Providers" },
  { id: "diagnostics", label: "Diagnostics" },
];

type Overrides = {
  activeMission?: MissionRecord | null;
  deltas?: { delta: string }[];
};

function dock(activeModule: string, overrides: Overrides = {}) {
  const onModuleChange = vi.fn();
  const view = render(
    <ThemeProvider>
      <AgentDock
        activeModule={activeModule as never}
        onModuleChange={onModuleChange}
        activeMission={overrides.activeMission ?? null}
        deltas={(overrides.deltas ?? []).map((delta, index) => ({
          index,
          delta: delta.delta,
          done: false,
        }))}
        isStreaming={false}
      />
    </ThemeProvider>,
  );
  return { ...view, onModuleChange };
}

function mission(overrides: Partial<MissionRecord> = {}): MissionRecord {
  return {
    id: "m1",
    title: "t",
    goal: "g",
    state: "completed",
    autonomy: "supervised",
    model: null,
    createdAt: "2026-10-30T00:00:00.000Z",
    updatedAt: "2026-10-30T00:00:00.000Z",
    acceptanceCriteria: [],
    deltas: [],
    toolCalls: [],
    artifacts: [],
    verification: null,
    finalOutcome: null,
    ...overrides,
  } as MissionRecord;
}

beforeEach(() => {
  vi.clearAllMocks();
  // Panels in this dock fetch relative /api/* URLs; jsdom has no base URL.
  // Answer non-2xx so every service degrades to its own honest `unavailable`
  // state instead of handing panels a payload shape they do not expect.
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => ({ ok: false, status: 503, json: async () => ({}) }) as Response),
  );
  clearDiagnostics();
});

describe("AgentDock", () => {
  it("renders all nine module buttons with their labels", () => {
    dock("pulse");
    for (const { label } of BUTTONS) {
      expect(screen.getByRole("button", { name: label })).toBeInTheDocument();
    }
  });

  it("marks only the active module as selected", () => {
    dock("proof");
    expect(screen.getByRole("button", { name: "Proof" }).className).toContain("bg-primary/10");
    expect(screen.getByRole("button", { name: "⚡ Pulse" }).className).not.toContain(
      "bg-primary/10",
    );
  });

  it("reports every module switch", () => {
    const { onModuleChange } = dock("agent");
    for (const { id, label } of BUTTONS) {
      fireEvent.click(screen.getByRole("button", { name: label }));
      expect(onModuleChange).toHaveBeenCalledWith(id);
    }
    expect(onModuleChange).toHaveBeenCalledTimes(BUTTONS.length);
  });

  it("shows the idle agent stream when there is no active mission", () => {
    dock("agent");
    expect(
      screen.getByText("No active mission. Start a mission to see the agent stream."),
    ).toBeInTheDocument();
    expect(screen.getByText("Idle. Run a mission to see the agent stream.")).toBeInTheDocument();
  });

  it("renders recent stream deltas when a mission is streaming", () => {
    dock("agent", { deltas: [{ delta: "step one" }, { delta: "step two" }] });
    expect(screen.getByText("step one")).toBeInTheDocument();
    expect(screen.getByText("step two")).toBeInTheDocument();
    expect(
      screen.queryByText("Idle. Run a mission to see the agent stream."),
    ).not.toBeInTheDocument();
  });

  it("caps the visible stream so a long mission cannot flood the dock", () => {
    const deltas = Array.from({ length: 60 }, (_, i) => ({ delta: `d${i}` }));
    dock("agent", { deltas });
    expect(screen.getByText("d59")).toBeInTheDocument();
    // only the last 40 are rendered
    expect(screen.queryByText("d0")).not.toBeInTheDocument();
    expect(screen.queryByText("d19")).not.toBeInTheDocument();
    expect(screen.getByText("d20")).toBeInTheDocument();
  });

  it("shows the proof inspector for the active mission and the empty panel otherwise", () => {
    dock("proof", { activeMission: mission() });
    expect(screen.getByText("PROOF INSPECTOR")).toBeInTheDocument();
    expect(screen.getByText("GOAL")).toBeInTheDocument();
  });

  it("states the preview surface honestly", () => {
    dock("preview");
    expect(
      screen.getByText(
        "Artifact preview opens with mission artifacts; the Run tab handles run targets.",
      ),
    ).toBeInTheDocument();
  });

  it("degrades MCP to the desktop-required notice in a browser preview", () => {
    dock("mcp");
    expect(screen.getAllByText(DESKTOP_REQUIRED_MESSAGE)).toHaveLength(1);
    expect(screen.getByText("Technical details are available in Diagnostics.")).toBeInTheDocument();
    // the real inspector's own feed must not render in a browser preview
    expect(screen.queryByText("Live feed")).not.toBeInTheDocument();
  });

  it("degrades Providers to the desktop-required notice in a browser preview", () => {
    dock("providers");
    expect(screen.getAllByText(DESKTOP_REQUIRED_MESSAGE)).toHaveLength(1);
    // the real fallback-chain editor must not render in a browser preview
    expect(screen.queryByText(/API key/)).not.toBeInTheDocument();
  });

  it("shows diagnostics and the pulse surface", () => {
    dock("diagnostics");
    expect(screen.getByText("DIAGNOSTICS")).toBeInTheDocument();
    expect(screen.getByText("No diagnostics recorded.")).toBeInTheDocument();
  });

  it("shows the changes surface", () => {
    dock("changes");
    expect(screen.queryByText("DIAGNOSTICS")).not.toBeInTheDocument();
  });
});
