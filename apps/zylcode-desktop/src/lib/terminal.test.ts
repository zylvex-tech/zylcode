// Transport-classification contract for the terminal service (Live
// Commissioning §2): `transportError` is what lets the UI distinguish
// "the backend is gone" (→ BLOCKED + auto-recovery) from "the command
// failed" (→ health unchanged). The old implementation inferred this by
// string-prefix sniffing stderrTail; these tests pin the explicit flag.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { terminalExec } from "./terminal";

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
