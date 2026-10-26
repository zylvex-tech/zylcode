//! Terminal sessions with persistent working directories.
//!
//! # Why this exists
//!
//! The Run surface honestly reported TERMINAL as LIMITED because there was
//! no execution path an operator could type into. This module provides the
//! real thing: a session-aware command executor that remembers the working
//! directory between commands, captures stdout/stderr with a hard output
//! cap, and kills runaway commands at the timeout — the same fail-closed
//! discipline as the rest of the codebase.
//!
//! # Design notes
//!
//! This is a **command runner, not a PTY**: every command is a separate
//! short-lived process (no interactive shell session, no line editing at the
//! process level — the UI owns the prompt). Each exec runs through the
//! selected shell:
//!
//! | mode | program |
//! |---|---|
//! | `auto` | Windows: `powershell.exe` when present, else `cmd.exe`; elsewhere `sh` |
//! | `powershell` | `powershell.exe -NoProfile -NonInteractive -EncodedCommand` |
//! | `pwsh` | `pwsh.exe -NoProfile -NonInteractive -EncodedCommand` |
//! | `cmd` | `cmd.exe /V:ON /C` |
//! | `sh` | `sh -c` |
//!
//! The mode is chosen per request (`TerminalRequest.shell`), else from
//! `ZYLCODE_TERMINAL_SHELL`, else `auto`. An unknown mode or a missing shell
//! executable is an error the caller can read — never a silent fall back to
//! a different shell.
//!
//! Because a child process cannot mutate the parent's working directory,
//! cwd tracking works like lightweight web terminals do: the wrapper appends
//! an echo of the resulting directory (`__ZYLCODE_CWD__<path>`), which is
//! parsed out of the output and stored on the session. That marker is an
//! implementation detail of the transport — the UI never shows it.
//!
//! Commands run with the process environment plus `ZYLCODE_TERMINAL=1`; a
//! suite can assert from inside that it is talking to the terminal service,
//! mirroring the sandbox's ready-marker pattern.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Marker line the wrapper appends to learn the resulting cwd. Parsed and
/// stripped before the output reaches the UI.
const CWD_MARKER: &str = "__ZYLCODE_CWD__";

/// Marker line the PowerShell wrapper appends to report the command's exit
/// status *without* calling `exit` — see [`powershell_script`] for why.
/// Also stripped before the output reaches the UI.
const RC_MARKER: &str = "__ZYLCODE_RC__";

/// Hard cap per stream so a runaway command cannot exhaust memory. Both
/// streams keep their TAIL — the end of the output is where errors live.
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// Default per-command budget.
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// A persistent shell session: an id and the directory the next command
/// runs in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSession {
    pub id: String,
    pub cwd: Option<PathBuf>,
}

/// Request to run one command in a session.
#[derive(Debug, Clone, Deserialize)]
pub struct TerminalRequest {
    /// Empty/None creates (and returns) a fresh session id.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Working directory override; defaults to the session's stored cwd,
    /// then the workspace root for new sessions.
    #[serde(default)]
    pub cwd: Option<String>,
    /// Shell mode for this command: `auto`, `powershell`, `pwsh`, `cmd` or
    /// `sh`. `None`/absent falls back to `ZYLCODE_TERMINAL_SHELL`, then the
    /// platform default — the shell never changes silently underneath a
    /// request that did name one.
    #[serde(default)]
    pub shell: Option<String>,
    pub command: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// The result of one executed command.
#[derive(Debug, Clone, Serialize)]
pub struct TerminalOutput {
    pub session_id: String,
    pub cwd: String,
    /// The shell this command actually ran in (`powershell`, `pwsh`, `cmd`,
    /// `sh`) so the UI can say what really executed instead of implying an
    /// interactive shell the backend does not provide.
    pub shell: String,
    pub exit_code: Option<i32>,
    /// `Some` when the command was killed at the timeout.
    pub timed_out: bool,
    pub stdout_tail: String,
    pub stderr_tail: String,
    /// True when output was capped at [`MAX_OUTPUT_BYTES`] per stream.
    pub truncated: bool,
    pub duration_ms: u128,
}

/// Parse the wrapper's trailing markers, returning the visible output
/// (marker lines removed), the resulting directory, and — when the shell
/// reported one — the command's exit code.
fn split_cwd_marker(raw: &str) -> (String, Option<String>, Option<i32>) {
    let mut visible = String::with_capacity(raw.len());
    let mut cwd = None;
    let mut rc = None;
    for line in raw.lines() {
        if let Some(idx) = line.find(CWD_MARKER) {
            let candidate = line[idx + CWD_MARKER.len()..].trim();
            if !candidate.is_empty() {
                cwd = Some(candidate.to_string());
            }
        } else if let Some(idx) = line.find(RC_MARKER) {
            // A malformed/empty RC line means "no code reported" — fall back
            // to the process status rather than inventing a zero.
            rc = line[idx + RC_MARKER.len()..].trim().parse::<i32>().ok();
        } else {
            visible.push_str(line);
            visible.push('\n');
        }
    }
    (visible, cwd, rc)
}

/// Decode PowerShell's CLIXML error-stream serialization into plain text.
///
/// When `powershell.exe`'s stderr is redirected, it wraps error records in a
/// CLIXML document prefixed by a `#< CLIXML` marker line — the readable
/// message is in there, but a terminal shows it as XML noise. This extracts
/// the `<S S="Error">` payloads and undoes PowerShell's `_xNNNN_` escapes
/// (single left-to-right pass, so `_x005F_x000A_` — a literal `_x000A_` —
/// is not double-decoded).
///
/// Returns `None` unless the input is unambiguously CLIXML, so anything else
/// is passed through untouched — a decoder that guesses would hide real
/// output.
fn decode_powershell_clixml(stderr: &str) -> Option<String> {
    let trimmed = stderr.trim_start_matches('\u{feff}').trim_start();
    let rest = trimmed.strip_prefix("#< CLIXML")?.trim_start();
    if !rest.starts_with("<Objs") {
        return None;
    }
    let mut out = String::new();
    let mut found = false;
    let mut search = rest;
    // Only the error records: progress/verbose payloads are not part of the
    // command's own diagnostics.
    while let Some(start) = search.find("<S S=\"Error\">") {
        let after = &search[start + "<S S=\"Error\">".len()..];
        let Some(end) = after.find("</S>") else { break };
        found = true;
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&unescape_ps_xml(&unescape_xml_entities(&after[..end])));
        search = &after[end + "</S>".len()..];
    }
    // An empty-but-valid CLIXML means "no error records" — report nothing
    // rather than the XML shell.
    found.then_some(out)
}

/// Undo PowerShell's `_xNNNN_` character escapes in one left-to-right pass.
///
/// `_x005F_` is how PowerShell escapes a literal underscore that would
/// otherwise begin an escape sequence: decoding it to `_` and *not*
/// re-scanning the following text is what keeps `lit_x005F_x000A_end`
/// decoding to the literal `lit_x000A_end` instead of a newline.
fn unescape_ps_xml(s: &str) -> String {
    const ESCAPED_UNDERSCORE: &str = "_x005F_";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(idx) = rest.find("_x") {
        out.push_str(&rest[..idx]);
        let tail = &rest[idx..];
        if let Some(after) = tail.strip_prefix(ESCAPED_UNDERSCORE) {
            out.push('_');
            rest = after;
            continue;
        }
        // `_xNNNN_`: two-char prefix, four hex digits, closed by an
        // underscore. Only CR and LF are produced here — anything else is
        // left verbatim rather than turned into a surprise control char.
        let code = tail
            .get(2..6)
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            .and_then(char::from_u32);
        if matches!(code, Some('\r') | Some('\n')) && tail.get(6..7) == Some("_") {
            out.push(code.unwrap());
            rest = &tail[7..];
            continue;
        }
        // Not a recognizable escape: keep it verbatim.
        out.push_str("_x");
        rest = &tail[2..];
    }
    out.push_str(rest);
    out
}

/// Decode the five built-in XML entities plus numeric references.
fn unescape_xml_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(idx) = rest.find('&') {
        out.push_str(&rest[..idx]);
        let tail = &rest[idx..];
        let Some(semi) = tail.find(';') else {
            out.push_str(tail);
            return out;
        };
        let entity = &tail[1..semi];
        let decoded = match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &tail[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn tail(s: &str) -> (String, bool) {
    let bytes = s.as_bytes();
    if bytes.len() <= MAX_OUTPUT_BYTES {
        return (s.to_string(), false);
    }
    // Cut at a char boundary at or after the cap so multi-byte UTF-8 and
    // CRLF pairs survive.
    let mut cut = bytes.len() - MAX_OUTPUT_BYTES;
    while cut < bytes.len() && (bytes[cut] & 0b1100_0000) == 0b1000_0000 {
        cut += 1;
    }
    (s[cut..].to_string(), true)
}

/// Which platform shell the wrapper script is built for.
///
/// The wrapper exists so one shell invocation can run the user's command
/// *and* report the directory it ended in. That report is only useful if the
/// command's own effects — above all `cd` — are still in force when the
/// marker is printed, so how the command is grouped matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // `PowerShell` is the tool's real name; renaming it to dodge the lint would only obscure which shell this is.
enum Shell {
    /// Windows `cmd.exe /V:ON /C`.
    Cmd,
    /// Windows PowerShell 5.x, `powershell.exe -EncodedCommand`.
    PowerShell,
    /// PowerShell 7+, `pwsh.exe -EncodedCommand`.
    Pwsh,
    /// POSIX `sh -c`.
    Posix,
}

/// The shell modes the application exposes. `auto` picks the platform
/// default; the others name a concrete program, so a request that says
/// `powershell` is never quietly served by `cmd.exe` (and vice versa).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellMode {
    /// Platform default: [`Shell::auto`] on this machine.
    #[default]
    Auto,
    /// Windows `cmd.exe`.
    Cmd,
    /// Windows PowerShell 5.x (`powershell.exe`).
    PowerShell,
    /// PowerShell 7+ (`pwsh.exe`).
    Pwsh,
    /// POSIX `sh`.
    Sh,
}

impl ShellMode {
    /// Parse a caller-supplied mode name. Unknown names are rejected with a
    /// message naming the accepted ones — silently substituting a different
    /// shell is exactly the defect this selection exists to prevent.
    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" | "" => Ok(Self::Auto),
            "cmd" | "cmd.exe" => Ok(Self::Cmd),
            "powershell" | "powershell.exe" => Ok(Self::PowerShell),
            "pwsh" | "pwsh.exe" => Ok(Self::Pwsh),
            "sh" | "sh.exe" => Ok(Self::Sh),
            other => anyhow::bail!(
                "unsupported shell mode {other:?} — expected one of: auto, cmd, powershell, pwsh, sh"
            ),
        }
    }

    /// The concrete shell this mode selects on this machine.
    fn resolve(self, auto: Shell) -> Shell {
        match self {
            Self::Auto => auto,
            Self::Cmd => Shell::Cmd,
            Self::PowerShell => Shell::PowerShell,
            Self::Pwsh => Shell::Pwsh,
            Self::Sh => Shell::Posix,
        }
    }

    /// `ZYLCODE_TERMINAL_SHELL` as a mode, or `auto` when unset.
    ///
    /// A set-but-invalid value is returned as an error to surface on every
    /// exec rather than being swallowed into a default the operator never
    /// asked for.
    fn from_env() -> Result<Self> {
        match std::env::var("ZYLCODE_TERMINAL_SHELL") {
            Ok(raw) => Self::parse(&raw).map_err(|e| e.context("invalid ZYLCODE_TERMINAL_SHELL")),
            Err(std::env::VarError::NotPresent) => Ok(Self::Auto),
            Err(e) => Err(anyhow::anyhow!("cannot read ZYLCODE_TERMINAL_SHELL: {e}")),
        }
    }
}

/// True when `name` resolves through `PATH`.
fn on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

impl Shell {
    /// The platform default shell. Windows prefers `powershell.exe` — it is
    /// present on every stock install and accepts the PowerShell dialect the
    /// desktop's users reach for (`ls`, `clear`, `Get-Location`); `cmd.exe`
    /// is the fallback only when PowerShell is genuinely absent.
    fn auto() -> Shell {
        if cfg!(target_family = "windows") {
            // PATH first, then the well-known install location — a stock
            // Windows keeps it there even when PATH has been edited.
            let on_disk = std::env::var_os("SystemRoot")
                .map(|root| {
                    Path::new(&root)
                        .join("System32")
                        .join("WindowsPowerShell")
                        .join("v1.0")
                        .join("powershell.exe")
                        .is_file()
                })
                .unwrap_or(false);
            if on_path("powershell.exe") || on_disk {
                Shell::PowerShell
            } else {
                Shell::Cmd
            }
        } else {
            Shell::Posix
        }
    }

    /// The program this shell starts.
    fn program(self) -> &'static str {
        match self {
            Shell::Cmd => "cmd.exe",
            Shell::PowerShell => "powershell.exe",
            Shell::Pwsh => "pwsh.exe",
            Shell::Posix => "sh",
        }
    }

    /// Short name reported back to the UI in [`TerminalOutput::shell`].
    fn label(self) -> &'static str {
        match self {
            Shell::Cmd => "cmd",
            Shell::PowerShell => "powershell",
            Shell::Pwsh => "pwsh",
            Shell::Posix => "sh",
        }
    }

    /// Build `(program, leading args, trailing arg)` for one wrapped command.
    ///
    /// The script runs the command, prints `<CWD_MARKER><dir>`, and exits with
    /// the command's own status — a trailing `echo` alone would reset it to 0
    /// and mask real failures.
    fn wrap(self, command: &str) -> (String, Vec<String>, String) {
        match self {
            // cmd needs /V:ON because %CD%/%ERRORLEVEL% would otherwise
            // expand at PARSE time (before `cd` runs, and before the
            // command's exit code exists); !CD!/!ERRORLEVEL! expand at
            // execution. cmd.exe has no subshell, so `( ... )` groups the
            // whole command for the appended `&` chain *without* forking —
            // `cd` still applies to the trailing `& echo !CD!`, and the
            // group keeps those markers out of a `for ... do` body (where
            // cmd's `&` would bind to the loop body and terminate it on the
            // first iteration).
            Shell::Cmd => (
                "cmd.exe".to_string(),
                vec!["/V:ON".to_string(), "/C".to_string()],
                format!("({command}) & echo {CWD_MARKER}!CD! & exit /b !ERRORLEVEL!"),
            ),
            // A POSIX `( ... )` IS a subshell: the `cd` inside dies with it,
            // `$PWD` still reports the session directory, and the session
            // would never advance — `cwd_persists_across_commands_in_a_session`
            // failed on the Linux and macOS runners for exactly this reason.
            // `{ ... }` groups in the *current* shell, so `cd` survives to the
            // marker echo while still closing the group as a whole (a `for`
            // loop therefore cannot swallow the markers into its body).
            //
            // The closing brace sits on its own line so a trailing `#` comment
            // in the user's command cannot eat it: `(echo hi # note)` is a
            // syntax error that fails the whole command, whereas
            // `{ echo hi # note\n}` still closes.
            Shell::Posix => (
                "sh".to_string(),
                vec!["-c".to_string()],
                format!("{{ {command}\n}}; __zyl_rc=$?; echo {CWD_MARKER}$PWD; exit $__zyl_rc"),
            ),
            // PowerShell: `-EncodedCommand` takes the script as base64
            // UTF-16LE, so the user's own quoting, `$` and quotes never pass
            // through a command-line re-parse — the documented strategy for
            // handing arbitrary content to powershell.exe. `-NoProfile`
            // keeps machine state out of the result, `-NonInteractive`
            // makes a prompt fail instead of hanging the session.
            Shell::PowerShell | Shell::Pwsh => (
                self.program().to_string(),
                vec![
                    "-NoProfile".to_string(),
                    "-NonInteractive".to_string(),
                    "-EncodedCommand".to_string(),
                ],
                encode_utf16le_base64(&powershell_script(command)),
            ),
        }
    }
}

/// The PowerShell wrapper script: run the command in the current scope (so a
/// `cd` is still in force), then report `$PWD` and the command's own status
/// as trailing marker lines.
///
/// - `$global:LASTEXITCODE` is pre-cleared so a pure cmdlet command (which
///   never sets it) reports 0, while a failing cmdlet — `$?` false with no
///   native exit code — reports 1 instead of a stale code;
/// - `$ProgressPreference = 'SilentlyContinue'` is set **first**, before any
///   other cmdlet: the module load that `New-Object` triggers emits a
///   "Preparing modules for first use" progress record, which a redirected
///   stderr serializes as CLIXML noise. Setting the preference afterwards
///   does not help — the record has already been written;
/// - **no `exit` statement**: PowerShell formats pipeline output lazily, at
///   the end of the pipeline, and an explicit `exit` terminates the process
///   before that flush — silently discarding formatted output such as
///   `Get-Location`'s table. Emitting the status as a marker line and
///   letting the script end naturally keeps every byte the command produced;
/// - every tail statement sits on its own line, so a trailing `#` comment in
///   the user's command cannot swallow a marker;
/// - the console encoding is forced to UTF-8 when possible so non-ASCII
///   output survives the pipe (guarded: a process without a console must not
///   abort the run).
fn powershell_script(command: &str) -> String {
    format!(
        "$ProgressPreference = 'SilentlyContinue'\n\
         try {{ [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false) }} catch {{}}\n\
         $global:LASTEXITCODE = 0\n\
         {command}\n\
         $zyl_ok = $?\n\
         $zyl_rc = $global:LASTEXITCODE\n\
         if (-not $zyl_ok) {{ if ($zyl_rc -eq 0) {{ $zyl_rc = 1 }} }}\n\
         Write-Output \"{CWD_MARKER}$PWD\"\n\
         Write-Output \"{RC_MARKER}$zyl_rc\"\n"
    )
}

/// Remove the wrapper's own source text from a PowerShell error message.
///
/// PowerShell renders a failure as `<CommandLine> : <Message>`. Under
/// `-EncodedCommand` the "command line" is the **entire decoded script**, so
/// a failing command's stderr opens with our wrapper verbatim — every marker
/// and internal variable included. That text is transport, not output.
///
/// This drops that leading echo and nothing else: the genuine message and
/// PowerShell's own `+ CategoryInfo` diagnostics survive untouched. It is
/// deliberately conservative — unless the stderr begins with the wrapper's
/// own first line and matches every wrapper line in order, the input is
/// returned unchanged, so ordinary output is never mangled.
fn strip_wrapper_echo(visible: &str, command: &str) -> String {
    let script = powershell_script(command);
    let expected: Vec<&str> = script
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if expected.is_empty() {
        return visible.to_string();
    }

    let lines: Vec<&str> = visible.lines().collect();
    let mut idx = 0;
    let mut matched = 0;
    while idx < lines.len() && matched < expected.len() {
        let line = lines[idx].trim();
        if line.is_empty() {
            idx += 1;
            continue;
        }
        if line == expected[matched] {
            matched += 1;
            idx += 1;
        } else {
            break;
        }
    }
    if matched != expected.len() {
        return visible.to_string();
    }

    // Skip the blank line(s) PowerShell inserts between the echoed script and
    // the message, then drop the bare `:` that followed the command line.
    let mut rest = &lines[idx..];
    while rest.first().is_some_and(|line| line.trim().is_empty()) {
        rest = &rest[1..];
    }
    let mut out = String::new();
    for (n, line) in rest.iter().enumerate() {
        let line = if n == 0 {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix(':')
                .map(str::trim_start)
                .unwrap_or(trimmed)
        } else {
            line
        };
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
    }
    out
}

/// Base64 (standard alphabet, padded) over the script's UTF-16LE bytes —
/// exactly what `-EncodedCommand` expects. Kept dependency-free on purpose:
/// the encoder is small, and the decoder lives in the tests that pin it.
fn encode_utf16le_base64(script: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Strip Windows' verbatim prefix (`\\?\C:\...`, `\\?\UNC\...`) so paths are
/// usable and displayable in a shell and in the UI. `canonicalize` on
/// Windows returns these; PowerShell's `$PWD` reports the plain form, so an
/// unstripped root would make the session cwd disagree with the shell's own.
fn strip_verbatim(path: PathBuf) -> PathBuf {
    if !cfg!(target_family = "windows") {
        return path;
    }
    let Some(s) = path.to_str() else { return path };
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path
    }
}

/// Session store shared by the HTTP service and the Tauri command.
pub struct TerminalHub {
    root: PathBuf,
    /// Shell selected by `ZYLCODE_TERMINAL_SHELL` at construction (`auto`
    /// when unset, an error when set to something unknown).
    env_mode: Result<ShellMode, String>,
    /// Resolved once: what `auto` means on this machine.
    auto: Shell,
    sessions: Mutex<HashMap<String, TerminalSession>>,
}

impl TerminalHub {
    pub fn new(root: PathBuf) -> Self {
        Self::with_mode(root, ShellMode::from_env().map_err(|e| format!("{e:#}")))
    }

    /// Construct with an explicit default mode (used by callers that carry
    /// their own configuration, and by tests that must not mutate the
    /// process environment).
    pub fn with_mode(root: PathBuf, mode: Result<ShellMode, String>) -> Self {
        Self {
            root: strip_verbatim(root),
            env_mode: mode,
            auto: Shell::auto(),
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// The default mode for requests that do not name one.
    fn default_mode(&self) -> Result<ShellMode> {
        match &self.env_mode {
            Ok(mode) => Ok(*mode),
            Err(e) => anyhow::bail!("{e}"),
        }
    }

    /// The shell for one request: request field, then the hub default, then
    /// the platform `auto` resolution.
    fn shell_for(&self, req: &TerminalRequest) -> Result<Shell> {
        let mode = match req.shell.as_deref() {
            Some(raw) => ShellMode::parse(raw)?,
            None => self.default_mode()?,
        };
        Ok(mode.resolve(self.auto))
    }

    /// Resolve (or create) the session and its starting directory.
    fn resolve_session(&self, req: &TerminalRequest) -> (TerminalSession, PathBuf) {
        let mut sessions = self.sessions.lock().unwrap();
        let id = req
            .session_id
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let session = sessions
            .entry(id.clone())
            .or_insert_with(|| TerminalSession {
                id: id.clone(),
                cwd: None,
            });
        let base = req
            .cwd
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| session.cwd.clone())
            .unwrap_or_else(|| self.root.clone());
        (session.clone(), base)
    }

    fn store_cwd(&self, session_id: &str, cwd: PathBuf) {
        if let Some(s) = self.sessions.lock().unwrap().get_mut(session_id) {
            s.cwd = Some(cwd);
        }
    }

    /// Forget all sessions (cwd history included). Called by the reset
    /// route; never fabricates state afterwards.
    pub fn reset(&self) {
        self.sessions.lock().unwrap().clear();
    }

    pub fn session_ids(&self) -> Vec<String> {
        self.sessions.lock().unwrap().keys().cloned().collect()
    }

    /// Execute one command. The child inherits the environment plus the
    /// `ZYLCODE_TERMINAL=1` marker; on timeout it is killed and the outcome
    /// says so rather than returning invented output.
    pub async fn exec(&self, req: &TerminalRequest) -> Result<TerminalOutput> {
        let (session, base) = self.resolve_session(req);
        if req.command.trim().is_empty() {
            anyhow::bail!("terminal command must not be empty");
        }
        if req.command.contains('\0') {
            anyhow::bail!("terminal command must not contain NUL bytes");
        }
        let shell = self.shell_for(req)?;
        if !base.is_dir() {
            anyhow::bail!(
                "working directory '{}' does not exist — pick an existing workspace \
                 (--workspace <path> or ZYLCODE_WORKSPACE) or `cd` somewhere that does",
                base.display()
            );
        }
        let timeout = Duration::from_secs(req.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));

        let (program, pre, wrapped) = shell.wrap(&req.command);

        let started = std::time::Instant::now();
        let spawn = tokio::process::Command::new(&program)
            .args(&pre)
            .arg(wrapped)
            .current_dir(&base)
            .env("ZYLCODE_TERMINAL", "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();
        // The directory was checked above, so a NotFound here means the shell
        // program itself is missing — say which mode asked for it instead of
        // dropping a bare OS error on the caller.
        let child = match spawn {
            Ok(child) => child,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => anyhow::bail!(
                "shell executable '{}' not found (shell mode '{}') — install it or choose \
                 another mode: auto, cmd, powershell, pwsh, sh",
                program,
                shell.label()
            ),
            Err(e) => {
                return Err(
                    anyhow::Error::new(e).context(format!("failed to start shell '{program}'"))
                );
            }
        };

        let run = tokio::time::timeout(timeout, child.wait_with_output()).await;
        let duration_ms = started.elapsed().as_millis();
        let output = match run {
            Ok(inner) => inner.context("terminal command failed to run")?,
            Err(_) => {
                // wait_with_output consumed the child; best-effort kill of
                // the process group is out of scope — report honestly.
                return Ok(TerminalOutput {
                    session_id: session.id,
                    cwd: base.to_string_lossy().to_string(),
                    shell: shell.label().to_string(),
                    exit_code: None,
                    timed_out: true,
                    stdout_tail: String::new(),
                    stderr_tail: format!(
                        "command timed out after {}s and was terminated",
                        timeout.as_secs()
                    ),
                    truncated: false,
                    duration_ms,
                });
            }
        };

        let stdout_raw = String::from_utf8_lossy(&output.stdout);
        let (stdout_visible, new_cwd, reported_rc) = split_cwd_marker(&stdout_raw);
        let (stdout_tail, out_trunc) = tail(&stdout_visible);
        let stderr_raw = String::from_utf8_lossy(&output.stderr);
        // PowerShell wraps error records in CLIXML on a redirected stderr;
        // decode so the message the user sees is the message PowerShell wrote.
        let stderr_decoded =
            decode_powershell_clixml(&stderr_raw).unwrap_or_else(|| stderr_raw.into_owned());
        // A failing command makes PowerShell echo the whole `-EncodedCommand`
        // script as the error's command line. Strip that transport text so
        // neither the wrapper nor the cwd/rc markers reach the transcript.
        let stderr_visible = match shell {
            Shell::PowerShell | Shell::Pwsh => strip_wrapper_echo(&stderr_decoded, &req.command),
            _ => stderr_decoded,
        };
        let (stderr_tail, err_trunc) = tail(&stderr_visible);

        // Persist the resulting cwd for the next command in this session.
        let final_cwd = strip_verbatim(new_cwd.map(PathBuf::from).unwrap_or_else(|| base.clone()));
        if final_cwd.is_dir() {
            self.store_cwd(&session.id, final_cwd.clone());
        }

        // cmd.exe and sh exit with the command's own status, so the process
        // code is authoritative there. PowerShell cannot use `exit` (it
        // discards lazily-formatted output), so it reports the status as a
        // marker instead — prefer that, and fall back to the process status
        // when the wrapper died before it could report one.
        let exit_code = reported_rc.or_else(|| output.status.code());

        Ok(TerminalOutput {
            session_id: session.id,
            cwd: final_cwd.to_string_lossy().to_string(),
            shell: shell.label().to_string(),
            exit_code,
            timed_out: false,
            stdout_tail,
            stderr_tail,
            truncated: out_trunc || err_trunc,
            duration_ms,
        })
    }
}

/// Convenience for call sites that only need one shot against a root.
pub async fn exec_in_root(root: &Path, req: &TerminalRequest) -> Result<TerminalOutput> {
    TerminalHub::new(root.to_path_buf()).exec(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::create_dir_all(root.join("sub")).unwrap();
        (dir, root)
    }

    /// Compare a reported cwd with the directory it is supposed to name.
    ///
    /// macOS hands `tempfile` `/var/folders/...` while the shell that ran the
    /// command reports the physical `/private/var/folders/...`; both name one
    /// directory, so resolve symlinks before comparing. If either side will
    /// not resolve, fall back to the exact comparison this assertion used
    /// before — never to a looser one.
    fn same_path(reported: &str, expected: &Path) -> bool {
        let reported = PathBuf::from(reported);
        match (reported.canonicalize(), expected.canonicalize()) {
            (Ok(a), Ok(b)) => a == b,
            _ => reported == expected.to_path_buf(),
        }
    }

    #[tokio::test]
    async fn echo_runs_and_reports_exit_zero() {
        let (_dir, root) = temp_repo();
        let hub = TerminalHub::new(root.clone());
        let out = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: None,
                command: "echo hello_zylcode".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert_eq!(out.exit_code, Some(0));
        assert!(out.stdout_tail.contains("hello_zylcode"), "{out:?}");
        assert!(
            same_path(&out.cwd, &root),
            "expected cwd {}, got {}",
            root.display(),
            out.cwd
        );
        assert!(!out.stdout_tail.contains(CWD_MARKER));
        assert_eq!(out.session_ids_check(), ());
    }

    impl TerminalOutput {
        fn session_ids_check(&self) {}
    }

    #[tokio::test]
    async fn cwd_persists_across_commands_in_a_session() {
        let (_dir, root) = temp_repo();
        let hub = TerminalHub::new(root.clone());
        let first = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: None,
                command: "cd sub".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert!(
            same_path(&first.cwd, &root.join("sub")),
            "expected cwd inside sub, got {}",
            first.cwd
        );
        let second = hub
            .exec(&TerminalRequest {
                session_id: Some(first.session_id.clone()),
                cwd: None,
                shell: None,
                command: "echo again".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert!(
            same_path(&second.cwd, &root.join("sub")),
            "the second command must run in the directory the first left it in, got {}",
            second.cwd
        );
    }

    // -- wrapper shape ---------------------------------------------------
    //
    // These are the invariants that keep the two runtime tests above honest.
    // They are pure string assertions so they run on every platform; the
    // behavioural half of the proof is `cwd_persists_across_commands_in_a_session`
    // executing for real through `sh` on the Linux and macOS CI runners.

    #[test]
    fn posix_wrapper_groups_without_forking_so_cd_survives() {
        let (program, args, script) = Shell::Posix.wrap("cd sub");
        assert_eq!(program, "sh");
        assert_eq!(args, vec!["-c".to_string()]);
        assert!(
            script.starts_with("{ cd sub\n}"),
            "the command must be grouped by a current-shell {{ }} group, not a \
             subshell: a `( ... )` group dies before the marker echo and the \
             session never advances. got {script:?}"
        );
        assert!(
            !script.starts_with('('),
            "a parenthesised group forks on POSIX: got {script:?}"
        );
        assert!(
            script.contains(&format!("echo {CWD_MARKER}$PWD")),
            "missing cwd marker: {script:?}"
        );
        assert!(
            script.ends_with("exit $__zyl_rc"),
            "the wrapper must exit with the command's own status: {script:?}"
        );
    }

    #[test]
    fn posix_wrapper_closing_brace_survives_a_trailing_comment() {
        // `{ echo hi # note\n}` closes on the next line; `(echo hi # note)`
        // puts the `)` inside the comment and fails the whole command.
        let (_, _, script) = Shell::Posix.wrap("echo hi # note");
        assert!(
            script.contains("\n}; "),
            "the group must close on its own line so a trailing comment cannot \
             swallow it: {script:?}"
        );
        assert!(
            !script.contains("(echo hi # note)"),
            "the subshell form fails this command outright: {script:?}"
        );
    }

    #[test]
    fn cmd_wrapper_keeps_its_group_and_delayed_expansion() {
        let (program, args, script) = Shell::Cmd.wrap("cd sub");
        assert_eq!(program, "cmd.exe");
        assert_eq!(args, vec!["/V:ON".to_string(), "/C".to_string()]);
        assert_eq!(
            script,
            format!("(cd sub) & echo {CWD_MARKER}!CD! & exit /b !ERRORLEVEL!")
        );
    }

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn timeout_kills_the_command_and_says_so() {
        let (_dir, root) = temp_repo();
        let hub = TerminalHub::new(root);
        let out = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: None,
                command: "ping -n 10 127.0.0.1".into(),
                timeout_secs: Some(1),
            })
            .await
            .unwrap();
        assert!(out.timed_out, "{out:?}");
        assert!(out.exit_code.is_none());
    }

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn output_is_capped_with_a_tail() {
        let (_dir, root) = temp_repo();
        let hub = TerminalHub::new(root);
        let out = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                // `for /l %i in (...) do` is cmd dialect — run it *as* cmd
                // instead of hoping the default shell understands it.
                shell: Some("cmd".into()),
                command: "for /l %i in (1,1,20000) do @echo line%i".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert_eq!(out.shell, "cmd", "{out:?}");
        assert!(out.truncated, "expected truncation, got: {out:?}");
        assert!(out.stdout_tail.len() <= MAX_OUTPUT_BYTES + 4);
        assert!(
            out.stdout_tail.trim_end().ends_with("line20000"),
            "tail keeps the end: ...{}",
            &out.stdout_tail[out.stdout_tail.len().saturating_sub(120)..]
        );
        assert!(!out.stdout_tail.contains("line1 \n"), "head was dropped");
    }

    #[tokio::test]
    async fn missing_command_reports_nonzero_exit_with_stderr() {
        let (_dir, root) = temp_repo();
        let hub = TerminalHub::new(root);
        let out = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: None,
                command: "definitely_not_a_real_command_xyz".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert_ne!(out.exit_code, Some(0));
        assert!(!out.stderr_tail.is_empty(), "{out:?}");
    }

    #[tokio::test]
    async fn empty_command_is_rejected() {
        let (_dir, root) = temp_repo();
        let hub = TerminalHub::new(root);
        assert!(hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: None,
                command: "   ".into(),
                timeout_secs: None,
            })
            .await
            .is_err());
    }

    // -- shell selection -------------------------------------------------
    //
    // The defect these exist for: `ls`/`Write-Output` used to be handed to
    // `cmd.exe`, which answers every PowerShell command with 9009. Selection
    // must be explicit, honoured exactly, and loud when it cannot be.

    /// Run one command through a fresh hub with an explicit mode.
    async fn exec_with(root: &Path, shell: Option<&str>, command: &str) -> Result<TerminalOutput> {
        TerminalHub::new(root.to_path_buf())
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: shell.map(str::to_string),
                command: command.into(),
                timeout_secs: None,
            })
            .await
    }

    /// False on a machine without Windows PowerShell (then the PowerShell
    /// behavioural tests skip *and say so* rather than pretending).
    ///
    /// Every caller is a `#[cfg(target_family = "windows")]` test, so on
    /// POSIX targets this helper has no users and would be dead code.
    #[cfg(target_family = "windows")]
    fn powershell_available() -> bool {
        Shell::auto() == Shell::PowerShell
    }

    #[test]
    fn shell_mode_names_parse_and_unknown_names_are_rejected() {
        for (raw, want) in [
            ("auto", ShellMode::Auto),
            ("", ShellMode::Auto),
            ("cmd", ShellMode::Cmd),
            ("powershell", ShellMode::PowerShell),
            ("pwsh", ShellMode::Pwsh),
            ("sh", ShellMode::Sh),
            ("PowerShell", ShellMode::PowerShell),
            ("  CMD  ", ShellMode::Cmd),
        ] {
            assert_eq!(ShellMode::parse(raw).unwrap(), want, "parsing {raw:?}");
        }
        let err = ShellMode::parse("fish").unwrap_err();
        assert!(
            err.to_string().contains("unsupported shell mode"),
            "unknown modes must name the problem: {err}"
        );
        assert!(err.to_string().contains("powershell"), "{err}");
    }

    #[tokio::test]
    async fn requested_shell_mode_is_the_one_that_runs() {
        let (_dir, root) = temp_repo();
        // `cmd` is asked for, so `cmd` must be the answer — and on POSIX
        // runners (no cmd.exe at all) the request must fail loudly instead
        // of being served by `sh`.
        let out = exec_with(&root, Some("cmd"), "echo cmd-mode").await;
        if cfg!(target_family = "windows") {
            let out = out.unwrap();
            assert_eq!(out.shell, "cmd", "the request said cmd: {out:?}");
            assert_eq!(out.exit_code, Some(0), "{out:?}");
            assert!(out.stdout_tail.contains("cmd-mode"), "{out:?}");
        } else {
            let err = format!("{:#}", out.unwrap_err());
            assert!(
                err.contains("cmd.exe") && err.contains("not found"),
                "expected a missing-shell error naming cmd.exe, got: {err}"
            );
        }
    }

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn windows_auto_shell_accepts_the_powershell_dialect() {
        let (_dir, root) = temp_repo();
        if !powershell_available() {
            eprintln!("SKIP: powershell.exe is not installed on this machine");
            return;
        }
        let hub = TerminalHub::new(root);
        for command in [
            r#"Write-Output "ps-output-ok""#,
            "Get-Location",
            "$PSVersionTable.PSVersion.Major",
            "ls",
        ] {
            let out = hub
                .exec(&TerminalRequest {
                    session_id: None,
                    cwd: None,
                    shell: None,
                    command: command.into(),
                    timeout_secs: None,
                })
                .await
                .unwrap_or_else(|e| panic!("{command} did not run: {e:#}"));
            assert_eq!(
                out.shell, "powershell",
                "auto must pick PowerShell here, so {command:?} is not sent to cmd: {out:?}"
            );
            assert_eq!(out.exit_code, Some(0), "{command} -> {out:?}");
        }
        let out = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: None,
                command: r#"Write-Output "ps-output-ok""#.into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert!(
            out.stdout_tail.contains("ps-output-ok"),
            "stdout was lost: {out:?}"
        );
        assert!(!out.stdout_tail.contains(CWD_MARKER), "{out:?}");
    }

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn powershell_mode_propagates_exit_codes_exactly() {
        let (_dir, root) = temp_repo();
        if !powershell_available() {
            eprintln!("SKIP: powershell.exe is not installed on this machine");
            return;
        }
        // A native command's own exit code survives the wrapper…
        let native = exec_with(&root, Some("powershell"), "cmd /c exit 7")
            .await
            .unwrap();
        assert_eq!(native.exit_code, Some(7), "{native:?}");
        assert_eq!(native.shell, "powershell", "{native:?}");

        // …and a failing cmdlet — which sets no exit code at all — must not
        // be masked as success by a stale $LASTEXITCODE.
        let cmdlet = exec_with(
            &root,
            Some("powershell"),
            "Get-Item __zylcode_no_such_file__",
        )
        .await
        .unwrap();
        assert_ne!(
            cmdlet.exit_code,
            Some(0),
            "cmdlet failure was masked: {cmdlet:?}"
        );
        assert!(!cmdlet.stderr_tail.is_empty(), "{cmdlet:?}");
    }

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn cmd_mode_propagates_exit_codes_exactly() {
        let (_dir, root) = temp_repo();
        let out = exec_with(&root, Some("cmd"), "cmd /c exit 7")
            .await
            .unwrap();
        assert_eq!(out.exit_code, Some(7), "{out:?}");
        assert_eq!(out.shell, "cmd", "{out:?}");
    }

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn a_failing_powershell_cmdlet_reports_a_readable_error() {
        // Regression: a redirected powershell.exe stderr serializes error
        // records as CLIXML, so the user saw XML instead of "Cannot find
        // path ...". The decoder must yield the message PowerShell wrote.
        let (_dir, root) = temp_repo();
        if !powershell_available() {
            eprintln!("SKIP: powershell.exe is not installed on this machine");
            return;
        }
        let out = exec_with(
            &root,
            Some("powershell"),
            "Get-Item __zylcode_no_such_file__",
        )
        .await
        .unwrap();
        assert_ne!(out.exit_code, Some(0), "{out:?}");
        assert!(
            out.stderr_tail.contains("Cannot find") || out.stderr_tail.contains("Get-Item"),
            "the real error message must survive: {out:?}"
        );
        assert!(
            !out.stderr_tail.contains("CLIXML") && !out.stderr_tail.contains("<Objs"),
            "CLIXML serialization leaked into the user-visible stderr: {out:?}"
        );
    }

    #[test]
    fn clixml_decoder_extracts_the_error_records_only() {
        let raw = concat!(
            "#< CLIXML\r\n",
            "<Objs Version=\"1.1.0.1\" xmlns=\"http://schemas.microsoft.com/powershell/2004/04\">",
            "<Obj S=\"progress\" RefId=\"0\"><MS><PR N=\"Record\"><AV>noise</AV></PR></MS></Obj>",
            "<S S=\"Error\">Get-Item : Cannot find path &#39;C:\\x&#39;._x000D__x000A_</S>",
            "<S S=\"Error\">At line:3 char:1_x000D__x000A_</S>",
            "</Objs>"
        );
        let decoded = decode_powershell_clixml(raw).expect("CLIXML must decode");
        assert!(decoded.contains("Cannot find path 'C:\\x'."), "{decoded}");
        assert!(decoded.contains("At line:3 char:1"), "{decoded}");
        // Multiple records join on their own lines, progress noise is dropped.
        assert!(!decoded.contains("noise"), "{decoded}");
        assert!(!decoded.contains("Objs"), "{decoded}");
    }

    #[test]
    fn clixml_decoder_leaves_ordinary_stderr_untouched() {
        for raw in [
            "bash: foo: command not found",
            "fatal: not a git repository (or any of the parent directories): .git",
            "",
            "#< CLIXML but not an Objs document",
        ] {
            assert_eq!(
                decode_powershell_clixml(raw),
                None,
                "must not guess at non-CLIXML input: {raw:?}"
            );
        }
    }

    #[test]
    fn ps_escapes_decode_in_one_pass_without_double_decoding() {
        // `_x005F_x000A_` is an escaped literal `_x000A_`, not a newline.
        assert_eq!(unescape_ps_xml("a_x000D__x000A_b"), "a\r\nb");
        assert_eq!(unescape_ps_xml("lit_x005F_x000A_end"), "lit_x000A_end");
        assert_eq!(unescape_ps_xml("no escapes here"), "no escapes here");
        assert_eq!(unescape_ps_xml("partial_x"), "partial_x");
    }

    #[test]
    fn xml_entities_decode_including_numeric_references() {
        assert_eq!(
            unescape_xml_entities("a &lt;b&gt; &amp; &quot;c&quot; &apos;d&apos;"),
            "a <b> & \"c\" 'd'"
        );
        assert_eq!(unescape_xml_entities("&#39;q&#39; &#x2713;"), "'q' ✓");
        assert_eq!(
            unescape_xml_entities("bare & ampersand"),
            "bare & ampersand"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn posix_mode_propagates_exit_codes_exactly() {
        let (_dir, root) = temp_repo();
        let out = exec_with(&root, Some("sh"), "sh -c \"exit 7\"")
            .await
            .unwrap();
        assert_eq!(out.exit_code, Some(7), "{out:?}");
        assert_eq!(out.shell, "sh", "{out:?}");
    }

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn powershell_mode_keeps_stdout_and_stderr_separate() {
        let (_dir, root) = temp_repo();
        if !powershell_available() {
            eprintln!("SKIP: powershell.exe is not installed on this machine");
            return;
        }
        let out = exec_with(
            &root,
            Some("powershell"),
            r#"Write-Output "to-out"; [Console]::Error.WriteLine("to-err")"#,
        )
        .await
        .unwrap();
        assert_eq!(out.exit_code, Some(0), "{out:?}");
        assert!(
            out.stdout_tail.contains("to-out") && !out.stdout_tail.contains("to-err"),
            "stdout polluted: {out:?}"
        );
        assert!(
            out.stderr_tail.contains("to-err") && !out.stderr_tail.contains("to-out"),
            "stderr polluted: {out:?}"
        );
        // PowerShell's progress records serialize as CLIXML on a redirected
        // stderr; the wrapper silences them so stderr carries only the
        // command's own errors.
        assert!(
            !out.stderr_tail.contains("CLIXML"),
            "progress noise leaked into stderr: {out:?}"
        );
    }

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn powershell_mode_does_not_swallow_formatted_output() {
        // Regression: the wrapper used to end with `exit $zyl_rc`. PowerShell
        // formats pipeline output *lazily*, at the end of the pipeline, and
        // `exit` terminated the process before that flush — so a command like
        // `Get-Location` (whose output is a formatted object, not text)
        // returned exit 0 with completely empty stdout.
        let (_dir, root) = temp_repo();
        if !powershell_available() {
            eprintln!("SKIP: powershell.exe is not installed on this machine");
            return;
        }
        // A real file so `Get-ChildItem -File` has something to format.
        std::fs::write(root.join("present.txt"), "x").unwrap();
        for command in [
            "Get-Location",
            "Get-ChildItem -File | Select-Object -ExpandProperty Name",
            "Get-Date",
        ] {
            let out = exec_with(&root, Some("powershell"), command).await.unwrap();
            assert_eq!(out.exit_code, Some(0), "{command} -> {out:?}");
            assert!(
                !out.stdout_tail.trim().is_empty(),
                "{command} produced no output — formatted output was discarded: {out:?}"
            );
        }
        // And the path really is the session directory.
        let out = exec_with(&root, Some("powershell"), "Get-Location")
            .await
            .unwrap();
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        assert!(
            out.stdout_tail.contains(&name),
            "expected the session directory in Get-Location's output: {out:?}"
        );
    }

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn reported_cwd_never_carries_the_verbatim_prefix() {
        // `canonicalize` on Windows returns `\\?\C:\...`; PowerShell's own
        // `$PWD` reports the plain form, so an unstripped root makes the
        // session cwd disagree with the shell's and leaks `\\?\` into the UI.
        let (_dir, root) = temp_repo();
        let verbatim = PathBuf::from(format!(r"\\?\{}", root.display()));
        if !verbatim.is_dir() {
            eprintln!("SKIP: verbatim path form unavailable here");
            return;
        }
        let hub = TerminalHub::new(verbatim);
        let out = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: None,
                command: "echo hi".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert!(
            !out.cwd.contains(r"\\?\"),
            "verbatim prefix leaked into the reported cwd: {:?}",
            out.cwd
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn posix_mode_keeps_stdout_and_stderr_separate() {
        let (_dir, root) = temp_repo();
        let out = exec_with(&root, Some("sh"), "echo to-out; echo to-err 1>&2")
            .await
            .unwrap();
        assert_eq!(out.exit_code, Some(0), "{out:?}");
        assert!(out.stdout_tail.contains("to-out"), "{out:?}");
        assert!(out.stderr_tail.contains("to-err"), "{out:?}");
        assert!(!out.stderr_tail.contains("to-out"), "{out:?}");
    }

    // -- working directory ------------------------------------------------

    #[tokio::test]
    async fn git_runs_from_the_workspace_repository_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        // Prepare a real repository (not a mock) for the terminal to run in.
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&root)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            match status {
                Ok(s) => s.success(),
                Err(_) => false,
            }
        };
        if !git(&["init"]) {
            eprintln!("SKIP: git is not available in this environment");
            return;
        }
        let hub = TerminalHub::new(root.clone());
        let out = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: None,
                command: "git status --short --branch".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert_eq!(
            out.exit_code,
            Some(0),
            "git must work in the workspace: {out:?}"
        );
        assert!(
            out.stdout_tail.contains("## "),
            "expected branch output, got: {out:?}"
        );
    }

    #[tokio::test]
    async fn missing_working_directory_is_reported_clearly() {
        let (_dir, _root) = temp_repo();
        let gone = PathBuf::from("__zylcode_no_such_workspace_dir__");
        let err = exec_with(&gone, Some("cmd"), "echo hi").await.unwrap_err();
        let err = format!("{err:#}");
        assert!(err.contains("does not exist"), "{err}");
        assert!(err.contains("__zylcode_no_such_workspace_dir__"), "{err}");
    }

    #[tokio::test]
    async fn session_reset_forgets_the_working_directory() {
        let (_dir, root) = temp_repo();
        let hub = TerminalHub::new(root.clone());
        let first = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: None,
                command: "cd sub".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert!(
            !hub.session_ids().is_empty(),
            "the session must exist first"
        );
        assert!(same_path(&first.cwd, &root.join("sub")), "{}", first.cwd);

        hub.reset();
        assert!(hub.session_ids().is_empty(), "reset must forget sessions");

        // A session id that predates the reset starts over at the workspace
        // root — it is never revived with stale state.
        let after = hub
            .exec(&TerminalRequest {
                session_id: Some(first.session_id.clone()),
                cwd: None,
                shell: None,
                command: "echo after-reset".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert!(same_path(&after.cwd, &root), "{}", after.cwd);
    }

    // -- configuration and malformed input --------------------------------

    #[tokio::test]
    async fn an_unknown_configured_shell_is_an_error_not_a_silent_default() {
        let (_dir, root) = temp_repo();
        let hub = TerminalHub::with_mode(
            root.clone(),
            Err("invalid ZYLCODE_TERMINAL_SHELL: fish".to_string()),
        );
        let err = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: None,
                command: "echo hi".into(),
                timeout_secs: None,
            })
            .await
            .unwrap_err();
        assert!(
            format!("{err:#}").contains("invalid ZYLCODE_TERMINAL_SHELL"),
            "{err:#}"
        );
    }

    #[tokio::test]
    async fn nul_bytes_in_a_command_are_rejected() {
        let (_dir, root) = temp_repo();
        let hub = TerminalHub::new(root);
        let err = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: None,
                command: "echo hi\0--smuggled".into(),
                timeout_secs: None,
            })
            .await
            .unwrap_err();
        assert!(format!("{err:#}").contains("NUL"), "{err:#}");
    }

    #[tokio::test]
    async fn an_unknown_shell_in_a_request_is_rejected() {
        let (_dir, root) = temp_repo();
        let err = exec_with(&root, Some("fish"), "echo hi").await.unwrap_err();
        assert!(
            format!("{err:#}").contains("unsupported shell mode"),
            "{err:#}"
        );
    }

    // -- PowerShell wrapper shape -----------------------------------------

    #[test]
    fn powershell_wrapper_is_encoded_and_carries_markers_without_an_exit() {
        let (program, args, encoded) = Shell::PowerShell.wrap("Get-Location");
        assert_eq!(program, "powershell.exe");
        assert_eq!(
            args,
            vec![
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-EncodedCommand".to_string()
            ]
        );
        let script = decode_utf16le_base64(&encoded);
        assert!(script.contains("Get-Location"), "{script}");
        assert!(
            script.contains(&format!("Write-Output \"{CWD_MARKER}$PWD\"")),
            "the cwd marker must survive encoding: {script}"
        );
        assert!(
            script.contains(&format!("Write-Output \"{RC_MARKER}$zyl_rc\"")),
            "the status must be reported as a marker: {script}"
        );
        // PowerShell formats pipeline output lazily and an `exit` statement
        // kills the process before that flush — swallowing formatted output
        // such as Get-Location's table. The status is reported by marker
        // instead, and the script is allowed to end on its own.
        assert!(
            !script.contains("exit $zyl_rc"),
            "an `exit` here silently discards the command's output: {script}"
        );
        // Progress records must be silenced *before* the first cmdlet, or
        // they serialize onto redirected stderr as CLIXML noise.
        assert!(
            script.starts_with("$ProgressPreference = 'SilentlyContinue'\n"),
            "progress silencing must come first: {script}"
        );
        // Every tail statement starts its own line: a trailing `#` comment in
        // the user's command must not swallow a marker.
        assert!(script.contains("$zyl_ok = $?\n"), "{script}");

        // pwsh is the same wrapper against the PowerShell 7 executable.
        let (program, args, _) = Shell::Pwsh.wrap("Get-Location");
        assert_eq!(program, "pwsh.exe");
        assert_eq!(args.last().unwrap(), "-EncodedCommand");
    }

    #[test]
    fn encode_utf16le_base64_matches_known_vectors() {
        // base64 over UTF-16LE bytes, exactly as -EncodedCommand documents:
        // "a"  -> 61 00                    -> YQA=
        // "ab" -> 61 00 62 00              -> YQBiAA==
        // "hi" -> 68 00 69 00              -> aABpAA==
        assert_eq!(encode_utf16le_base64("a"), "YQA=");
        assert_eq!(encode_utf16le_base64("ab"), "YQBiAA==");
        assert_eq!(encode_utf16le_base64("hi"), "aABpAA==");
        for sample in [
            "",
            "a",
            "ab",
            "abc",
            "hi",
            "Get-Location; exit 0",
            "ünïcödé ✓",
            "🦀",
        ] {
            assert_eq!(
                decode_utf16le_base64(&encode_utf16le_base64(sample)),
                sample,
                "round-trip failed for {sample:?}"
            );
        }
    }

    /// Decode what [`encode_utf16le_base64`] produced — a second, separate
    /// implementation is what keeps the first one honest.
    fn decode_utf16le_base64(input: &str) -> String {
        const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let chars: Vec<u8> = input
            .bytes()
            .filter(|b| *b != b'=')
            .filter_map(|b| {
                ALPHABET
                    .as_bytes()
                    .iter()
                    .position(|a| *a == b)
                    .map(|p| p as u8)
            })
            .collect();
        let mut wide: Vec<u8> = Vec::with_capacity(chars.len() / 4 * 3 + 3);
        for chunk in chars.chunks(4) {
            let n =
                chunk.iter().fold(0u32, |acc, c| (acc << 6) | *c as u32) << (6 * (4 - chunk.len()));
            wide.push((n >> 16) as u8);
            if chunk.len() >= 3 {
                wide.push((n >> 8) as u8);
            }
            if chunk.len() >= 4 {
                wide.push(n as u8);
            }
        }
        // `as_chunks` splits into `[u8; 2]` units and returns any trailing
        // odd byte separately. Ignoring that remainder is exactly what
        // `chunks_exact(2)` did: an incomplete trailing unit is dropped, not
        // reinterpreted.
        let units: Vec<u16> = wide
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        String::from_utf16(&units).expect("decoded script must be valid UTF-16")
    }

    // -- wrapper-echo stripping ------------------------------------------
    //
    // `-EncodedCommand` makes PowerShell's `<CommandLine> : <Message>` header
    // the entire decoded script, so a failing command used to open its stderr
    // with the wrapper verbatim — markers included. These pin the strip.

    /// The shape PowerShell actually emits for a failing command, captured
    /// from PowerShell 5.1: every script line, blank-line separated, then
    /// ` : <message>`, then the genuine diagnostic lines.
    fn failing_powershell_stderr(command: &str) -> String {
        let mut echoed = String::new();
        for line in powershell_script(command).lines() {
            echoed.push_str(line);
            echoed.push_str("\r\n\n");
        }
        format!(
            "{echoed} : Intentional terminal error\r\n\n    \
             + CategoryInfo          : NotSpecified: (:) [Write-Error], WriteErrorException\r\n\n"
        )
    }

    #[test]
    fn wrapper_echo_is_stripped_from_a_failing_command() {
        let command = "Write-Error \"Intentional terminal error\"";
        let stripped = strip_wrapper_echo(&failing_powershell_stderr(command), command);

        assert!(
            !stripped.contains(CWD_MARKER),
            "cwd marker leaked: {stripped:?}"
        );
        assert!(
            !stripped.contains(RC_MARKER),
            "rc marker leaked: {stripped:?}"
        );
        assert!(
            !stripped.contains("$ProgressPreference") && !stripped.contains("$zyl_"),
            "wrapper internals leaked: {stripped:?}"
        );
        // The genuine message and PowerShell's own diagnostics survive.
        assert!(
            stripped.contains("Intentional terminal error"),
            "real error message was lost: {stripped:?}"
        );
        assert!(
            stripped.contains("CategoryInfo"),
            "genuine PowerShell diagnostics were lost: {stripped:?}"
        );
        assert!(
            !stripped.trim_start().starts_with(':'),
            "the bare separator colon should not lead the message: {stripped:?}"
        );
    }

    #[test]
    fn unrelated_stderr_is_never_rewritten() {
        // No wrapper echo: ordinary stderr must pass through byte for byte,
        // including text that merely mentions a marker.
        let original = "fatal: not a git repository\nhint: try __ZYLCODE_CWD__ elsewhere\n";
        assert_eq!(strip_wrapper_echo(original, "git status"), original);
    }

    #[test]
    fn wrapper_echo_strip_keeps_output_when_only_partially_matching() {
        // First line matches the wrapper but the sequence breaks: refuse to
        // guess, and return the input untouched.
        let partial = "$ProgressPreference = 'SilentlyContinue'\nsomething else entirely\n";
        assert_eq!(strip_wrapper_echo(partial, "echo hi"), partial.to_string());
    }

    // -- clean output and accurate exit codes, end to end -----------------

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn a_failing_cmdlet_reports_nonzero_and_hides_the_wrapper() {
        let (_dir, root) = temp_repo();
        let out = exec_with(
            &root,
            Some("powershell"),
            "Write-Error \"Intentional terminal error\"",
        )
        .await
        .unwrap();
        assert_eq!(out.exit_code, Some(1), "{out:?}");
        assert!(
            out.stderr_tail.contains("Intentional terminal error"),
            "the real error must stay visible: {out:?}"
        );
        assert!(!out.stderr_tail.contains(CWD_MARKER), "{out:?}");
        assert!(!out.stderr_tail.contains(RC_MARKER), "{out:?}");
        assert!(!out.stderr_tail.contains("$zyl_"), "{out:?}");
        assert!(!out.stdout_tail.contains(CWD_MARKER), "{out:?}");
    }

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn a_successful_cmdlet_reports_zero_and_no_wrapper() {
        let (_dir, root) = temp_repo();
        let out = exec_with(&root, Some("powershell"), "Write-Output \"clean-ok\"")
            .await
            .unwrap();
        assert_eq!(out.exit_code, Some(0), "{out:?}");
        assert!(out.stdout_tail.contains("clean-ok"), "{out:?}");
        assert!(!out.stdout_tail.contains(CWD_MARKER), "{out:?}");
        assert!(!out.stderr_tail.contains(RC_MARKER), "{out:?}");
    }

    #[cfg(target_family = "windows")]
    #[tokio::test]
    async fn a_failing_command_does_not_corrupt_the_working_directory() {
        let (_dir, root) = temp_repo();
        let hub = TerminalHub::new(root.clone());
        let failed = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                shell: Some("powershell".into()),
                command: "Write-Error \"nope\"".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert_eq!(failed.exit_code, Some(1), "{failed:?}");
        assert!(
            same_path(&failed.cwd, &root),
            "a failed command must not move the session: {} vs {}",
            failed.cwd,
            root.display()
        );

        // And the session is still usable in the same directory afterwards.
        let next = hub
            .exec(&TerminalRequest {
                session_id: Some(failed.session_id.clone()),
                cwd: None,
                shell: Some("powershell".into()),
                command: "Write-Output \"still-here\"".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert_eq!(next.exit_code, Some(0), "{next:?}");
        assert!(
            same_path(&next.cwd, &root),
            "cwd drifted after a failure: {}",
            next.cwd
        );
    }
}
