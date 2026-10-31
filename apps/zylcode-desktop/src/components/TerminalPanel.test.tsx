// Regression suite for the stale-terminal-recovery defect (Live
// Commissioning §2): a backend that starts *after* the frontend must
// recover without a page reload, a backend that dies mid-session must flip
// the badge back to BLOCKED, ordinary command failures must NOT change
// health, and a failed session reset must not claim success.

import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TerminalPanel, RECOVERY_INTERVAL_MS } from "./TerminalPanel";
import { isClearCommand } from "../lib/terminal";
import { terminalExec, terminalReset } from "../lib/terminal";
import type { TerminalOutput } from "../lib/terminal";

// ---------------------------------------------------------------------------
// xterm mocks — capture everything the panel writes so tests can assert on
// terminal *content* (recovery/reset messages), not just badge state.
// ---------------------------------------------------------------------------

const sink = vi.hoisted(() => ({
  writes: [] as string[],
  dataHandler: null as ((d: string) => void) | null,
  /** How many times the panel asked xterm to wipe the buffer. */
  clears: 0,
  /** How many times the panel asked the fit addon to re-measure. */
  fits: 0,
}));

vi.mock("@xterm/xterm", () => {
  class Terminal {
    write(s: string) {
      sink.writes.push(s);
    }
    writeln(s: string) {
      sink.writes.push(`${s}\r\n`);
    }
    clear() {
      sink.clears += 1;
    }
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
    fit() {
      sink.fits += 1;
    }
    loadAddon() {}
  },
}));

// `availableNote` / `isClearCommand` are pure helpers that moved into
// `../lib/terminal`; only the transport is stubbed.
vi.mock("../lib/terminal", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/terminal")>();
  return { ...actual, terminalExec: vi.fn(), terminalReset: vi.fn() };
});

const execMock = vi.mocked(terminalExec);
const resetMock = vi.mocked(terminalReset);

/** Transport failure: the service is unreachable. */
function down(): TerminalOutput {
  return {
    sessionId: "",
    cwd: "",
    shell: "",
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
    shell: "powershell",
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
    shell: "powershell",
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
  sink.clears = 0;
  sink.fits = 0;
  execMock.mockReset();
  resetMock.mockReset();
  resetMock.mockResolvedValue(true);
});

afterEach(() => {
  vi.useRealTimers();
});

describe("TerminalPanel clearing", () => {
  // `clear`/`cls` cannot work as shell commands here: every command is a fresh
  // non-interactive process, so `Clear-Host` has no screen to clear. Clearing
  // is therefore a presentation-layer operation — it must wipe the transcript
  // and nothing else (no shell round-trip, no cwd reset, no shell change).

  it("isClearCommand matches only a whole clear/cls line", () => {
    for (const yes of ["clear", "cls", "  CLEAR  ", "Cls", "clear\n"]) {
      expect(isClearCommand(yes), `${JSON.stringify(yes)} should clear`).toBe(true);
    }
    for (const no of ["", "echo clear", "clear; git status", "cls -Force", "clears", "cls;ls"]) {
      expect(isClearCommand(no), `${JSON.stringify(no)} should not clear`).toBe(false);
    }
  });

  it("`clear` wipes the transcript without running anything in the shell", async () => {
    execMock.mockResolvedValue(up());
    render(<TerminalPanel />);
    await flush();
    const execCallsBefore = execMock.mock.calls.length;

    sink.writes.length = 0;
    type("clear");
    await flush();

    expect(sink.clears).toBe(1);
    expect(execMock.mock.calls.length).toBe(execCallsBefore);
  });

  it("`cls` clears too, and leaves the working directory and shell untouched", async () => {
    execMock.mockResolvedValue(up());
    render(<TerminalPanel />);
    await flush();

    // Pick a non-default shell so an accidental reset would be visible.
    fireEvent.change(screen.getByLabelText("shell"), { target: { value: "cmd" } });
    const execCallsBefore = execMock.mock.calls.length;

    sink.writes.length = 0;
    type("cls");
    await flush();

    expect(sink.clears).toBe(1);
    expect(execMock.mock.calls.length).toBe(execCallsBefore);
    // cwd survived: the prompt still names the session directory
    // ("workspace"), not the "zylcode" placeholder used when cwd is empty.
    expect(terminalText()).toContain("workspace");
    expect((screen.getByLabelText("shell") as HTMLSelectElement).value).toBe("cmd");
  });

  it("the Clear button uses the same implementation", async () => {
    execMock.mockResolvedValue(up());
    render(<TerminalPanel />);
    await flush();
    const execCallsBefore = execMock.mock.calls.length;

    fireEvent.click(screen.getByRole("button", { name: "Clear" }));
    await flush();

    expect(sink.clears).toBe(1);
    expect(execMock.mock.calls.length).toBe(execCallsBefore);
  });

  it("does not treat commands that merely contain clear/cls as a clear", async () => {
    execMock.mockResolvedValue(up());
    render(<TerminalPanel />);
    await flush();

    for (const line of ["echo clear", "clear; git status", "cls -Force"]) {
      sink.clears = 0;
      const before = execMock.mock.calls.length;
      type(line);
      await flush();
      expect(sink.clears, `${line} must not clear`).toBe(0);
      expect(execMock.mock.calls.length, `${line} must reach the shell`).toBe(before + 1);
    }
  });
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
    // The badge names the shell the backend actually reported — no more
    // generic "real shell" claim.
    expect(
      screen.getByText("powershell · command runner · cwd persists per session"),
    ).toBeInTheDocument();
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

describe("TerminalPanel shell selection (shell-mismatch regression)", () => {
  it("defaults to the platform shell (auto) and forwards it on every exec", async () => {
    execMock.mockResolvedValue(up());

    render(<TerminalPanel />);
    await flush();

    // The selector exists, starts on auto…
    const select = screen.getByRole("combobox", { name: "shell" }) as HTMLSelectElement;
    expect(select.value).toBe("auto");
    // …and the mount probe already said so.
    expect(execMock.mock.calls[0][0].shell).toBe("auto");

    // A command typed afterwards carries the same mode.
    type("Write-Output hi");
    await flush();
    const last = execMock.mock.calls[execMock.mock.calls.length - 1];
    expect(last[0].shell).toBe("auto");
  });

  it("forwards an explicitly chosen shell on subsequent commands", async () => {
    execMock.mockResolvedValue(up());

    render(<TerminalPanel />);
    await flush();

    fireEvent.change(screen.getByRole("combobox", { name: "shell" }), {
      target: { value: "cmd" },
    });
    type("dir");
    await flush();

    // The command must be executed as cmd — never silently routed to the
    // default shell (the original defect: PowerShell syntax sent to cmd).
    const last = execMock.mock.calls[execMock.mock.calls.length - 1];
    expect(last[0].shell).toBe("cmd");
  });

  it("advertises the honest model: a command runner, one process per command", async () => {
    execMock.mockResolvedValue(up());

    render(<TerminalPanel />);
    await flush();

    const text = terminalText();
    expect(text).toContain("commands run for real in your selected shell");
    expect(text).toContain("command runner, not an interactive PTY");
    expect(text).toContain("cwd persists per session");
    // No claim of an interactive shell session.
    expect(text).not.toContain("interactive shell ready");
  });
});

describe("TerminalPanel input and layout", () => {
  it("refits the terminal when the window is resized", async () => {
    execMock.mockResolvedValue(up());
    render(<TerminalPanel />);
    await flush();

    const before = sink.fits;
    expect(before).toBeGreaterThanOrEqual(1);

    fireEvent(window, new Event("resize"));

    expect(sink.fits).toBe(before + 1);
  });

  it("erases the last character on backspace and stops at an empty line", async () => {
    execMock.mockResolvedValue(up());
    render(<TerminalPanel />);
    await flush();
    sink.writes.length = 0;

    act(() => {
      sink.dataHandler?.("a");
      sink.dataHandler?.("b");
      sink.dataHandler?.("\u007f");
      // A fourth backspace on an empty line must be a no-op, not a crash.
      sink.dataHandler?.("\u007f");
      sink.dataHandler?.("\u007f");
    });

    expect(terminalText()).toBe("ab\b \b\b \b");
  });

  it("reprints the prompt on an empty Enter instead of calling the shell", async () => {
    execMock.mockResolvedValue(up());
    render(<TerminalPanel />);
    await flush();
    const before = execMock.mock.calls.length;
    sink.writes.length = 0;

    act(() => sink.dataHandler?.("\r"));
    await flush();

    expect(execMock.mock.calls.length).toBe(before);
    expect(sink.writes.join("")).toContain("$");
  });

  it("marks truncated output in the transcript so a cut tail is visible", async () => {
    execMock.mockResolvedValue(up());
    render(<TerminalPanel />);
    await flush();

    execMock.mockResolvedValue({ ...up(), stdoutTail: "partial tail", truncated: true });
    type("generate-a-huge-report");
    await flush();

    expect(terminalText()).toContain("output truncated — showing tail");
  });
});
