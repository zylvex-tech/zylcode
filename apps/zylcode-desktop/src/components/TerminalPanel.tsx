import { useCallback, useEffect, useRef, useState } from "react";
import type { Terminal } from "@xterm/xterm";
import type { FitAddon } from "@xterm/addon-fit";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import {
  availableNote,
  isClearCommand,
  terminalExec,
  terminalReset,
  type TerminalOutput,
} from "../lib/terminal";

const BRAND_BG = "#141414"; // the logo's own dark chip color

/**
 * How often to re-probe the terminal service while it reports BLOCKED.
 * Kept short enough to feel automatic, long enough to not hammer a dead
 * backend. Exported so regression tests assert against the real value.
 */
export const RECOVERY_INTERVAL_MS = 2000;

/**
 * The shell modes the backend accepts. `auto` lets the platform choose
 * (PowerShell when present on Windows, `sh` elsewhere); the rest name a
 * concrete executable that is then used exactly or fails loudly.
 */
export const SHELL_MODES = ["auto", "powershell", "pwsh", "cmd", "sh"] as const;

/**
 * Real terminal surface: keystrokes go to a persistent backend session
 * (zylcode serve-intel HTTP route, or the Tauri terminal_exec command),
 * output comes back from actually executed commands, and the working
 * directory persists across them.
 *
 * Honest about what it is: a **command runner, not an interactive PTY** —
 * each command is one separate process in the selected shell. The shell
 * selector names the shell explicitly (`auto` = the platform default), and
 * the backend reports which shell actually ran each command in `out.shell`.
 *
 * Health contract (regression-tested):
 * - backend down at mount → BLOCKED truthfully, then re-probe every
 *   RECOVERY_INTERVAL_MS and recover to AVAILABLE automatically when the
 *   backend returns — no page reload required;
 * - backend dies mid-session → the next command's transport failure flips
 *   the badge back to BLOCKED and re-arms the same recovery loop;
 * - a command that merely *fails* (non-zero exit) never changes health;
 * - a failed session reset reports failure instead of claiming reset.
 */
export const TerminalPanel: React.FC = () => {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const sessionRef = useRef<string | null>(null);
  const cwdRef = useRef<string>("");
  const busyRef = useRef(false);
  const [status, setStatus] = useState<CapabilityStatus>("BLOCKED");
  const [statusNote, setStatusNote] = useState("connecting to terminal service…");
  const [ready, setReady] = useState(false);
  // Shell mode for new commands (`auto` = platform default). Kept in a ref
  // too so the xterm data handler and recovery probe always send the current
  // choice without being re-bound on every change.
  const [shell, setShell] = useState("auto");
  const shellRef = useRef("auto");

  // Refs used inside the xterm data handler and the recovery timer without
  // re-binding either (first-render closures stay correct via refs).
  const readyRef = useRef(false);
  const lineRef = useRef("");
  const historyRef = useRef<string[]>([]);
  const histIdxRef = useRef<number | undefined>(undefined);
  const statusRef = useRef<CapabilityStatus>("BLOCKED");
  const recoverTimerRef = useRef<number | null>(null);

  const writePrompt = useCallback(() => {
    const term = termRef.current;
    if (!term) return;
    const dir = cwdRef.current ? cwdRef.current.split(/[\\/]/).pop() || cwdRef.current : "zylcode";
    term.write(`\r\n\u001b[36m${dir}\u001b[0m \u001b[32m$\u001b[0m `);
  }, []);

  /**
   * The one implementation of "clear the visible transcript". Used by both the
   * Clear button and the `clear`/`cls` commands, so the two can never drift.
   * It touches only the xterm buffer: the session id, the working directory
   * and the selected shell are all left exactly as they were.
   */
  const clearScreen = useCallback(() => {
    termRef.current?.clear();
    writePrompt();
  }, [writePrompt]);

  const setStatusBoth = useCallback((next: CapabilityStatus, note: string) => {
    statusRef.current = next;
    setStatus(next);
    setStatusNote(note);
  }, []);

  const stopRecovery = useCallback(() => {
    if (recoverTimerRef.current !== null) {
      window.clearInterval(recoverTimerRef.current);
      recoverTimerRef.current = null;
    }
  }, []);

  /**
   * Quietly re-probe the service until it answers. On success: adopt the
   * (re)created session, flip the badge to AVAILABLE, tell the user in the
   * terminal itself, and stop the timer. Failed probes change nothing —
   * BLOCKED stays until the backend is genuinely back.
   */
  const armRecovery = useCallback(() => {
    if (recoverTimerRef.current !== null) return;
    recoverTimerRef.current = window.setInterval(async () => {
      const probe = await terminalExec({
        sessionId: sessionRef.current,
        cwd: null,
        shell: shellRef.current,
        command: "echo zylcode_terminal_ready",
      });
      if (probe.transportError) return; // still down — stay BLOCKED
      stopRecovery();
      if (probe.sessionId) sessionRef.current = probe.sessionId;
      if (probe.cwd) cwdRef.current = probe.cwd;
      setStatusBoth("AVAILABLE", availableNote(probe.shell));
      termRef.current?.write("\r\n\u001b[32m— terminal service recovered —\u001b[0m");
      writePrompt();
    }, RECOVERY_INTERVAL_MS);
  }, [setStatusBoth, stopRecovery, writePrompt]);

  const runCommand = useCallback(
    async (line: string) => {
      const term = termRef.current;
      if (!term) return;
      if (isClearCommand(line)) {
        // Presentation-layer clear: the command never reaches the shell, so it
        // cannot move the working directory, end the session or change shells.
        clearScreen();
        return;
      }
      busyRef.current = true;
      const out: TerminalOutput = await terminalExec({
        sessionId: sessionRef.current,
        cwd: cwdRef.current || null,
        shell: shellRef.current,
        command: line,
      });
      if (out.transportError) {
        // The service is gone, not the command: report BLOCKED truthfully
        // and re-arm the recovery loop.
        setStatusBoth("BLOCKED", out.stderrTail);
        term.write(`\r\n\u001b[31m${out.stderrTail.replace(/\n/g, "\r\n")}\u001b[0m`);
        busyRef.current = false;
        writePrompt();
        armRecovery();
        return;
      }
      if (statusRef.current === "BLOCKED") {
        // A command got through while the badge said BLOCKED (e.g. the
        // user typed before the recovery probe fired): trust the evidence.
        stopRecovery();
        setStatusBoth("AVAILABLE", availableNote(out.shell));
      }
      if (out.sessionId) sessionRef.current = out.sessionId;
      if (out.cwd) cwdRef.current = out.cwd;
      if (out.stdoutTail) {
        term.write(`\r\n${out.stdoutTail.replace(/\n/g, "\r\n")}`);
      }
      if (out.stderrTail) {
        term.write(`\r\n\u001b[31m${out.stderrTail.replace(/\n/g, "\r\n")}\u001b[0m`);
      }
      if (out.exitCode !== null && out.exitCode !== 0) {
        term.write(`\r\n\u001b[90mexit ${out.exitCode}\u001b[0m`);
      }
      if (out.truncated) {
        term.write(`\r\n\u001b[33m(output truncated — showing tail)\u001b[0m`);
      }
      busyRef.current = false;
      writePrompt();
    },
    [armRecovery, clearScreen, setStatusBoth, stopRecovery, writePrompt],
  );

  useEffect(() => {
    let disposed = false;
    let cleanup: (() => void) | undefined;

    (async () => {
      try {
        const [{ Terminal }, { FitAddon }] = await Promise.all([
          import("@xterm/xterm"),
          import("@xterm/addon-fit"),
        ]);
        if (disposed || !hostRef.current) return;
        const term = new Terminal({
          fontFamily: "'Cascadia Mono', 'Consolas', monospace",
          fontSize: 12,
          convertEol: true,
          cursorBlink: true,
          theme: {
            background: BRAND_BG,
            foreground: "#d4d4d4",
            cursor: "#00a8cc",
            selectionBackground: "#33474f",
          },
        });
        const fit = new FitAddon();
        term.loadAddon(fit);
        term.open(hostRef.current);
        fit.fit();
        termRef.current = term;
        fitRef.current = fit;

        term.onData((data: string) => {
          if (!readyRef.current) return;
          if (data === "\r") {
            if (busyRef.current) return;
            const line = lineRef.current;
            lineRef.current = "";
            if (line.trim()) {
              historyRef.current.push(line);
              histIdxRef.current = undefined;
              void runCommand(line);
            } else {
              writePrompt();
            }
          } else if (data === "\u007f") {
            // backspace
            if (lineRef.current.length > 0) {
              lineRef.current = lineRef.current.slice(0, -1);
              term.write("\b \b");
            }
          } else if (data >= " ") {
            lineRef.current += data;
            term.write(data);
          }
        });

        // Probe the service so the badge tells the truth before typing.
        const probe = await terminalExec({
          sessionId: sessionRef.current,
          cwd: null,
          shell: shellRef.current,
          command: "echo zylcode_terminal_ready",
        });
        if (disposed) return;
        if (probe.transportError) {
          setStatusBoth("BLOCKED", probe.stderrTail);
          term.write(`\r\n\u001b[31m${probe.stderrTail.replace(/\n/g, "\r\n")}\u001b[0m`);
          armRecovery();
        } else {
          if (probe.sessionId) sessionRef.current = probe.sessionId;
          if (probe.cwd) cwdRef.current = probe.cwd;
          setStatusBoth("AVAILABLE", availableNote(probe.shell));
          term.write(
            `${"\u001b[90m"}ZylCode terminal — commands run for real in your selected shell.${"\u001b[0m"}\r\n` +
              `${"\u001b[90m"}One process per command (command runner, not an interactive PTY); cwd persists per session.${"\u001b[0m"}`,
          );
          writePrompt();
        }
        readyRef.current = true;
        setReady(true);

        const onResize = () => fit.fit();
        window.addEventListener("resize", onResize);
        cleanup = () => {
          window.removeEventListener("resize", onResize);
          term.dispose();
        };
      } catch (err) {
        if (!disposed) {
          setStatusBoth("BLOCKED", `terminal UI failed to load: ${String(err)}`);
        }
      }
    })();

    return () => {
      disposed = true;
      stopRecovery();
      cleanup?.();
    };
  }, [armRecovery, runCommand, setStatusBoth, stopRecovery, writePrompt]);

  const handleReset = useCallback(async () => {
    const ok = await terminalReset();
    const term = termRef.current;
    if (!ok) {
      // Never claim a reset happened when the service never received it.
      term?.writeln("\u001b[31m— reset failed: terminal service unavailable —\u001b[0m");
      setStatusBoth("BLOCKED", "terminal service unavailable — session not reset");
      armRecovery();
      return;
    }
    sessionRef.current = null;
    cwdRef.current = "";
    term?.writeln("");
    term?.writeln("\u001b[90m— session reset —\u001b[0m");
    writePrompt();
  }, [armRecovery, setStatusBoth, writePrompt]);

  return (
    <div className="flex flex-col gap-3 h-full min-h-0">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <StatusBadge status={status} />
          <span className="text-xs text-text-muted font-mono truncate">{statusNote}</span>
        </div>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-1 text-xs text-text-muted font-mono">
            shell
            <select
              aria-label="shell"
              value={shell}
              onChange={(e) => {
                setShell(e.target.value);
                shellRef.current = e.target.value;
              }}
              className="text-xs px-1 py-1 rounded border border-border bg-transparent text-text-primary"
            >
              {SHELL_MODES.map((mode) => (
                <option key={mode} value={mode}>
                  {mode}
                </option>
              ))}
            </select>
          </label>
          <button
            onClick={clearScreen}
            disabled={!ready}
            className="text-xs px-2 py-1 rounded border border-border hover:border-primary/50 text-text-muted hover:text-text-primary disabled:opacity-40"
          >
            Clear
          </button>
          <button
            onClick={handleReset}
            disabled={!ready}
            className="text-xs px-2 py-1 rounded border border-border hover:border-primary/50 text-text-muted hover:text-text-primary disabled:opacity-40"
          >
            Reset session
          </button>
        </div>
      </div>
      <div className="flex-1 min-h-0 rounded-md overflow-hidden border border-border">
        <div ref={hostRef} className="w-full h-full" style={{ background: BRAND_BG }} />
      </div>
    </div>
  );
};

export default TerminalPanel;
