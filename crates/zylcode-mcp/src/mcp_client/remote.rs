//! A discovered MCP tool, adapted to ZylCode's own tool contract.
//!
//! # Why an adapter instead of a second framework
//!
//! A remote tool is still a tool: it must be gated before it runs, its attempt
//! must be recorded whether or not it is allowed, and its result must land in
//! the same evidence trail as `fs.read`. Re-implementing that for MCP would
//! create a second path with subtly different rules — the exact shape of defect
//! this repository keeps finding. So [`RemoteTool`] implements [`RealTool`] and
//! is dispatched through [`crate::real_tools::dispatch_with`], which is the same
//! gate-and-evidence code the built-in tools use.
//!
//! # Risk is derived, not declared
//!
//! MCP annotations are *hints from the server*, and a server may omit them or
//! lie. The mapping therefore fails **closed**: only an explicit
//! `readOnlyHint: true` earns [`RiskLevel::Read`]. An unannotated tool is
//! [`RiskLevel::Write`], which a policy treats as approval-requiring, rather
//! than being treated as harmless because nobody said otherwise.
//!
//! # Namespacing
//!
//! The exposed id is `server__tool`. A remote server offering `fs.read` becomes
//! `someserver__fs.read`, so it can never be mistaken for the built-in executor
//! in a policy rule, an evidence record or a transcript.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

use super::registry::ServerSlot;
use super::session::{RemoteToolDef, ToolCallOutcome};
use crate::real_tools::{
    RealTool, RiskLevel, ToolContext, ToolEvidence, ToolPermissions, ToolResult,
};

/// A tool that lives in an external MCP server.
#[derive(Debug)]
pub struct RemoteTool {
    /// `server__tool`, the id ZylCode exposes.
    id: String,
    /// Server the tool belongs to.
    server_id: String,
    /// Protocol-level definition, kept for schema and annotations.
    def: RemoteToolDef,
    /// Cached description so `description()` can return a `&str`.
    description: String,
    /// The live session owner; the tool does not hold a session itself, because
    /// a restart replaces the session under the same tool.
    slot: Arc<ServerSlot>,
}

impl RemoteTool {
    /// Build the adapter for one discovered tool.
    pub fn new(server_id: &str, def: RemoteToolDef, slot: Arc<ServerSlot>) -> Self {
        let id = def.namespaced_id(server_id);
        let description = def
            .description
            .clone()
            .or_else(|| def.title.clone())
            .unwrap_or_else(|| format!("MCP tool `{}` from server `{server_id}`", def.name));
        Self {
            id,
            server_id: server_id.to_string(),
            def,
            description,
            slot,
        }
    }

    /// The server this tool belongs to.
    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    /// The protocol-level definition, including its argument schema.
    pub fn definition(&self) -> &RemoteToolDef {
        &self.def
    }

    /// Risk derived from server-provided hints, failing closed.
    ///
    /// Order matters: an explicit destructive hint outranks a read-only one,
    /// because a server that sets both is describing something that changes the
    /// world and should be treated that way.
    fn derive_risk(&self) -> RiskLevel {
        let annotations = self.def.annotations.as_ref();
        let destructive = annotations
            .and_then(|a| a.destructive_hint)
            .unwrap_or(false);
        if destructive {
            return RiskLevel::Destructive;
        }
        let read_only = annotations.and_then(|a| a.read_only_hint).unwrap_or(false);
        if read_only {
            return RiskLevel::Read;
        }
        let open_world = annotations.and_then(|a| a.open_world_hint).unwrap_or(false);
        if open_world {
            return RiskLevel::Network;
        }
        RiskLevel::Write
    }

    /// A one-line summary for the evidence record's `stdout` field.
    fn summarize(outcome: &ToolCallOutcome) -> String {
        let text = outcome
            .content
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .filter(|s| !s.is_empty())
            .or_else(|| {
                outcome
                    .structured
                    .as_ref()
                    .map(|s| serde_json::to_string(s).unwrap_or_default())
            })
            .unwrap_or_else(|| "[no textual content]".to_string());
        let status = if outcome.is_error {
            "tool reported an error"
        } else {
            "ok"
        };
        format!("{status}: {text}")
    }
}

#[async_trait]
impl RealTool for RemoteTool {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn permissions(&self) -> ToolPermissions {
        let annotations = self.def.annotations.as_ref();
        let read_only = annotations.and_then(|a| a.read_only_hint).unwrap_or(false);
        ToolPermissions {
            read: read_only,
            write: !read_only,
            // The server executes; this client does not spawn anything, so
            // `execute` stays false and `network` describes what the *server*
            // does on our behalf.
            execute: false,
            network: annotations.and_then(|a| a.open_world_hint).unwrap_or(false),
            destructive: annotations
                .and_then(|a| a.destructive_hint)
                .unwrap_or(false),
        }
    }

    fn risk_level(&self) -> RiskLevel {
        self.derive_risk()
    }

    /// The operation this id is bound to, for audit.
    ///
    /// Unlike built-ins, a remote tool's binding is the protocol address
    /// itself: server plus tool name. It can never be `None`, because there is
    /// no unbound remote executor.
    fn bound_operation(&self) -> Option<String> {
        Some(format!("mcp:{}:{}", self.server_id, self.def.name))
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> Result<ToolResult> {
        let started = Instant::now();
        let mut evidence = ToolEvidence::begin(
            &self.id,
            self.risk_level(),
            self.bound_operation(),
            &params,
            context,
        );

        match self.slot.call_tool(&self.def.name, params).await {
            Ok(outcome) => {
                evidence.end_time = Some(chrono::Utc::now());
                evidence.stdout = Some(Self::summarize(&outcome));
                if outcome.is_error {
                    evidence.exit_status = Some(1);
                } else {
                    evidence.exit_status = Some(0);
                }
                Ok(ToolResult {
                    // A tool-level error is a *successful* protocol exchange:
                    // the call reached the server and the server answered. It is
                    // reported as `success: false` so callers do not treat a
                    // failed tool as a completed task, while the evidence keeps
                    // the distinction between "refused" and "ran and failed".
                    success: !outcome.is_error,
                    output: json!({
                        "content": outcome.content,
                        "structured": outcome.structured,
                        "is_error": outcome.is_error,
                        "server": self.server_id,
                        "tool": self.def.name,
                    }),
                    evidence,
                    changed_files: Vec::new(),
                    duration: started.elapsed(),
                })
            }
            Err(error) => {
                evidence.end_time = Some(chrono::Utc::now());
                evidence.stderr = Some(error.to_string());
                Err(error)
            }
        }
    }
}
