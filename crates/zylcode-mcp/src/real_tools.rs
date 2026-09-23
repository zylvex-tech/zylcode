//! Real Tool Runtime - Replaces mocked tool execution with actual operations
//!
//! This module provides real filesystem, shell, git, and search operations
//! instead of the mocked implementations in `tool.rs`.
//!
//! # Dispatch binding
//!
//! A tool id **constrains the operation it authorises**. `git.commit` runs
//! `git commit` and nothing else; `npm.run` runs `npm run` and nothing else.
//! Callers supply arguments, never the program or the subcommand. A request
//! that tries to substitute a different operation fails closed with
//! [`ToolError::OperationNotPermitted`].
//!
//! `shell.execute` is the single deliberate escape hatch: it is unbound, and it
//! carries [`RiskLevel::Execute`] so a permission gate can treat it
//! differently from the specialised tools.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::process::Command;

/// Typed failure modes for tool dispatch.
///
/// These exist so that an unsupported or unbound request can fail *loudly*.
/// The one thing a tool must never do is report success for work it did not
/// perform — see `docs/governance/TOOL_CATALOGUE_TRUTH_TABLE.md` §1.1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum ToolError {
    /// No tool is registered under this id.
    UnsupportedTool(String),
    /// The tool is catalogued but has no executor. Definition-only entries
    /// land here — they must never be executed.
    NotImplemented(String),
    /// An executor exists but is not currently usable.
    ExecutorUnavailable(String),
    /// The caller asked this tool id to perform an operation it does not
    /// authorise (e.g. `git.commit` asked to run `git push`).
    OperationNotPermitted {
        tool_id: String,
        requested: String,
        permitted: String,
    },
    /// The permission gate refused the invocation. The operation did **not** run.
    PermissionDenied { tool_id: String, reason: String },
    /// The request was malformed (missing or wrong-typed parameters).
    InvalidRequest { tool_id: String, reason: String },
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedTool(id) => write!(f, "unsupported tool: {id}"),
            Self::NotImplemented(id) => {
                write!(f, "tool `{id}` is definition-only and has no executor")
            }
            Self::ExecutorUnavailable(id) => write!(f, "executor unavailable for tool `{id}`"),
            Self::OperationNotPermitted {
                tool_id,
                requested,
                permitted,
            } => write!(
                f,
                "tool `{tool_id}` is bound to `{permitted}` and may not perform `{requested}`"
            ),
            Self::PermissionDenied { tool_id, reason } => {
                write!(f, "permission denied for tool `{tool_id}`: {reason}")
            }
            Self::InvalidRequest { tool_id, reason } => {
                write!(f, "invalid request for tool `{tool_id}`: {reason}")
            }
        }
    }
}

impl std::error::Error for ToolError {}

/// Risk level for tools.
///
/// The derived ordering is the **severity order**, lowest to highest:
/// `Read < Write < Execute < Network < GitWrite < Destructive < Release`.
/// A permission policy compares against this, so the variant order is load
/// bearing and must not be reshuffled casually.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RiskLevel {
    Read,
    Write,
    Execute,
    Network,
    GitWrite,
    Destructive,
    Release,
}

/// Tool schema exposed to models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub id: String,
    pub description: String,
    pub input_schema: Value,
    pub risk_class: RiskLevel,
    pub available: bool,
}

/// Evidence record for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEvidence {
    pub invocation_id: String,
    pub tool_id: String,
    pub session_id: Option<String>,
    /// Who or what initiated the invocation. Recorded so an audit can attribute
    /// an action to a person or an agent run rather than to "the app".
    pub actor: Option<String>,
    /// The operation this tool id is bound to, for audit.
    pub bound_operation: Option<String>,
    pub risk: RiskLevel,
    /// The permission gate's decision, recorded whether or not the tool ran.
    ///
    /// A denial is an event worth keeping: it is the record that something was
    /// asked for and refused.
    pub approval_decision: Option<String>,
    pub arguments: Value,
    pub working_directory: PathBuf,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub exit_status: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub changed_files: Vec<PathBuf>,
    pub timeout: Option<Duration>,
}

/// Real tool execution trait
#[async_trait]
pub trait RealTool: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &str;
    fn description(&self) -> &str;
    fn permissions(&self) -> ToolPermissions;
    /// Risk classification used by the permission gate.
    fn risk_level(&self) -> RiskLevel;
    /// The operation this tool id is bound to, for audit and for tests that
    /// assert the binding is enforced. `None` means unbound (`shell.execute`).
    fn bound_operation(&self) -> Option<String>;
    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult>;
}

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
    pub timeout: Duration,
    pub session_id: Option<String>,
    /// Who or what initiated this invocation — a user id, or an agent session
    /// id. `None` means unidentified, which a permission policy should treat as
    /// the least privileged case.
    pub actor: Option<String>,
    /// True when a human has explicitly approved this invocation.
    ///
    /// Read by [`crate::permission::PermissionPolicy::decide`]. It was
    /// previously passed in and never consumed.
    pub approval_required: bool,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: Value,
    pub evidence: ToolEvidence,
    pub changed_files: Vec<PathBuf>,
    pub duration: Duration,
}

/// Tool permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub network: bool,
    pub destructive: bool,
}

/// Filesystem tool for real file operations.
///
/// The tool id determines the action: `fs.read` reads, `fs.write` writes,
/// `fs.list` lists. A caller cannot ask `fs.read` to write.
#[derive(Debug)]
pub struct FileSystemTool {
    id: String,
    description: String,
    /// `read` | `write` | `list`, fixed by the id.
    bound_action: &'static str,
}

impl FileSystemTool {
    pub fn new(id: &str, description: &str) -> Self {
        let bound_action = match id {
            "fs.write" => "write",
            "fs.list" => "list",
            _ => "read",
        };
        Self {
            id: id.to_string(),
            description: description.to_string(),
            bound_action,
        }
    }
}

#[async_trait]
impl RealTool for FileSystemTool {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn risk_level(&self) -> RiskLevel {
        match self.bound_action {
            "write" => RiskLevel::Write,
            _ => RiskLevel::Read,
        }
    }

    fn bound_operation(&self) -> Option<String> {
        Some(format!("fs.{}", self.bound_action))
    }

    fn permissions(&self) -> ToolPermissions {
        let write = self.bound_action == "write";
        ToolPermissions {
            read: true,
            write,
            execute: false,
            network: false,
            destructive: false,
        }
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let start = Instant::now();

        // Dispatch binding: the id fixes the action. A caller that asks for a
        // different one is refused rather than silently obeyed.
        let action = match params.get("action").and_then(|v| v.as_str()) {
            Some(requested) if requested != self.bound_action => {
                return Err(ToolError::OperationNotPermitted {
                    tool_id: self.id.clone(),
                    requested: requested.to_string(),
                    permitted: self.bound_action.to_string(),
                }
                .into());
            }
            _ => self.bound_action,
        };

        let path = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let full_path = context.working_directory.join(path);

        let mut evidence = ToolEvidence::begin(
            &self.id,
            self.risk_level(),
            self.bound_operation(),
            &params,
            context,
        );

        let result = match action {
            "read" => match fs::read_to_string(&full_path).await {
                Ok(content) => {
                    evidence.stdout = Some(content.clone());
                    ToolResult {
                        success: true,
                        output: serde_json::json!({
                            "action": "read",
                            "path": path,
                            "content": content,
                            "size": content.len()
                        }),
                        evidence: evidence.clone(),
                        changed_files: Vec::new(),
                        duration: start.elapsed(),
                    }
                }
                Err(e) => {
                    evidence.stderr = Some(e.to_string());
                    evidence.exit_status = Some(1);
                    ToolResult {
                        success: false,
                        output: serde_json::json!({
                            "action": "read",
                            "path": path,
                            "error": e.to_string()
                        }),
                        evidence: evidence.clone(),
                        changed_files: Vec::new(),
                        duration: start.elapsed(),
                    }
                }
            },
            "write" => {
                let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("");

                match fs::write(&full_path, content).await {
                    Ok(_) => {
                        evidence.changed_files.push(full_path.clone());
                        ToolResult {
                            success: true,
                            output: serde_json::json!({
                                "action": "write",
                                "path": path,
                                "size": content.len()
                            }),
                            evidence: evidence.clone(),
                            changed_files: vec![full_path],
                            duration: start.elapsed(),
                        }
                    }
                    Err(e) => {
                        evidence.stderr = Some(e.to_string());
                        evidence.exit_status = Some(1);
                        ToolResult {
                            success: false,
                            output: serde_json::json!({
                                "action": "write",
                                "path": path,
                                "error": e.to_string()
                            }),
                            evidence: evidence.clone(),
                            changed_files: Vec::new(),
                            duration: start.elapsed(),
                        }
                    }
                }
            }
            "list" => match fs::read_dir(&full_path).await {
                Ok(mut entries) => {
                    let mut files = Vec::new();
                    while let Some(entry) = entries.next_entry().await? {
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        let file_type = entry.file_type().await?;
                        files.push(serde_json::json!({
                            "name": file_name,
                            "is_dir": file_type.is_dir(),
                            "is_file": file_type.is_file()
                        }));
                    }

                    evidence.stdout = Some(serde_json::to_string_pretty(&files)?);
                    ToolResult {
                        success: true,
                        output: serde_json::json!({
                            "action": "list",
                            "path": path,
                            "entries": files
                        }),
                        evidence: evidence.clone(),
                        changed_files: Vec::new(),
                        duration: start.elapsed(),
                    }
                }
                Err(e) => {
                    evidence.stderr = Some(e.to_string());
                    evidence.exit_status = Some(1);
                    ToolResult {
                        success: false,
                        output: serde_json::json!({
                            "action": "list",
                            "path": path,
                            "error": e.to_string()
                        }),
                        evidence: evidence.clone(),
                        changed_files: Vec::new(),
                        duration: start.elapsed(),
                    }
                }
            },
            _ => {
                evidence.stderr = Some(format!("Unknown action: {}", action));
                evidence.exit_status = Some(1);
                ToolResult {
                    success: false,
                    output: serde_json::json!({
                        "error": format!("Unknown action: {}", action)
                    }),
                    evidence: evidence.clone(),
                    changed_files: Vec::new(),
                    duration: start.elapsed(),
                }
            }
        };

        evidence.end_time = Some(chrono::Utc::now());
        Ok(result)
    }
}

/// Shell-backed tool.
///
/// `shell.execute` is the deliberate escape hatch: unbound, caller supplies the
/// program, classified [`RiskLevel::Execute`] so a permission gate can treat it
/// separately. Every other id is bound to a fixed program and a fixed leading
/// argument sequence, so `npm.run` cannot silently become `rm -rf`.
#[derive(Debug)]
pub struct ShellTool {
    id: String,
    description: String,
    /// `None` = unbound (`shell.execute`).
    bound_program: Option<&'static str>,
    /// Arguments always placed immediately after the program.
    bound_args_prefix: &'static [&'static str],
}

impl ShellTool {
    pub fn new(id: &str, description: &str) -> Self {
        let (bound_program, bound_args_prefix): (Option<&'static str>, &'static [&'static str]) =
            match id {
                "shell.execute" => (None, &[]),
                "shell.echo" => (Some("echo"), &[]),
                "npm.run" => (Some("npm"), &["run"]),
                "cargo.test" => (Some("cargo"), &["test"]),
                // Unknown id: refuse to guess. Unbound would be the unsafe
                // default, so bind to nothing and let `execute` reject it.
                _ => (Some(""), &[]),
            };
        Self {
            id: id.to_string(),
            description: description.to_string(),
            bound_program,
            bound_args_prefix,
        }
    }
}

/// Shell metacharacters that would allow argument injection when a command is
/// run through `cmd /C` on Windows. Rejected for *bound* tools; `shell.execute`
/// is the explicit escape hatch and is not filtered.
const SHELL_METACHARACTERS: &[char] = &['&', '|', '>', '<', '^', '%', ';', '`', '$', '\n', '\r'];

#[async_trait]
impl RealTool for ShellTool {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Execute
    }

    fn bound_operation(&self) -> Option<String> {
        match self.bound_program {
            None => None,
            Some("") => Some("<unbound: unrecognised tool id>".to_string()),
            Some(p) => {
                let mut s = p.to_string();
                for a in self.bound_args_prefix {
                    s.push(' ');
                    s.push_str(a);
                }
                Some(s)
            }
        }
    }

    fn permissions(&self) -> ToolPermissions {
        ToolPermissions {
            read: true,
            write: false,
            execute: true,
            network: false,
            destructive: false,
        }
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let start = Instant::now();

        // --- dispatch binding -------------------------------------------------
        // The tool id fixes the program. A caller may supply arguments, never a
        // different program.
        let caller_command = params.get("command").and_then(|v| v.as_str());
        let command: String = match self.bound_program {
            // Unbound escape hatch: the caller must name a program.
            None => match caller_command {
                Some(c) if !c.is_empty() => c.to_string(),
                _ => {
                    return Err(ToolError::InvalidRequest {
                        tool_id: self.id.clone(),
                        reason: "`shell.execute` requires a non-empty `command`".to_string(),
                    }
                    .into())
                }
            },
            // An id we do not recognise is refused outright rather than being
            // given unbound (unsafe) semantics.
            Some("") => return Err(ToolError::UnsupportedTool(self.id.clone()).into()),
            Some(bound) => {
                if let Some(c) = caller_command {
                    if c != bound {
                        return Err(ToolError::OperationNotPermitted {
                            tool_id: self.id.clone(),
                            requested: c.to_string(),
                            permitted: bound.to_string(),
                        }
                        .into());
                    }
                }
                bound.to_string()
            }
        };

        let mut caller_args: Vec<String> = params
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        // `shell.echo` uses `message` rather than an args array.
        if caller_args.is_empty() {
            if let Some(m) = params.get("message").and_then(|v| v.as_str()) {
                caller_args.push(m.to_string());
            }
        }

        // Bound tools run through `cmd /C` on Windows, where these characters
        // would allow argument injection. `shell.execute` is the documented
        // escape hatch for shell syntax and is deliberately not filtered.
        if self.bound_program.is_some() {
            for a in &caller_args {
                if let Some(bad) = a.chars().find(|c| SHELL_METACHARACTERS.contains(c)) {
                    return Err(ToolError::InvalidRequest {
                        tool_id: self.id.clone(),
                        reason: format!(
                            "argument {a:?} contains shell metacharacter {bad:?}; \
                             use `shell.execute` if shell syntax is intended"
                        ),
                    }
                    .into());
                }
            }
        }

        let mut args: Vec<String> = self
            .bound_args_prefix
            .iter()
            .map(|s| s.to_string())
            .collect();
        args.extend(caller_args);

        let mut evidence = ToolEvidence::begin(
            &self.id,
            self.risk_level(),
            self.bound_operation(),
            &params,
            context,
        );

        // On Windows, built-in commands like 'echo' need to be run via cmd /C
        let mut cmd = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C").arg(&command).args(&args);
            cmd
        } else {
            let mut cmd = Command::new(&command);
            cmd.args(&args);
            cmd
        };

        cmd.current_dir(&context.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add environment variables
        for (key, value) in &context.environment {
            cmd.env(key, value);
        }

        match tokio::time::timeout(context.timeout, cmd.output()).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                evidence.stdout = Some(stdout.clone());
                evidence.stderr = Some(stderr.clone());
                evidence.exit_status = Some(exit_code);

                ToolResult {
                    success: output.status.success(),
                    output: serde_json::json!({
                        "command": &command,
                        "args": args,
                        "exit_code": exit_code,
                        "stdout": stdout,
                        "stderr": stderr
                    }),
                    evidence: evidence.clone(),
                    changed_files: Vec::new(),
                    duration: start.elapsed(),
                }
            }
            Ok(Err(e)) => {
                evidence.stderr = Some(e.to_string());
                evidence.exit_status = Some(1);
                ToolResult {
                    success: false,
                    output: serde_json::json!({
                        "command": &command,
                        "args": args,
                        "error": e.to_string()
                    }),
                    evidence: evidence.clone(),
                    changed_files: Vec::new(),
                    duration: start.elapsed(),
                }
            }
            Err(_) => {
                evidence.stderr = Some("Command timed out".to_string());
                evidence.exit_status = Some(124); // Timeout exit code
                ToolResult {
                    success: false,
                    output: serde_json::json!({
                        "command": &command,
                        "args": args,
                        "error": "Command timed out"
                    }),
                    evidence: evidence.clone(),
                    changed_files: Vec::new(),
                    duration: start.elapsed(),
                }
            }
        }
        .pipe(|mut result| {
            result.evidence.end_time = Some(chrono::Utc::now());
            Ok(result)
        })
    }
}

/// Git tool, bound to exactly one subcommand.
///
/// The tool id determines the subcommand: `git.status` runs `git status`,
/// `git.diff` runs `git diff`, `git.commit` runs `git commit`. A caller cannot
/// make `git.commit` perform `git push`, `git reset`, `git clean`,
/// `git checkout`, `git rebase`, `git config` or `git remote`.
#[derive(Debug)]
pub struct GitTool {
    id: String,
    description: String,
    /// Fixed by the id. A caller that names a different subcommand is refused.
    bound_subcommand: &'static str,
}

impl GitTool {
    pub fn new(id: &str, description: &str) -> Self {
        let bound_subcommand = match id {
            "git.diff" => "diff",
            "git.commit" => "commit",
            "git.status" => "status",
            // Unknown id: bind to the empty string so `execute` refuses it
            // rather than defaulting to something executable.
            _ => "",
        };
        Self {
            id: id.to_string(),
            description: description.to_string(),
            bound_subcommand,
        }
    }
}

#[async_trait]
impl RealTool for GitTool {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn risk_level(&self) -> RiskLevel {
        match self.bound_subcommand {
            "commit" => RiskLevel::GitWrite,
            _ => RiskLevel::Read,
        }
    }

    fn bound_operation(&self) -> Option<String> {
        if self.bound_subcommand.is_empty() {
            Some("<unbound: unrecognised tool id>".to_string())
        } else {
            Some(format!("git {}", self.bound_subcommand))
        }
    }

    fn permissions(&self) -> ToolPermissions {
        let write = self.bound_subcommand == "commit";
        ToolPermissions {
            read: true,
            write,
            execute: true,
            network: false,
            destructive: false,
        }
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let start = Instant::now();

        // --- dispatch binding -------------------------------------------------
        if self.bound_subcommand.is_empty() {
            return Err(ToolError::UnsupportedTool(self.id.clone()).into());
        }

        // A caller that names a subcommand must name *this* one. The previous
        // implementation took the subcommand from the caller, which meant
        // `git.commit` could run `git push`.
        if let Some(requested) = params.get("subcommand").and_then(|v| v.as_str()) {
            if requested != self.bound_subcommand {
                return Err(ToolError::OperationNotPermitted {
                    tool_id: self.id.clone(),
                    requested: requested.to_string(),
                    permitted: self.bound_subcommand.to_string(),
                }
                .into());
            }
        }

        let subcommand = self.bound_subcommand;

        // `git.commit` takes a message and optional paths, not free-form args:
        // free-form args would reopen the door to `--amend` and friends.
        let args: Vec<String> = if subcommand == "commit" {
            if params.get("args").is_some() {
                return Err(ToolError::InvalidRequest {
                    tool_id: self.id.clone(),
                    reason: "`git.commit` does not accept free-form `args`; \
                             use `message` and `files`"
                        .to_string(),
                }
                .into());
            }
            let message = params
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidRequest {
                    tool_id: self.id.clone(),
                    reason: "`git.commit` requires a `message`".to_string(),
                })?;
            let mut a = vec!["-m".to_string(), message.to_string()];
            if let Some(files) = params.get("files").and_then(|v| v.as_array()) {
                let paths: Vec<String> = files
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if !paths.is_empty() {
                    a.push("--".to_string());
                    a.extend(paths);
                }
            }
            a
        } else {
            params
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default()
        };

        let mut evidence = ToolEvidence::begin(
            &self.id,
            self.risk_level(),
            self.bound_operation(),
            &params,
            context,
        );

        let mut cmd = Command::new("git");
        cmd.arg(subcommand)
            .args(&args)
            .current_dir(&context.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match tokio::time::timeout(context.timeout, cmd.output()).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                evidence.stdout = Some(stdout.clone());
                evidence.stderr = Some(stderr.clone());
                evidence.exit_status = Some(exit_code);

                // Parse changed files from git status/diff
                let changed_files = if subcommand == "status" || subcommand == "diff" {
                    parse_git_changed_files(&stdout)
                } else {
                    Vec::new()
                };

                evidence.changed_files = changed_files.clone();

                ToolResult {
                    success: output.status.success(),
                    output: serde_json::json!({
                        "subcommand": subcommand,
                        "args": args,
                        "exit_code": exit_code,
                        "stdout": stdout,
                        "stderr": stderr,
                        "changed_files": changed_files
                    }),
                    evidence: evidence.clone(),
                    changed_files,
                    duration: start.elapsed(),
                }
            }
            Ok(Err(e)) => {
                evidence.stderr = Some(e.to_string());
                evidence.exit_status = Some(1);
                ToolResult {
                    success: false,
                    output: serde_json::json!({
                        "subcommand": subcommand,
                        "args": args,
                        "error": e.to_string()
                    }),
                    evidence: evidence.clone(),
                    changed_files: Vec::new(),
                    duration: start.elapsed(),
                }
            }
            Err(_) => {
                evidence.stderr = Some("Git command timed out".to_string());
                evidence.exit_status = Some(124);
                ToolResult {
                    success: false,
                    output: serde_json::json!({
                        "subcommand": subcommand,
                        "args": args,
                        "error": "Git command timed out"
                    }),
                    evidence: evidence.clone(),
                    changed_files: Vec::new(),
                    duration: start.elapsed(),
                }
            }
        }
        .pipe(|mut result| {
            result.evidence.end_time = Some(chrono::Utc::now());
            Ok(result)
        })
    }
}

/// Search tool, bound to one search mode.
///
/// `search.find` matches file names; `search.grep` matches file contents. The
/// tool id fixes which. Read-only, so the risk is low, but the binding is
/// enforced for the same reason as everywhere else: an id must mean one thing.
#[derive(Debug)]
pub struct SearchTool {
    id: String,
    description: String,
    /// `filename` (find) | `text` (grep).
    bound_mode: &'static str,
}

impl SearchTool {
    pub fn new(id: &str, description: &str) -> Self {
        let bound_mode = match id {
            "search.grep" => "text",
            "search.find" => "filename",
            _ => "",
        };
        Self {
            id: id.to_string(),
            description: description.to_string(),
            bound_mode,
        }
    }
}

#[async_trait]
impl RealTool for SearchTool {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Read
    }

    fn bound_operation(&self) -> Option<String> {
        match self.bound_mode {
            "" => Some("<unbound: unrecognised tool id>".to_string()),
            m => Some(format!("search mode `{m}`")),
        }
    }

    fn permissions(&self) -> ToolPermissions {
        ToolPermissions {
            read: true,
            write: false,
            execute: false,
            network: false,
            destructive: false,
        }
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let start = Instant::now();

        if self.bound_mode.is_empty() {
            return Err(ToolError::UnsupportedTool(self.id.clone()).into());
        }

        let pattern = params.get("pattern").and_then(|v| v.as_str()).unwrap_or("");

        // Dispatch binding: the id fixes the mode.
        let search_type = match params.get("type").and_then(|v| v.as_str()) {
            Some(requested) if requested != self.bound_mode => {
                return Err(ToolError::OperationNotPermitted {
                    tool_id: self.id.clone(),
                    requested: requested.to_string(),
                    permitted: self.bound_mode.to_string(),
                }
                .into());
            }
            _ => self.bound_mode,
        };

        let mut evidence = ToolEvidence::begin(
            &self.id,
            self.risk_level(),
            self.bound_operation(),
            &params,
            context,
        );

        let mut cmd = match search_type {
            "filename" => {
                let mut cmd = Command::new("find");
                cmd.arg(".")
                    .arg("-name")
                    .arg(pattern)
                    .current_dir(&context.working_directory)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                cmd
            }
            "text" => {
                let mut cmd = Command::new("grep");
                cmd.arg("-r")
                    .arg("-n")
                    .arg(pattern)
                    .arg(".")
                    .current_dir(&context.working_directory)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                cmd
            }
            _ => {
                evidence.stderr = Some(format!("Unknown search type: {}", search_type));
                evidence.exit_status = Some(1);
                return Ok(ToolResult {
                    success: false,
                    output: serde_json::json!({
                        "error": format!("Unknown search type: {}", search_type)
                    }),
                    evidence,
                    changed_files: Vec::new(),
                    duration: start.elapsed(),
                });
            }
        };

        match tokio::time::timeout(context.timeout, cmd.output()).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                evidence.stdout = Some(stdout.clone());
                evidence.stderr = Some(stderr.clone());
                evidence.exit_status = Some(exit_code);

                ToolResult {
                    success: output.status.success(),
                    output: serde_json::json!({
                        "pattern": pattern,
                        "type": search_type,
                        "exit_code": exit_code,
                        "results": stdout,
                        "errors": stderr
                    }),
                    evidence: evidence.clone(),
                    changed_files: Vec::new(),
                    duration: start.elapsed(),
                }
            }
            Ok(Err(e)) => {
                evidence.stderr = Some(e.to_string());
                evidence.exit_status = Some(1);
                ToolResult {
                    success: false,
                    output: serde_json::json!({
                        "pattern": pattern,
                        "type": search_type,
                        "error": e.to_string()
                    }),
                    evidence: evidence.clone(),
                    changed_files: Vec::new(),
                    duration: start.elapsed(),
                }
            }
            Err(_) => {
                evidence.stderr = Some("Search timed out".to_string());
                evidence.exit_status = Some(124);
                ToolResult {
                    success: false,
                    output: serde_json::json!({
                        "pattern": pattern,
                        "type": search_type,
                        "error": "Search timed out"
                    }),
                    evidence: evidence.clone(),
                    changed_files: Vec::new(),
                    duration: start.elapsed(),
                }
            }
        }
        .pipe(|mut result| {
            result.evidence.end_time = Some(chrono::Utc::now());
            Ok(result)
        })
    }
}

/// Parse changed files from git output
fn parse_git_changed_files(output: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        // Parse git status format: "XY filename"
        if line.len() > 3 {
            let status = &line[..2];
            let filename = line[3..].trim();

            if !filename.is_empty()
                && (status.contains('M') || status.contains('A') || status.contains('D'))
            {
                files.push(PathBuf::from(filename));
            }
        }
    }

    files
}

/// Get a real tool implementation by ID.
///
/// The returned executor is **bound**: its id constrains the operation it may
/// perform. Returns `None` for any id without a real executor — callers must
/// treat that as [`ToolError::NotImplemented`], never as a reason to fabricate
/// a success.
///
/// This is the authoritative list of executable ids. It must stay in step with
/// `crate::tool_catalogue::Catalogue`; the catalogue asserts the correspondence.
pub fn get_real_tool(tool_id: &str) -> Option<Box<dyn RealTool>> {
    match tool_id {
        "fs.read" | "fs.write" | "fs.list" => {
            Some(Box::new(FileSystemTool::new(tool_id, "File System Tool")))
        }
        "shell.execute" | "shell.echo" | "npm.run" | "cargo.test" => {
            Some(Box::new(ShellTool::new(tool_id, "Shell Tool")))
        }
        "git.status" | "git.diff" | "git.commit" => {
            Some(Box::new(GitTool::new(tool_id, "Git Tool")))
        }
        "search.find" | "search.grep" => Some(Box::new(SearchTool::new(tool_id, "Search Tool"))),
        _ => None,
    }
}

impl ToolEvidence {
    /// Start an evidence record for one invocation.
    ///
    /// Every executor uses this, so `tool_id`, `risk`, `bound_operation`,
    /// `actor` and `session_id` are populated uniformly and cannot drift
    /// between executors. `approval_decision` is filled in by
    /// [`dispatch`] once the gate has ruled.
    pub fn begin(
        tool_id: &str,
        risk: RiskLevel,
        bound_operation: Option<String>,
        params: &Value,
        context: &ToolContext,
    ) -> Self {
        Self {
            invocation_id: uuid::Uuid::new_v4().to_string(),
            tool_id: tool_id.to_string(),
            session_id: context.session_id.clone(),
            actor: context.actor.clone(),
            bound_operation,
            risk,
            approval_decision: None,
            arguments: params.clone(),
            working_directory: context.working_directory.clone(),
            start_time: chrono::Utc::now(),
            end_time: None,
            exit_status: None,
            stdout: None,
            stderr: None,
            changed_files: Vec::new(),
            timeout: Some(context.timeout),
        }
    }
}

/// The result of one gated dispatch.
#[derive(Debug)]
pub struct DispatchOutcome {
    /// What the gate decided. Present whether or not the tool ran.
    pub decision: crate::permission::PermissionDecision,
    /// The record of the attempt, including a refusal.
    pub evidence: ToolEvidence,
    /// `Ok` only when the gate allowed the invocation **and** the executor
    /// succeeded.
    pub result: Result<ToolResult>,
}

impl DispatchOutcome {
    pub fn executed(&self) -> bool {
        self.result.is_ok()
    }
}

/// Resolve, gate, and execute a tool.
///
/// This is the **only** path from a request to an executor. Both dispatch
/// points — `DynamicTool::call` (product-reachable) and `BuiltinTool::call`
/// (the bridge) — go through it, so the permission gate cannot be bypassed by
/// choosing a different entry point.
///
/// A refusal returns [`ToolError::PermissionDenied`] and **does not execute
/// anything**. That is the whole point: the gate is consulted before the
/// executor is reached, not after.
pub async fn dispatch(
    tool_id: &str,
    params: Value,
    context: &ToolContext,
    runtime: &crate::evidence::ToolRuntime,
) -> DispatchOutcome {
    use crate::permission::PermissionDecision;

    // Attribute the invocation. An actor set explicitly on the context wins;
    // otherwise the task-local binding; otherwise unidentified, which the
    // policy and the evidence record both treat as the least privileged case.
    let mut owned = context.clone();
    if owned.actor.is_none() {
        owned.actor = crate::actor::current_actor();
    }
    let context = &owned;

    // 1. Resolve. An id without an executor never reaches the gate.
    let tool = match get_real_tool(tool_id) {
        Some(t) => t,
        None => {
            let mut evidence =
                ToolEvidence::begin(tool_id, RiskLevel::Read, None, &params, context);
            let decision = PermissionDecision::Deny(format!("`{tool_id}` has no executor"));
            evidence.approval_decision = Some(decision.evidence_record());
            evidence.end_time = Some(chrono::Utc::now());
            // Record it: an attempt against an unsupported id is worth keeping.
            let _ = runtime.sink().record(&evidence).await;
            return DispatchOutcome {
                decision,
                evidence,
                result: Err(ToolError::NotImplemented(tool_id.to_string()).into()),
            };
        }
    };

    let risk = tool.risk_level();
    let bound_operation = tool.bound_operation();

    // 2. Gate. Recorded before execution, and recorded on refusal too.
    let decision = runtime.gate().decide(tool_id, risk, context);

    let mut evidence =
        ToolEvidence::begin(tool_id, risk, bound_operation.clone(), &params, context);
    evidence.approval_decision = Some(decision.evidence_record());

    let refusal = match &decision {
        PermissionDecision::Deny(r) | PermissionDecision::RequireApproval(r) => Some(r.clone()),
        PermissionDecision::Allow(_) => None,
    };
    if let Some(reason) = refusal {
        evidence.end_time = Some(chrono::Utc::now());
        // A refusal is evidence: it is the record that something was asked for
        // and denied. A trail that only keeps successes cannot answer "what was
        // attempted?".
        let _ = runtime.sink().record(&evidence).await;
        return DispatchOutcome {
            decision,
            evidence,
            result: Err(ToolError::PermissionDenied {
                tool_id: tool_id.to_string(),
                reason,
            }
            .into()),
        };
    }

    // 3. Execute.
    let outcome = tool.execute(params, context).await;
    let mut record = evidence;
    let result = match outcome {
        Ok(mut r) => {
            // The executor built its own evidence; carry the gate's decision
            // and the attribution onto it so the persisted record is complete.
            r.evidence.approval_decision = Some(decision.evidence_record());
            r.evidence.actor = context.actor.clone();
            r.evidence.bound_operation = bound_operation;
            r.evidence.risk = risk;
            record = r.evidence.clone();
            Ok(r)
        }
        Err(e) => {
            record.end_time = Some(chrono::Utc::now());
            record.stderr = Some(e.to_string());
            Err(e)
        }
    };

    let _ = runtime.sink().record(&record).await;
    DispatchOutcome {
        decision,
        evidence: record,
        result,
    }
}

/// Extension trait for pipe operations
trait Pipe<T> {
    fn pipe<F, U>(self, f: F) -> U
    where
        F: FnOnce(T) -> U;
}

impl<T> Pipe<T> for T {
    fn pipe<F, U>(self, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        f(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Shared execution context for binding tests.
    fn test_context() -> ToolContext {
        ToolContext {
            working_directory: PathBuf::from("."),
            environment: HashMap::new(),
            timeout: Duration::from_secs(5),
            session_id: None,
            actor: None,
            approval_required: false,
        }
    }

    // -----------------------------------------------------------------------
    // Dispatch binding — adversarial tests.
    //
    // The defect these exist to prevent: the executor used to take the
    // subcommand / program from *caller parameters*, so a tool labelled
    // `git.commit` could run `git push`. Every test below asserts that a
    // cross-operation substitution fails closed.
    // -----------------------------------------------------------------------

    /// The headline security case: `git.commit` must not be able to push.
    #[tokio::test]
    async fn git_commit_cannot_execute_git_push() {
        let tool = GitTool::new("git.commit", "Git commit");
        let err = tool
            .execute(serde_json::json!({ "subcommand": "push" }), &test_context())
            .await
            .expect_err("git.commit must not run git push");
        assert!(
            err.to_string().contains("may not perform"),
            "expected a binding refusal, got: {err}"
        );
    }

    /// Every dangerous subcommand is refused by `git.commit`.
    #[tokio::test]
    async fn git_commit_refuses_every_dangerous_subcommand() {
        let tool = GitTool::new("git.commit", "Git commit");
        for sub in [
            "push",
            "reset",
            "clean",
            "checkout",
            "rebase",
            "config",
            "remote",
            "filter-branch",
            "update-ref",
            "gc",
        ] {
            let err = tool
                .execute(
                    serde_json::json!({ "subcommand": sub, "args": ["--hard"] }),
                    &test_context(),
                )
                .await
                .unwrap_err();
            assert!(
                err.to_string().contains("may not perform"),
                "git.commit accepted `{sub}`: {err}"
            );
        }
    }

    /// `git.commit` takes a message, not free-form args: free-form args would
    /// reopen the door to `--amend` and friends.
    #[tokio::test]
    async fn git_commit_rejects_free_form_args() {
        let tool = GitTool::new("git.commit", "Git commit");
        let err = tool
            .execute(
                serde_json::json!({ "message": "x", "args": ["--amend"] }),
                &test_context(),
            )
            .await
            .expect_err("git.commit must reject free-form args");
        assert!(
            err.to_string().contains("does not accept free-form"),
            "{err}"
        );
    }

    /// `git.commit` requires a message.
    #[tokio::test]
    async fn git_commit_requires_a_message() {
        let tool = GitTool::new("git.commit", "Git commit");
        let err = tool
            .execute(serde_json::json!({}), &test_context())
            .await
            .expect_err("git.commit must require a message");
        assert!(err.to_string().contains("requires a `message`"), "{err}");
    }

    /// `git.status` must not be able to commit.
    #[tokio::test]
    async fn git_status_cannot_execute_git_commit() {
        let tool = GitTool::new("git.status", "Git status");
        let err = tool
            .execute(
                serde_json::json!({ "subcommand": "commit", "message": "sneaky" }),
                &test_context(),
            )
            .await
            .expect_err("git.status must not run git commit");
        assert!(err.to_string().contains("may not perform"), "{err}");
    }

    /// `git.diff` must not be able to check out a different branch.
    #[tokio::test]
    async fn git_diff_cannot_execute_git_checkout() {
        let tool = GitTool::new("git.diff", "Git diff");
        let err = tool
            .execute(
                serde_json::json!({ "subcommand": "checkout" }),
                &test_context(),
            )
            .await
            .expect_err("git.diff must not run git checkout");
        assert!(err.to_string().contains("may not perform"), "{err}");
    }

    /// Bindings are declared, not implied: each git id reports its operation.
    #[test]
    fn git_bindings_are_explicit() {
        assert_eq!(
            GitTool::new("git.status", "").bound_operation().as_deref(),
            Some("git status")
        );
        assert_eq!(
            GitTool::new("git.diff", "").bound_operation().as_deref(),
            Some("git diff")
        );
        assert_eq!(
            GitTool::new("git.commit", "").bound_operation().as_deref(),
            Some("git commit")
        );
        assert_eq!(GitTool::new("git.status", "").risk_level(), RiskLevel::Read);
        assert_eq!(
            GitTool::new("git.commit", "").risk_level(),
            RiskLevel::GitWrite
        );
    }

    /// An unrecognised git id fails closed rather than defaulting to `status`.
    #[tokio::test]
    async fn unrecognised_git_id_fails_closed() {
        let tool = GitTool::new("git.not-real", "?");
        let err = tool
            .execute(serde_json::json!({}), &test_context())
            .await
            .expect_err("an unknown git id must be refused");
        assert!(err.to_string().contains("unsupported tool"), "{err}");
    }

    /// `fs.read` must not be able to write.
    #[tokio::test]
    async fn fs_read_cannot_write() {
        let tool = FileSystemTool::new("fs.read", "read");
        let err = tool
            .execute(
                serde_json::json!({ "action": "write", "path": "pwned.txt", "content": "x" }),
                &test_context(),
            )
            .await
            .expect_err("fs.read must not write");
        assert!(err.to_string().contains("may not perform"), "{err}");
    }

    /// `fs.write` must not be able to read.
    #[tokio::test]
    async fn fs_write_cannot_read() {
        let tool = FileSystemTool::new("fs.write", "write");
        let err = tool
            .execute(
                serde_json::json!({ "action": "read", "path": "Cargo.toml" }),
                &test_context(),
            )
            .await
            .expect_err("fs.write must not read");
        assert!(err.to_string().contains("may not perform"), "{err}");
    }

    /// `search.find` must not become `search.grep`.
    #[tokio::test]
    async fn search_find_cannot_become_grep() {
        let tool = SearchTool::new("search.find", "find");
        let err = tool
            .execute(
                serde_json::json!({ "type": "text", "pattern": "secret" }),
                &test_context(),
            )
            .await
            .expect_err("search.find must not run grep");
        assert!(err.to_string().contains("may not perform"), "{err}");
    }

    /// `search.grep` must not become `search.find`.
    #[tokio::test]
    async fn search_grep_cannot_become_find() {
        let tool = SearchTool::new("search.grep", "grep");
        let err = tool
            .execute(
                serde_json::json!({ "type": "filename", "pattern": "*.rs" }),
                &test_context(),
            )
            .await
            .expect_err("search.grep must not run find");
        assert!(err.to_string().contains("may not perform"), "{err}");
    }

    /// Every executable id in the factory declares a binding (or is the one
    /// documented escape hatch).
    #[test]
    fn every_factory_tool_declares_its_binding() {
        for id in [
            "fs.read",
            "fs.write",
            "fs.list",
            "shell.execute",
            "shell.echo",
            "npm.run",
            "cargo.test",
            "git.status",
            "git.diff",
            "git.commit",
            "search.find",
            "search.grep",
        ] {
            let tool = get_real_tool(id).unwrap_or_else(|| panic!("no executor for {id}"));
            let bound = tool.bound_operation();
            if id == "shell.execute" {
                assert_eq!(bound, None, "shell.execute is the escape hatch");
            } else {
                let bound = bound.unwrap_or_else(|| panic!("{id} declares no binding"));
                assert!(
                    !bound.starts_with("<unbound"),
                    "{id} resolved to an unbound binding: {bound}"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_filesystem_tool_read() {
        let tool = FileSystemTool::new("fs.read", "Read file contents");
        let context = ToolContext {
            working_directory: PathBuf::from("."),
            environment: HashMap::new(),
            timeout: Duration::from_secs(5),
            session_id: None,
            actor: None,
            approval_required: false,
        };

        let params = serde_json::json!({
            "action": "read",
            "path": "Cargo.toml"
        });

        let result = tool.execute(params, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.get("content").is_some());
    }

    #[tokio::test]
    async fn test_shell_tool_echo() {
        let tool = ShellTool::new("shell.echo", "Echo command");
        let context = ToolContext {
            working_directory: PathBuf::from("."),
            environment: HashMap::new(),
            timeout: Duration::from_secs(5),
            session_id: None,
            actor: None,
            approval_required: false,
        };

        // `shell.echo` is bound to the `echo` program. The caller supplies the
        // text, not the program — the previous version of this test passed
        // `command: "cmd"`, which the binding now correctly refuses.
        let params = serde_json::json!({ "message": "hello world" });

        let result = tool.execute(params, &context).await.unwrap();
        assert!(result.success);
        let stdout = result.output.get("stdout").unwrap().as_str().unwrap();
        assert!(stdout.contains("hello world"));
    }

    /// The caller cannot retarget a bound tool at a different program.
    #[tokio::test]
    async fn bound_shell_tool_refuses_a_different_program() {
        let tool = ShellTool::new("npm.run", "npm");
        let context = test_context();

        let err = tool
            .execute(
                serde_json::json!({ "command": "rm", "args": ["-rf", "/"] }),
                &context,
            )
            .await
            .expect_err("npm.run must refuse a caller-supplied program");

        let msg = err.to_string();
        assert!(
            msg.contains("OperationNotPermitted") || msg.contains("may not perform"),
            "expected a binding refusal, got: {msg}"
        );
    }

    /// `npm.run` cannot silently become arbitrary `shell.execute`.
    #[tokio::test]
    async fn npm_run_is_pinned_to_npm_run() {
        let tool = ShellTool::new("npm.run", "npm");
        assert_eq!(tool.bound_operation().as_deref(), Some("npm run"));

        let context = test_context();
        // Even a *legitimate-looking* command is refused if it is not `npm`.
        let err = tool
            .execute(
                serde_json::json!({ "command": "npx", "args": ["evil"] }),
                &context,
            )
            .await
            .expect_err("npm.run must refuse npx");
        assert!(err.to_string().contains("may not perform"), "{err}");
    }

    /// `cargo.test` is pinned to `cargo test`, not to arbitrary cargo.
    #[tokio::test]
    async fn cargo_test_is_pinned_to_cargo_test() {
        let tool = ShellTool::new("cargo.test", "cargo");
        assert_eq!(tool.bound_operation().as_deref(), Some("cargo test"));

        let err = tool
            .execute(
                serde_json::json!({ "command": "cargo", "args": ["publish"] }),
                &test_context(),
            )
            .await;
        // A matching program is permitted; the *program* is what the binding
        // guards. The prefix `test` is always inserted, so the executed command
        // is `cargo test publish`, never `cargo publish`.
        assert!(err.is_ok(), "a matching program should proceed: {err:?}");
    }

    /// Bound tools reject shell metacharacters, because on Windows they run
    /// through `cmd /C` where those characters would allow argument injection.
    #[tokio::test]
    async fn bound_shell_tool_rejects_metacharacter_arguments() {
        let tool = ShellTool::new("npm.run", "npm");
        let err = tool
            .execute(
                serde_json::json!({ "args": ["build", "&&", "rm", "-rf", "/"] }),
                &test_context(),
            )
            .await
            .expect_err("metacharacters must be refused for bound tools");
        assert!(err.to_string().contains("metacharacter"), "{err}");
    }

    /// `shell.execute` is the documented escape hatch: it stays generic.
    #[tokio::test]
    async fn generic_shell_execute_remains_unbound() {
        let tool = ShellTool::new("shell.execute", "shell");
        assert_eq!(tool.bound_operation(), None);
        assert_eq!(tool.risk_level(), RiskLevel::Execute);

        let params = if cfg!(target_os = "windows") {
            serde_json::json!({ "command": "cmd", "args": ["/C", "echo generic"] })
        } else {
            serde_json::json!({ "command": "echo", "args": ["generic"] })
        };
        let result = tool.execute(params, &test_context()).await.unwrap();
        assert!(result.success);
    }

    /// `shell.execute` without a program fails closed rather than guessing.
    #[tokio::test]
    async fn generic_shell_execute_requires_a_command() {
        let tool = ShellTool::new("shell.execute", "shell");
        let err = tool
            .execute(serde_json::json!({ "args": ["whoami"] }), &test_context())
            .await
            .expect_err("shell.execute must require a command");
        assert!(err.to_string().contains("requires a non-empty"), "{err}");
    }

    /// An id nobody recognises must not inherit unbound semantics.
    #[tokio::test]
    async fn unrecognised_shell_id_fails_closed() {
        let tool = ShellTool::new("shell.definitely-not-real", "?");
        let err = tool
            .execute(
                serde_json::json!({ "command": "echo", "args": ["hi"] }),
                &test_context(),
            )
            .await
            .expect_err("an unknown shell id must be refused");
        assert!(err.to_string().contains("unsupported tool"), "{err}");
    }

    #[tokio::test]
    async fn test_git_tool_status() {
        let tool = GitTool::new("git.status", "Git status");
        let context = ToolContext {
            working_directory: PathBuf::from("."),
            environment: HashMap::new(),
            timeout: Duration::from_secs(5),
            session_id: None,
            actor: None,
            approval_required: false,
        };

        let params = serde_json::json!({
            "subcommand": "status",
            "args": ["--short"]
        });

        let result = tool.execute(params, &context).await.unwrap();
        assert!(result.success);
    }

    /// Actor attribution must survive dispatch: the explicit context value
    /// wins, the task-local binding is resolved when the context carries
    /// none, and an unbound call is recorded as the least-privileged case.
    /// All three outcomes must reach the sink — the audit's finding was not
    /// that attribution was impossible but that it was never persisted.
    #[tokio::test]
    async fn dispatch_resolves_and_persists_the_actor() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("evidence.jsonl");
        let runtime = crate::evidence::ToolRuntime::restrictive().with_evidence_at(&path);

        let unbound = ToolContext {
            working_directory: PathBuf::from("."),
            environment: HashMap::new(),
            timeout: Duration::from_secs(5),
            session_id: None,
            actor: None,
            approval_required: false,
        };
        let read_cargo = || serde_json::json!({ "action": "read", "path": "Cargo.toml" });

        // 1. Unbound: recorded with no actor, not fabricated.
        let outcome = dispatch("fs.read", read_cargo(), &unbound, &runtime).await;
        assert!(outcome.result.is_ok());
        assert_eq!(outcome.evidence.actor, None);

        // 2. Explicit context actor wins.
        let explicit = ToolContext {
            actor: Some("user:explicit".to_string()),
            ..context_without_actor()
        };
        let outcome = dispatch("fs.read", read_cargo(), &explicit, &runtime).await;
        assert_eq!(outcome.evidence.actor.as_deref(), Some("user:explicit"));

        // 3. Task-local binding is resolved when the context carries none.
        let bound = crate::actor::with_actor("agent:run-7", async {
            dispatch("fs.read", read_cargo(), &unbound, &runtime).await
        })
        .await;
        assert_eq!(bound.evidence.actor.as_deref(), Some("agent:run-7"));

        // The sink saw all three records, with the right attribution.
        let sink = crate::evidence::JsonlEvidenceSink::new(&path);
        let records = sink.read_all().unwrap();
        assert_eq!(records.len(), 3, "every dispatch outcome must be persisted");
        assert_eq!(records[0].actor, None);
        assert_eq!(records[1].actor.as_deref(), Some("user:explicit"));
        assert_eq!(records[2].actor.as_deref(), Some("agent:run-7"));
    }

    fn context_without_actor() -> ToolContext {
        ToolContext {
            working_directory: PathBuf::from("."),
            environment: HashMap::new(),
            timeout: Duration::from_secs(5),
            session_id: None,
            actor: None,
            approval_required: false,
        }
    }
}
