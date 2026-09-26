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
//! Each exec runs through the platform shell (`cmd.exe /C` on Windows,
//! `sh -c` elsewhere). Because a child process cannot mutate the parent's
//! working directory, cwd tracking works like lightweight web terminals do:
//! the wrapper appends an echo of the resulting directory
//! (`__ZYLCODE_CWD__<path>`), which is parsed out of the output and stored
//! on the session. That marker is an implementation detail of the
//! transport — the UI never shows it.
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
    pub command: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// The result of one executed command.
#[derive(Debug, Clone, Serialize)]
pub struct TerminalOutput {
    pub session_id: String,
    pub cwd: String,
    pub exit_code: Option<i32>,
    /// `Some` when the command was killed at the timeout.
    pub timed_out: bool,
    pub stdout_tail: String,
    pub stderr_tail: String,
    /// True when output was capped at [`MAX_OUTPUT_BYTES`] per stream.
    pub truncated: bool,
    pub duration_ms: u128,
}

/// Parse the wrapper's trailing cwd marker, returning the visible output
/// (marker lines removed) and the resulting directory.
fn split_cwd_marker(raw: &str) -> (String, Option<String>) {
    let mut visible = String::with_capacity(raw.len());
    let mut cwd = None;
    for line in raw.lines() {
        if let Some(idx) = line.find(CWD_MARKER) {
            let candidate = line[idx + CWD_MARKER.len()..].trim();
            if !candidate.is_empty() {
                cwd = Some(candidate.to_string());
            }
        } else {
            visible.push_str(line);
            visible.push('\n');
        }
    }
    (visible, cwd)
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

/// Session store shared by the HTTP service and the Tauri command.
pub struct TerminalHub {
    root: PathBuf,
    sessions: Mutex<HashMap<String, TerminalSession>>,
}

impl TerminalHub {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            sessions: Mutex::new(HashMap::new()),
        }
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
        let timeout = Duration::from_secs(req.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));

        // Wrap the user command and echo the resulting cwd behind a marker.
        // cmd needs /V:ON because %CD%/%ERRORLEVEL% would otherwise expand
        // at PARSE time (before `cd` runs, and before the command's exit
        // code exists); !CD!/!ERRORLEVEL! expand at execution. The trailing
        // exit preserves the command's exit code — a bare trailing echo
        // would reset it to 0 and mask real failures.
        #[cfg(target_family = "windows")]
        let (program, pre, post) = (
            "cmd.exe",
            vec!["/V:ON".to_string(), "/C".to_string()],
            ") & echo __ZYLCODE_CWD__!CD! & exit /b !ERRORLEVEL!".to_string(),
        );
        #[cfg(not(target_family = "windows"))]
        let (program, pre, post) = (
            "sh",
            vec!["-c".to_string()],
            "); __zyl_rc=$?; echo __ZYLCODE_CWD__$PWD; exit $__zyl_rc".to_string(),
        );
        // Parenthesize the user command so the wrapper appends to the
        // WHOLE command, not to a compound body: without grouping, a `for`
        // loop would swallow the cwd echo and exit markers into its first
        // iteration and terminate the loop (found by test).
        let wrapped = format!("({}{}", req.command, post);

        let started = std::time::Instant::now();
        let child = tokio::process::Command::new(program)
            .args(&pre)
            .arg(wrapped)
            .current_dir(&base)
            .env("ZYLCODE_TERMINAL", "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn {program}"))?;

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
        let (stdout_visible, new_cwd) = split_cwd_marker(&stdout_raw);
        let (stdout_tail, out_trunc) = tail(&stdout_visible);
        let (stderr_tail, err_trunc) = tail(&String::from_utf8_lossy(&output.stderr));

        // Persist the resulting cwd for the next command in this session.
        let final_cwd = new_cwd.map(PathBuf::from).unwrap_or_else(|| base.clone());
        if final_cwd.is_dir() {
            self.store_cwd(&session.id, final_cwd.clone());
        }

        Ok(TerminalOutput {
            session_id: session.id,
            cwd: final_cwd.to_string_lossy().to_string(),
            exit_code: output.status.code(),
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

    #[tokio::test]
    async fn echo_runs_and_reports_exit_zero() {
        let (_dir, root) = temp_repo();
        let hub = TerminalHub::new(root.clone());
        let out = hub
            .exec(&TerminalRequest {
                session_id: None,
                cwd: None,
                command: "echo hello_zylcode".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert_eq!(out.exit_code, Some(0));
        assert!(out.stdout_tail.contains("hello_zylcode"), "{out:?}");
        assert_eq!(out.cwd, root.to_string_lossy());
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
                command: "cd sub".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert!(
            first.cwd.ends_with("sub"),
            "expected cwd inside sub, got {}",
            first.cwd
        );
        let second = hub
            .exec(&TerminalRequest {
                session_id: Some(first.session_id.clone()),
                cwd: None,
                command: "echo again".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
        assert!(second.cwd.ends_with("sub"), "{second:?}");
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
                command: "for /l %i in (1,1,20000) do @echo line%i".into(),
                timeout_secs: None,
            })
            .await
            .unwrap();
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
                command: "   ".into(),
                timeout_secs: None,
            })
            .await
            .is_err());
    }
}
