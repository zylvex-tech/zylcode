import { useCallback, useEffect, useRef, useState } from "react";
import { Panel } from "./Panel";
import { StatusBadge, type CapabilityStatus } from "./CapabilityStatus";
import {
  terminalExec,
  terminalReset,
  type TerminalOutput,
} from "../lib/terminal";

const BRAND_BG = "#141414"; // the logo's own dark chip color

/**
 * Real terminal surface: keystrokes go to a persistent backend session
 * (zylcode serve-intel HTTP route, or the Tauri terminal_exec command),
 * output comes back from actually executed commands, and the working
 * directory persists across them. xterm.js is imported lazily so tests
 * never touch browser-only APIs.
 */
export const TerminalPanel: React.FC = () => {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<any | null>(null);
  const fitRef = useRef<any | null>(null);
  const sessionRef = useRef<string | null>(null);
  const cwdRef = useRef<string>("");
  const busyRef = useRef(false);
  const [status, setStatus] = useState<CapabilityStatus>("BLOCKED");
  const [statusNote, setStatusNote] = useState("connecting to terminal service…");
  const [ready, setReady] = useState(false);

  const writePrompt = useCallback(() => {
    const term = termRef.current;
    if (!term) return;
    const dir = cwdRef.current ? cwdRef.current.split(/[\\/]/).pop() || cwdRef.current : "zylcode";
    term.write(`\r\n\u001b[36m${dir}\u001b[0m \u001b[32m$\u001b[0m `);
  }, []);

  const runCommand = useCallback(
    async (line: string) => {
      const term = termRef.current;
      if (!term) return;
      busyRef.current = true;
      const out: TerminalOutput = await terminalExec({
        sessionId: sessionRef.current,
        cwd: cwdRef.current || null,
        command: line,
      });
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
    [writePrompt],
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

        term.onData((data) => {
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
          sessionId: null,
          cwd: null,
          command: "echo zylcode_terminal_ready",
        });
        if (disposed) return;
        if (probe.stderrTail.startsWith("terminal ")) {
          setStatus("BLOCKED");
          setStatusNote(probe.stderrTail);
          term.write(`\r\n\u001b[31m${probe.stderrTail}\u001b[0m`);
        } else {
          sessionRef.current = probe.sessionId;
          cwdRef.current = probe.cwd;
          setStatus("AVAILABLE");
          setStatusNote("real shell · cwd persists per session");
          term.write("\u001b[90mZylCode terminal — commands run for real.\u001b[0m");
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
          setStatus("BLOCKED");
          setStatusNote(`terminal UI failed to load: ${String(err)}`);
        }
      }
    })();

    return () => {
      disposed = true;
      cleanup?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Refs used inside the xterm data handler without re-binding it.
  const readyRef = useRef(false);
  const lineRef = useRef("");
  const historyRef = useRef<string[]>([]);
  const histIdxRef = useRef<number | undefined>(undefined);

  const handleClear = useCallback(() => {
    termRef.current?.clear();
    writePrompt();
  }, [writePrompt]);

  const handleReset = useCallback(async () => {
    await terminalReset();
    sessionRef.current = null;
    cwdRef.current = "";
    termRef.current?.writeln("");
    termRef.current?.writeln("\u001b[90m— session reset —\u001b[0m");
    writePrompt();
  }, [writePrompt]);

  return (
    <div className="flex flex-col gap-3 h-full min-h-0">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <StatusBadge status={status} />
          <span className="text-xs text-text-muted font-mono truncate">{statusNote}</span>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleClear}
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
