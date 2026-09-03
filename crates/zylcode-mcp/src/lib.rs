pub mod audit;
pub mod config;
pub mod executor;
pub mod registry;
pub mod telemetry;
pub mod tool;

pub use audit::{AuditConfig, AuditEntry, AuditEventType, AuditLogger, AuditSeverity};
pub use config::{McpConfigFile, McpToolConfig, McpTransport};
pub use executor::{execute_with_recovery, ExecuteOptions};
pub use registry::ToolRegistry;
pub use telemetry::{Telemetry, TelemetryConfig, attrs, propagation, span_names};
pub use tool::{DynamicTool, Tool, ToolDescriptor};

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

/// Load `mcp.tools.yaml` (or JSON) and register enabled tools into `registry`.
/// Returns number of tools registered.
pub async fn register_from_config_file(registry: &ToolRegistry, path: &Path) -> Result<usize> {
    let cfg = McpConfigFile::from_path(path)?;
    register_from_config(registry, cfg).await
}

pub async fn register_from_config(registry: &ToolRegistry, cfg: McpConfigFile) -> Result<usize> {
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
    for t in cfg.enabled_tools() {
        tools.push(Arc::new(DynamicTool::new(t.clone())) as Arc<dyn Tool>);
    }
    let n = tools.len();
    registry.register_many(tools).await;
    Ok(n)
}

/// Convenience: load from `ZYLCODE_MCP_CONFIG` env or default `mcp.tools.yaml` if present.
pub async fn register_from_default_location(registry: &ToolRegistry) -> usize {
    let path = std::env::var("ZYLCODE_MCP_CONFIG").unwrap_or_else(|_| "mcp.tools.yaml".to_string());
    let p = Path::new(&path);
    if !p.exists() {
        return 0;
    }
    match register_from_config_file(registry, p).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, path = %p.display(), "failed to load MCP config");
            0
        }
    }
}
