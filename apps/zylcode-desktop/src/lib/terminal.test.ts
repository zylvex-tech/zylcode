// Transport-classification contract for the terminal service (Live
// Commissioning §2): `transportError` is what lets the UI distinguish
// "the backend is gone" (→ BLOCKED + auto-recovery) from "the command
// failed" (→ health unchanged). The old implementation inferred this by
// string-prefix sniffing stderrTail; these tests pin the explicit flag.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { availableNote, isClearCommand, terminalExec, terminalReset } from "./terminal";

// The desktop transport is reached through a dynamic import, so the Tauri
// module has to be stubbed for that branch to be exercisable in jsdom.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const fetchMock = vi.fn();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("terminalExec transport classification", () => {
  it("marks a rejected fetch (backend down) as a transport error", async () => {
    fetchMock.mockRejectedValue(new TypeError("fetch failed"));
    const out = await terminalExec({ command: "echo hi" });
    expect(out.transportError).toBe(true);
    expect(out.stderrTail).toContain("terminal unavailable");
    expect(out.stdoutTail).toBe("");
  });

  it("marks an HTTP error response as a transport error (proxy 502 etc.)", async () => {
    fetchMock.mockResolvedValue({ ok: false, status: 502 });
    const out = await terminalExec({ command: "echo hi" });
    expect(out.transportError).toBe(true);
    expect(out.stderrTail).toContain("HTTP 502");
  });

  it("marks a successful response as healthy even when the command fails", async () => {
    fetchMock.mockResolvedValue({
      ok: true,
      json: async () => ({
        session_id: "s1",
        cwd: "/w",
        exit_code: 3,
        stdout_tail: "",
        stderr_tail: "boom",
        duration_ms: 5,
      }),
    });
    const out = await terminalExec({ command: "boom" });
    expect(out.transportError).toBe(false);
    expect(out.exitCode).toBe(3);
    expect(out.stderrTail).toBe("boom");
  });

  it("does NOT classify an empty command as a transport error", async () => {
    const out = await terminalExec({ command: "   " });
    expect(out.transportError).toBe(false);
    expect(out.stderrTail).toContain("command must not be empty");
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

describe("terminalExec shell forwarding (shell-mismatch regression)", () => {
  /** Healthy snake_case response, as the backend actually serializes it. */
  function response(overrides: Record<string, unknown> = {}) {
    fetchMock.mockResolvedValue({
      ok: true,
      json: async () => ({
        session_id: "s1",
        cwd: "C:\\repo",
        shell: "powershell",
        exit_code: 0,
        stdout_tail: "ok",
        stderr_tail: "",
        duration_ms: 1,
        ...overrides,
      }),
    });
  }

  it("sends the requested shell mode to the backend in snake_case", async () => {
    response();
    await terminalExec({ command: "dir", shell: "cmd" });
    const body = JSON.parse((fetchMock.mock.calls[0][1] as { body: string }).body);
    expect(body.shell).toBe("cmd");
    expect(body.command).toBe("dir");
  });

  it("sends shell: null when none was chosen (backend default applies)", async () => {
    response();
    await terminalExec({ command: "echo hi" });
    const body = JSON.parse((fetchMock.mock.calls[0][1] as { body: string }).body);
    expect(body.shell).toBeNull();
  });

  it("surfaces the shell the backend reports alongside the output", async () => {
    response({ shell: "pwsh" });
    const out = await terminalExec({ command: "Get-Location", shell: "pwsh" });
    expect(out.shell).toBe("pwsh");
    expect(out.transportError).toBe(false);
    expect(out.stdoutTail).toBe("ok");
  });

  it("degrades to an empty shell name when an older backend omits it", async () => {
    response({ shell: undefined });
    const out = await terminalExec({ command: "echo hi" });
    expect(out.shell).toBe("");
    expect(out.exitCode).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Desktop transport: the `terminal_exec` / `terminal_reset` IPC commands.
// The environment is forced to TAURI_DESKTOP by injecting Tauri's internals
// global — exactly the signal `detectEnvironment` looks for.
// ---------------------------------------------------------------------------
const internals = globalThis as Record<string, unknown>;
const invokeMock = vi.mocked(invoke);

describe("terminalExec over desktop IPC", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    internals.__TAURI_INTERNALS__ = {};
  });
  afterEach(() => {
    delete internals.__TAURI_INTERNALS__;
    vi.unstubAllGlobals();
  });

  it("normalizes the snake_case IPC payload and skips HTTP", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    invokeMock.mockResolvedValue({
      session_id: "s9",
      cwd: "C:\\repo",
      shell: "pwsh",
      exit_code: 0,
      stdout_tail: "done",
      stderr_tail: "",
      timed_out: false,
      duration_ms: 12,
      truncated: true,
    });

    const out = await terminalExec({ command: "cargo test", shell: "pwsh" });

    expect(out.transportError).toBe(false);
    expect(out.sessionId).toBe("s9");
    expect(out.shell).toBe("pwsh");
    expect(out.exitCode).toBe(0);
    expect(out.stdoutTail).toBe("done");
    expect(out.timedOut).toBe(false);
    expect(out.truncated).toBe(true);
    expect(out.durationMs).toBe(12);
    expect(fetchMock).not.toHaveBeenCalled();
    expect(invokeMock).toHaveBeenCalledWith("terminal_exec", {
      request: {
        session_id: null,
        cwd: null,
        shell: "pwsh",
        command: "cargo test",
        timeout_secs: null,
      },
    });
  });

  it("reports an IPC rejection as unavailable with a diagnostic", async () => {
    invokeMock.mockRejectedValue(new Error("hub gone"));

    const out = await terminalExec({ command: "echo hi" });

    expect(out.transportError).toBe(true);
    expect(out.stderrTail).toContain("terminal unavailable");
  });
});

describe("terminalReset", () => {
  afterEach(() => {
    delete internals.__TAURI_INTERNALS__;
    vi.unstubAllGlobals();
    invokeMock.mockReset();
  });

  it("is true only when the desktop hub confirms the reset", async () => {
    internals.__TAURI_INTERNALS__ = {};
    invokeMock.mockResolvedValue({ status: "ok" });

    await expect(terminalReset()).resolves.toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("terminal_reset");
  });

  it("is false when the desktop hub answers with anything but ok", async () => {
    internals.__TAURI_INTERNALS__ = {};
    invokeMock.mockResolvedValue({ status: "no sessions" });

    await expect(terminalReset()).resolves.toBe(false);
  });

  it("is false when the desktop reset throws instead of claiming success", async () => {
    internals.__TAURI_INTERNALS__ = {};
    invokeMock.mockRejectedValue(new Error("hub gone"));

    await expect(terminalReset()).resolves.toBe(false);
  });

  it("POSTs to the service outside the desktop runtime", async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true });
    vi.stubGlobal("fetch", fetchMock);

    await expect(terminalReset()).resolves.toBe(true);
    expect(fetchMock).toHaveBeenCalledWith("/api/terminal/reset", { method: "POST" });
  });

  it("is false when the reset endpoint is unreachable", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("down")));

    await expect(terminalReset()).resolves.toBe(false);
  });
});

describe("terminal helpers", () => {
  it("names the shell that ran and says what persists", () => {
    expect(availableNote("pwsh")).toBe("pwsh · command runner · cwd persists per session");
    expect(availableNote("")).toBe("shell · command runner · cwd persists per session");
  });

  it("treats only a whole-line clear/cls as a transcript clear", () => {
    expect(isClearCommand("  CLEAR  ")).toBe(true);
    expect(isClearCommand("cls")).toBe(true);
    expect(isClearCommand("echo clear")).toBe(false);
    expect(isClearCommand("clear; git status")).toBe(false);
    expect(isClearCommand("")).toBe(false);
  });
});
