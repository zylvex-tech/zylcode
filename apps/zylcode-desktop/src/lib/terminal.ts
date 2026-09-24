// ---------------------------------------------------------------------------
// Terminal service (lib/terminal.ts)
//
// Transport parity with lib/repoIntel.ts: the Tauri desktop app calls the
// `terminal_exec` IPC command (shared in-process TerminalHub); the browser
// preview POSTs to /api/terminal/exec through the Vite proxy. Failures
// resolve to a TerminalOutput with the error on stderr — a broken terminal
// reports why, it never renders fake output.
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic } from "./runtime";

export interface TerminalRequest {
  sessionId?: string | null;
  cwd?: string | null;
  command: string;
  timeoutSecs?: number;
}

export interface TerminalOutput {
  sessionId: string;
  cwd: string;
  exitCode: number | null;
  timedOut: boolean;
  stdoutTail: string;
  stderrTail: string;
  truncated: boolean;
  durationMs: number;
}

function failed(question: string, detail: string): TerminalOutput {
  return {
    sessionId: "",
    cwd: "",
    exitCode: null,
    timedOut: false,
    stdoutTail: "",
    stderrTail: `${question}: ${detail}`,
    truncated: false,
    durationMs: 0,
  };
}

const isDesktop = () => detectEnvironment() === "TAURI_DESKTOP";

/** camelCase for the HTTP JSON contract; the Tauri command takes snake_case. */
function toSnake(req: TerminalRequest): Record<string, unknown> {
  return {
    session_id: req.sessionId ?? null,
    cwd: req.cwd ?? null,
    command: req.command,
    timeout_secs: req.timeoutSecs ?? null,
  };
}

export async function terminalExec(req: TerminalRequest): Promise<TerminalOutput> {
  if (!req.command.trim()) {
    return failed("terminal exec rejected", "command must not be empty");
  }
  try {
    if (isDesktop()) {
      const { invoke } = await import("@tauri-apps/api/core");
      const raw = await invoke<Record<string, unknown>>("terminal_exec", {
        request: toSnake(req),
      });
      return normalize(raw);
    }
    const res = await fetch("/api/terminal/exec", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(toSnake(req)),
    });
    if (!res.ok) {
      return failed("terminal service error", `HTTP ${res.status}`);
    }
    return normalize(await res.json());
  } catch (err) {
    recordDiagnostic("terminal", `exec failed: ${String(err)}`);
    return failed(
      "terminal unavailable",
      "start `zylcode serve-intel` (browser) or use the desktop app (Tauri IPC)",
    );
  }
}

function normalize(raw: Record<string, unknown>): TerminalOutput {
  return {
    sessionId: String(raw.sessionId ?? raw.session_id ?? ""),
    cwd: String(raw.cwd ?? ""),
    exitCode:
      typeof raw.exitCode === "number"
        ? raw.exitCode
        : typeof raw.exit_code === "number"
          ? (raw.exit_code as number)
          : null,
    timedOut: Boolean(raw.timedOut ?? raw.timed_out ?? false),
    stdoutTail: String(raw.stdoutTail ?? raw.stdout_tail ?? ""),
    stderrTail: String(raw.stderrTail ?? raw.stderr_tail ?? ""),
    truncated: Boolean(raw.truncated ?? false),
    durationMs: Number(raw.durationMs ?? raw.duration_ms ?? 0),
  };
}

export async function terminalReset(): Promise<boolean> {
  try {
    if (isDesktop()) return true; // sessions live with the app process
    const res = await fetch("/api/terminal/reset", { method: "POST" });
    return res.ok;
  } catch {
    return false;
  }
}
