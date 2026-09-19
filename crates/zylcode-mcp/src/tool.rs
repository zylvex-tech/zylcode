use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::config::McpToolConfig;
use crate::real_tools::ToolContext;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub id: String,
    pub transport: String,
    pub command: String,
    pub env_keys: Vec<String>,
    pub description: Option<String>,
}

#[async_trait]
pub trait Tool: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &str;
    fn descriptor(&self) -> ToolDescriptor;
    async fn call(&self, params: Value) -> Result<Value>;
    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DynamicTool {
    config: McpToolConfig,
    /// The policy this tool is subject to and where its evidence goes.
    ///
    /// Defaults to [`crate::evidence::ToolRuntime::restrictive`]: reads proceed,
    /// everything above them requires approval, and every outcome is written to
    /// the JSONL evidence log. A caller that wants different behaviour must say
    /// so with [`DynamicTool::with_runtime`].
    runtime: crate::evidence::ToolRuntime,
}

impl DynamicTool {
    pub fn new(config: McpToolConfig) -> Self {
        Self {
            config,
            runtime: crate::evidence::ToolRuntime::restrictive(),
        }
    }

    /// Use an explicit runtime. The caller is stating its policy and where the
    /// evidence goes.
    pub fn with_runtime(config: McpToolConfig, runtime: crate::evidence::ToolRuntime) -> Self {
        Self { config, runtime }
    }

    pub fn runtime(&self) -> &crate::evidence::ToolRuntime {
        &self.runtime
    }

    pub fn config(&self) -> &McpToolConfig {
        &self.config
    }

    pub fn transport_str(&self) -> String {
        self.config.transport.to_string()
    }
}

#[async_trait]
impl Tool for DynamicTool {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: self.config.id.clone(),
            transport: self.config.transport.to_string(),
            command: self.config.command.clone(),
            env_keys: self.config.env.keys().cloned().collect(),
            description: self.config.description.clone(),
        }
    }

    async fn call(&self, params: Value) -> Result<Value> {
        // Every invocation goes through the gate. There is no path around it:
        // this is the product-reachable dispatch point, and `BuiltinTool::call`
        // (the bridge) routes through the same function.
        //
        // This method previously did two things that both fabricated success:
        //
        //   1. When no real executor existed it returned
        //      `Ok(json!({"status": "unsupported_mock", "echo": params}))` —
        //      a *successful* result carrying an echo of the input. The
        //      `unsupported_mock` label is honest, but `Ok` is not: the caller
        //      and the retry wrapper in executor.rs both treat it as a
        //      completed call.
        //   2. When a real executor *failed* it caught the error and returned
        //      `Ok(json!({"status": "failed"}))`. A failure reported as `Ok` is
        //      the same defect with a different label.
        let context = ToolContext {
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            environment: std::env::vars().collect(),
            timeout: Duration::from_secs(30),
            session_id: None,
            actor: None,
            approval_required: false,
        };

        let outcome =
            crate::real_tools::dispatch(&self.config.id, params, &context, &self.runtime).await;
        let result = outcome.result?;

        Ok(serde_json::json!({
            "tool": self.config.id,
            "executed": true,
            "success": result.success,
            "output": result.output,
            "bound_operation": result.evidence.bound_operation,
            "risk": result.evidence.risk,
            "approval_decision": result.evidence.approval_decision,
            "invocation_id": result.evidence.invocation_id,
            "changed_files": result.changed_files,
        }))
    }
}

// Helper to convert HashMap env into sorted keys for descriptor
#[allow(dead_code)]
pub fn env_keys_sorted(env: &HashMap<String, String>) -> Vec<String> {
    let mut k: Vec<String> = env.keys().cloned().collect();
    k.sort_unstable();
    k
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpTransport;

    /// A configured tool with no real executor fails closed.
    ///
    /// Replaces `dynamic_tool_unsupported_mock`, which asserted the old
    /// behaviour: `Ok(json!({"status": "unsupported_mock"}))`. That returned a
    /// *successful* result for a call that never executed anything.
    #[tokio::test]
    async fn dynamic_tool_without_executor_fails_closed() {
        let cfg = McpToolConfig {
            id: "demo".into(),
            command: "npx demo".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        let t = DynamicTool::new(cfg);
        let err = t
            .call(serde_json::json!({"x": 1}))
            .await
            .expect_err("a tool with no executor must not report success");
        assert!(
            err.to_string().contains("no executor") || err.to_string().contains("definition-only"),
            "expected a typed NotImplemented refusal, got: {err}"
        );
    }

    /// A *failing* executor propagates its error rather than being wrapped in
    /// `Ok`. The old code caught the error and returned
    /// `Ok(json!({"status": "failed"}))`.
    #[tokio::test]
    async fn dynamic_tool_propagates_executor_failure() {
        let cfg = McpToolConfig {
            id: "git.commit".into(),
            command: "commit".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        // A permissive gate, so this test exercises the *binding* rather than
        // the permission gate. The gate's own behaviour is covered below.
        let t = DynamicTool::with_runtime(
            cfg,
            crate::evidence::ToolRuntime::permissive_without_evidence(),
        );
        // No `message`: the bound git.commit executor refuses the request.
        let err = t
            .call(serde_json::json!({}))
            .await
            .expect_err("a refused request must surface as an error, not an Ok");
        assert!(err.to_string().contains("requires a `message`"), "{err}");
    }

    /// The default gate refuses an unapproved write-class tool.
    ///
    /// `DynamicTool::new` uses the restrictive gate. `git.commit` is
    /// `RiskLevel::GitWrite`, so it must not run without approval — and the
    /// refusal must be a permission denial, not a fabricated success.
    #[tokio::test]
    async fn dynamic_tool_default_gate_refuses_unapproved_write() {
        let cfg = McpToolConfig {
            id: "git.commit".into(),
            command: "commit".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        let t = DynamicTool::new(cfg);
        let err = t
            .call(serde_json::json!({ "message": "should not run" }))
            .await
            .expect_err("git.commit must not run unapproved");
        assert!(err.to_string().contains("permission denied"), "{err}");
    }

    /// A read-class tool passes the default gate and returns real content.
    #[tokio::test]
    async fn dynamic_tool_default_gate_permits_reads() {
        let cfg = McpToolConfig {
            id: "fs.read".into(),
            command: "read".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        let t = DynamicTool::new(cfg);
        let out = t
            .call(serde_json::json!({ "path": "Cargo.toml" }))
            .await
            .expect("reads are permitted by the default gate");
        assert_eq!(out["executed"], true);
        assert!(out["approval_decision"]
            .as_str()
            .unwrap_or_default()
            .starts_with("allow: "));
    }

    #[tokio::test]
    async fn dynamic_tool_real_fs_read() {
        let cfg = McpToolConfig {
            id: "fs.read".into(),
            command: "read".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        let t = DynamicTool::new(cfg);
        let out = t
            .call(serde_json::json!({ "path": "Cargo.toml" }))
            .await
            .unwrap();
        // The dispatch envelope nests the executor output under `output`.
        assert!(
            out["output"].get("content").is_some(),
            "expected file content, got: {out}"
        );
    }
}
