pub mod actor;
pub mod audit;
pub mod builtin_plugins;
pub mod builtin_skills;
pub mod config;
pub mod enhanced_bridge;
pub mod enhanced_plugin_marketplace;
pub mod enhanced_skills;
pub mod evidence;
pub mod executor;
pub mod hot_reload;
pub mod permission;
pub mod plugin_marketplace;
pub mod real_tools;
pub mod registry;
pub mod skills_system;
pub mod telemetry;
pub mod tool;
pub mod tool_catalogue;

pub use actor::{current_actor, has_actor, with_actor};
pub use audit::{AuditConfig, AuditEntry, AuditEventType, AuditLogger, AuditSeverity};
pub use config::{McpConfigFile, McpToolConfig, McpTransport};
pub use enhanced_bridge::{EnhancedMcpBridge, ToolCategory, ToolDefinition};
pub use enhanced_plugin_marketplace::{
    EnhancedPluginMarketplace, MarketplaceAnalyticsReport, Payment, PaymentResult, RevenueManager,
};
pub use enhanced_skills::{
    CompositionEngine, EnhancedSkillsSystem, SkillsAnalytics, SkillsAnalyticsReport,
    SkillsMarketplace,
};
pub use evidence::{EvidenceSink, JsonlEvidenceSink, NullEvidenceSink, ToolRuntime};
pub use executor::{execute_with_recovery, ExecuteOptions};
pub use hot_reload::{
    AnalyticsReport, HotReloadConfig, HotReloadManager, ReloadCallback, ToolAnalytics,
};
pub use permission::{PermissionDecision, PermissionGate, PermissionPolicy};
pub use plugin_marketplace::{InstalledPlugin, PluginDefinition, PluginMarketplace, UIComponent};
pub use real_tools::{
    dispatch, get_real_tool, DispatchOutcome, FileSystemTool, GitTool, RealTool, RiskLevel,
    SearchTool, ShellTool, ToolContext, ToolError, ToolEvidence, ToolPermissions, ToolResult,
    ToolSchema,
};
pub use registry::ToolRegistry;
pub use skills_system::{SkillDefinition, SkillsSystem};
pub use telemetry::{attrs, propagation, span_names, Telemetry, TelemetryConfig};
pub use tool::{DynamicTool, Tool, ToolDescriptor};
pub use tool_catalogue::{CapabilityStatus, Catalogue, CatalogueMetrics, EvidenceRung, ToolEntry};

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
