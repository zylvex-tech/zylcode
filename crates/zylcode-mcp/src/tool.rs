use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::config::{McpToolConfig, McpTransport};

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
}

impl DynamicTool {
    pub fn new(config: McpToolConfig) -> Self {
        Self { config }
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
        // Foreign execution boundary — structured logging + context preservation.
        // For stdio we would spawn the command; for sse/websocket we would
        // HTTP/WebSocket. In this phase we provide a deterministic local
        // simulation that remains testable without external processes, but the
        // error recovery wrapper in executor.rs treats it as a foreign call.
        match self.config.transport {
            McpTransport::Stdio => {
                // Simulate stdio tool echo — real impl would use tokio::process::Command
                Ok(serde_json::json!({
                    "tool": self.config.id,
                    "transport": "stdio",
                    "echo": params,
                    "command": self.config.command,
                }))
            }
            McpTransport::Sse => Ok(serde_json::json!({
                "tool": self.config.id,
                "transport": "sse",
                "echo": params,
                "endpoint": self.config.command,
            })),
            McpTransport::Websocket => Ok(serde_json::json!({
                "tool": self.config.id,
                "transport": "websocket",
                "echo": params,
                "endpoint": self.config.command,
            })),
        }
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
    #[tokio::test]
    async fn dynamic_tool_echoes_params() {
        let cfg = McpToolConfig {
            id: "demo".into(),
            command: "npx demo".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        let t = DynamicTool::new(cfg);
        let out = t.call(serde_json::json!({"x":1})).await.unwrap();
        assert_eq!(out["tool"], "demo");
        assert_eq!(out["echo"]["x"], 1);
    }
}
