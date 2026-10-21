// Regression suite for the stale-terminal-recovery defect (Live
// Commissioning §2): a backend that starts *after* the frontend must
// recover without a page reload, a backend that dies mid-session must flip
// the badge back to BLOCKED, ordinary command failures must NOT change
// health, and a failed session reset must not claim success.

import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TerminalPanel, RECOVERY_INTERVAL_MS } from "./TerminalPanel";
import { terminalExec, terminalReset } from "../lib/terminal";
import type { TerminalOutput } from "../lib/terminal";

// ---------------------------------------------------------------------------
// xterm mocks — capture everything the panel writes so tests can assert on
// terminal *content* (recovery/reset messages), not just badge state.
// ---------------------------------------------------------------------------

const sink = vi.hoisted(() => ({
  writes: [] as string[],
  dataHandler: null as ((d: string) => void) | null,
}));

vi.mock("@xterm/xterm", () => {
  class Terminal {
    write(s: string) {
      sink.writes.push(s);
    }
    writeln(s: string) {
      sink.writes.push(`${s}\r\n`);
    }
    clear() {}
    dispose() {}
    open() {}
    loadAddon() {}
    onData(cb: (d: string) => void) {
      sink.dataHandler = cb;
    }
  }
  return { Terminal };
});

vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit() {}
    loadAddon() {}
  },
}));

vi.mock("../lib/terminal", () => ({
  terminalExec: vi.fn(),
  terminalReset: vi.fn(),
}));

const execMock = vi.mocked(terminalExec);
const resetMock = vi.mocked(terminalReset);

/** Transport failure: the service is unreachable. */
function down(): TerminalOutput {
  return {
    sessionId: "",
    cwd: "",
    exitCode: null,
    timedOut: false,
    stdoutTail: "",
    stderrTail:
      "terminal unavailable: start `zylcode serve-intel` (browser) or use the desktop app (Tauri IPC)",
    truncated: false,
    durationMs: 0,
    transportError: true,
  };
}

/** Healthy probe / command: marker output, transport fine. */
function up(sessionId = "s1"): TerminalOutput {
  return {
    sessionId,
    cwd: "/workspace",
    exitCode: 0,
    timedOut: false,
    stdoutTail: "zylcode_terminal_ready",
    stderrTail: "",
    truncated: false,
    durationMs: 1,
    transportError: false,
  };
}

/** A command that ran fine but failed (exit 1) — must not affect health. */
function cmdFailed(): TerminalOutput {
  return {
    sessionId: "s1",
    cwd: "/workspace",
    exitCode: 1,
    timedOut: false,
    stdoutTail: "",
    stderrTail: "command not found",
    truncated: false,
    durationMs: 2,
    transportError: false,
  };
}

/** Flush pending microtasks (mocked dynamic imports + probe awaits). */
function flush() {
  return act(async () => {
    for (let i = 0; i < 10; i++) await Promise.resolve();
  });
}

function type(line: string) {
  act(() => {
    for (const ch of line) sink.dataHandler?.(ch);
    sink.dataHandler?.("\r");
  });
}

const terminalText = () => sink.writes.join("");

beforeEach(() => {
  vi.useFakeTimers();
  sink.writes.length = 0;
  sink.dataHandler = null;
  execMock.mockReset();
  resetMock.mockReset();
  resetMock.mockResolvedValue(true);
});

afterEach(() => {
  vi.useRealTimers();
});

describe("TerminalPanel backend recovery (stale BLOCKED regression)", () => {
  it("reports BLOCKED truthfully at mount, then recovers to AVAILABLE without a reload", async () => {
    execMock.mockResolvedValue(down());

    render(<TerminalPanel />);
    await flush();

    // Backend down → honest BLOCKED with the real reason on the badge note.
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
    expect(screen.getByText(/terminal unavailable: start/)).toBeInTheDocument();
    const probesWhileDown = execMock.mock.calls.length;
    expect(probesWhileDown).toBeGreaterThanOrEqual(1);

    // Still down after a re-probe cycle: no false recovery.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(RECOVERY_INTERVAL_MS);
    });
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
    expect(execMock.mock.calls.length).toBeGreaterThan(probesWhileDown);

    // Backend comes back → next automatic probe recovers the surface.
    execMock.mockResolvedValue(up());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(RECOVERY_INTERVAL_MS);
    });

    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();
    expect(screen.getByText("real shell · cwd persists per session")).toBeInTheDocument();
    // The recovery is announced in the terminal itself…
    expect(terminalText()).toContain("terminal service recovered");
    // …and came from real re-probes, not a reload or a lucky first render.
    expect(execMock.mock.calls.length).toBeGreaterThanOrEqual(3);
  });

  it("flips back to BLOCKED when the backend dies mid-session, then recovers", async () => {
    execMock.mockResolvedValue(up());

    render(<TerminalPanel />);
    await flush();
    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();

    // Backend dies: the next command's transport failure is the evidence.
    execMock.mockResolvedValue(down());
    type("ls");
    await flush();

    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
    expect(terminalText()).toContain("terminal unavailable");

    // Backend returns → the armed recovery loop restores the badge.
    execMock.mockResolvedValue(up("s2"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(RECOVERY_INTERVAL_MS);
    });

    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();
    expect(terminalText()).toContain("terminal service recovered");
  });

  it("does NOT change health when a command merely fails (exit 1)", async () => {
    execMock.mockResolvedValue(up());

    render(<TerminalPanel />);
    await flush();
    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();

    execMock.mockResolvedValue(cmdFailed());
    type("does-not-exist");
    await flush();

    // The failure output is shown, but a failed command is not a dead
    // backend — the badge must stay AVAILABLE.
    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();
    expect(terminalText()).toContain("command not found");
    expect(terminalText()).toContain("exit 1");
  });

  it("recovers automatically when the user's own command succeeds before the probe fires", async () => {
    execMock.mockResolvedValue(down());

    render(<TerminalPanel />);
    await flush();
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();

    // Backend is back but the interval has not fired yet: the user types.
    execMock.mockResolvedValue(up());
    type("echo hi");
    await flush();

    // Evidence of a working transport overrides the stale badge immediately.
    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();
  });

  it("reports a failed session reset honestly instead of claiming success", async () => {
    execMock.mockResolvedValue(up());
    resetMock.mockResolvedValue(false);

    render(<TerminalPanel />);
    await flush();

    fireEvent.click(screen.getByRole("button", { name: "Reset session" }));
    await flush();

    expect(terminalText()).toContain("reset failed");
    expect(terminalText()).not.toContain("— session reset —");
    expect(screen.getByText("BLOCKED")).toBeInTheDocument();
  });

  it("keeps the normal reset path intact when the service accepts it", async () => {
    execMock.mockResolvedValue(up());
    resetMock.mockResolvedValue(true);

    render(<TerminalPanel />);
    await flush();

    fireEvent.click(screen.getByRole("button", { name: "Reset session" }));
    await flush();

    expect(terminalText()).toContain("— session reset —");
    expect(screen.getByText("AVAILABLE")).toBeInTheDocument();
  });
});
