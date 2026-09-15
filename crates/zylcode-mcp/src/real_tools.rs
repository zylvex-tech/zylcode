//! Real Tool Runtime - Replaces mocked tool execution with actual operations
//!
//! This module provides real filesystem, shell, git, and search operations
//! instead of the mocked implementations in `tool.rs`.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::process::Command;
use tracing::{error, info, warn};

/// Evidence record for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEvidence {
    pub invocation_id: String,
    pub tool_id: String,
    pub session_id: Option<String>,
    pub arguments: Value,
    pub working_directory: PathBuf,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub exit_status: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub changed_files: Vec<PathBuf>,
    pub timeout: Option<Duration>,
    pub approval_decision: Option<String>,
}

/// Real tool execution trait
#[async_trait]
pub trait RealTool: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &str;
    fn description(&self) -> &str;
    fn permissions(&self) -> ToolPermissions;
    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult>;
}

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
    pub timeout: Duration,
    pub session_id: Option<String>,
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

/// Filesystem tool for real file operations
#[derive(Debug)]
pub struct FileSystemTool {
    id: String,
    description: String,
}

impl FileSystemTool {
    pub fn new(id: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
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

    fn permissions(&self) -> ToolPermissions {
        ToolPermissions {
            read: true,
            write: true,
            execute: false,
            network: false,
            destructive: false,
        }
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let start = Instant::now();
        let invocation_id = uuid::Uuid::new_v4().to_string();
        
        let action = params.get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("read");
        
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        
        let full_path = context.working_directory.join(path);
        
        let mut evidence = ToolEvidence {
            invocation_id: invocation_id.clone(),
            tool_id: self.id.clone(),
            session_id: context.session_id.clone(),
            arguments: params.clone(),
            working_directory: context.working_directory.clone(),
            start_time: chrono::Utc::now(),
            end_time: None,
            exit_status: None,
            stdout: None,
            stderr: None,
            changed_files: Vec::new(),
            timeout: Some(context.timeout),
            approval_decision: None,
        };
        
        let result = match action {
            "read" => {
                match fs::read_to_string(&full_path).await {
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
                }
            }
            "write" => {
                let content = params.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
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
            "list" => {
                match fs::read_dir(&full_path).await {
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
                }
            }
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

/// Shell tool for real process execution
#[derive(Debug)]
pub struct ShellTool {
    id: String,
    description: String,
}

impl ShellTool {
    pub fn new(id: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
        }
    }
}

#[async_trait]
impl RealTool for ShellTool {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
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
        let invocation_id = uuid::Uuid::new_v4().to_string();
        
        let command = params.get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let args: Vec<String> = params.get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect())
            .unwrap_or_default();
        
        let mut evidence = ToolEvidence {
            invocation_id: invocation_id.clone(),
            tool_id: self.id.clone(),
            session_id: context.session_id.clone(),
            arguments: params.clone(),
            working_directory: context.working_directory.clone(),
            start_time: chrono::Utc::now(),
            end_time: None,
            exit_status: None,
            stdout: None,
            stderr: None,
            changed_files: Vec::new(),
            timeout: Some(context.timeout),
            approval_decision: None,
        };
        
        // On Windows, built-in commands like 'echo' need to be run via cmd /C
        let mut cmd = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C").arg(command).args(&args);
            cmd
        } else {
            let mut cmd = Command::new(command);
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
                        "command": command,
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
                        "command": command,
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
                        "command": command,
                        "args": args,
                        "error": "Command timed out"
                    }),
                    evidence: evidence.clone(),
                    changed_files: Vec::new(),
                    duration: start.elapsed(),
                }
            }
        }.pipe(|mut result| {
            result.evidence.end_time = Some(chrono::Utc::now());
            Ok(result)
        })
    }
}

/// Git tool for real git operations
#[derive(Debug)]
pub struct GitTool {
    id: String,
    description: String,
}

impl GitTool {
    pub fn new(id: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
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

    fn permissions(&self) -> ToolPermissions {
        ToolPermissions {
            read: true,
            write: true,
            execute: true,
            network: true,
            destructive: false,
        }
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let start = Instant::now();
        let invocation_id = uuid::Uuid::new_v4().to_string();
        
        let subcommand = params.get("subcommand")
            .and_then(|v| v.as_str())
            .unwrap_or("status");
        
        let args: Vec<String> = params.get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect())
            .unwrap_or_default();
        
        let mut evidence = ToolEvidence {
            invocation_id: invocation_id.clone(),
            tool_id: self.id.clone(),
            session_id: context.session_id.clone(),
            arguments: params.clone(),
            working_directory: context.working_directory.clone(),
            start_time: chrono::Utc::now(),
            end_time: None,
            exit_status: None,
            stdout: None,
            stderr: None,
            changed_files: Vec::new(),
            timeout: Some(context.timeout),
            approval_decision: None,
        };
        
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
        }.pipe(|mut result| {
            result.evidence.end_time = Some(chrono::Utc::now());
            Ok(result)
        })
    }
}

/// Search tool for real repository search
#[derive(Debug)]
pub struct SearchTool {
    id: String,
    description: String,
}

impl SearchTool {
    pub fn new(id: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
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
        let invocation_id = uuid::Uuid::new_v4().to_string();
        
        let pattern = params.get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let search_type = params.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("text");
        
        let mut evidence = ToolEvidence {
            invocation_id: invocation_id.clone(),
            tool_id: self.id.clone(),
            session_id: context.session_id.clone(),
            arguments: params.clone(),
            working_directory: context.working_directory.clone(),
            start_time: chrono::Utc::now(),
            end_time: None,
            exit_status: None,
            stdout: None,
            stderr: None,
            changed_files: Vec::new(),
            timeout: Some(context.timeout),
            approval_decision: None,
        };
        
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
        }.pipe(|mut result| {
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
            
            if !filename.is_empty() && (status.contains('M') || status.contains('A') || status.contains('D')) {
                files.push(PathBuf::from(filename));
            }
        }
    }
    
    files
}

/// Get a real tool implementation by ID
pub fn get_real_tool(tool_id: &str) -> Option<Box<dyn RealTool>> {
    match tool_id {
        "fs.read" | "fs.write" | "fs.list" => Some(Box::new(FileSystemTool::new(tool_id, "File System Tool"))),
        "shell.execute" | "npm.run" | "cargo.test" => Some(Box::new(ShellTool::new(tool_id, "Shell Tool"))),
        "git.status" | "git.diff" | "git.commit" => Some(Box::new(GitTool::new(tool_id, "Git Tool"))),
        "search.find" | "search.grep" => Some(Box::new(SearchTool::new(tool_id, "Search Tool"))),
        _ => None,
    }
}

/// Extension trait for pipe operations
trait Pipe<T> {
    fn pipe<F, U>(self, f: F) -> U where F: FnOnce(T) -> U;
}

impl<T> Pipe<T> for T {
    fn pipe<F, U>(self, f: F) -> U where F: FnOnce(T) -> U {
        f(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_filesystem_tool_read() {
        let tool = FileSystemTool::new("fs.read", "Read file contents");
        let context = ToolContext {
            working_directory: PathBuf::from("."),
            environment: HashMap::new(),
            timeout: Duration::from_secs(5),
            session_id: None,
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
            approval_required: false,
        };
        
        // Use a command that works on both Windows and Unix
        let params = if cfg!(target_os = "windows") {
            serde_json::json!({
                "command": "cmd",
                "args": ["/C", "echo hello world"]
            })
        } else {
            serde_json::json!({
                "command": "echo",
                "args": ["hello world"]
            })
        };
        
        let result = tool.execute(params, &context).await.unwrap();
        assert!(result.success);
        let stdout = result.output.get("stdout").unwrap().as_str().unwrap();
        assert!(stdout.contains("hello world"));
    }
    
    #[tokio::test]
    async fn test_git_tool_status() {
        let tool = GitTool::new("git.status", "Git status");
        let context = ToolContext {
            working_directory: PathBuf::from("."),
            environment: HashMap::new(),
            timeout: Duration::from_secs(5),
            session_id: None,
            approval_required: false,
        };
        
        let params = serde_json::json!({
            "subcommand": "status",
            "args": ["--short"]
        });
        
        let result = tool.execute(params, &context).await.unwrap();
        assert!(result.success);
    }
}