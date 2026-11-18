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
pub mod mcp_client;
pub mod nav_tools;
pub mod permission;
pub mod plugin_marketplace;
pub mod real_tools;
pub mod registry;
pub mod skills_system;
pub mod telemetry;
pub mod tool;
pub mod tool_catalogue;
pub mod workspace;

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
pub use nav_tools::{is_nav_tool, payload_id, NavTool};
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
pub use workspace::{current_workspace_root, tool_working_directory, with_workspace_root};

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

/// Register every catalogue-executable built-in that `registry` does not
/// already carry. Returns the ids it added, in catalogue order.
///
/// # Why this exists
///
/// [`crate::registry::ToolRegistry`] starts empty, and it is the only table the
/// agent loop consults (`AgentLoop::execute_tool_call`). A loop built over an
/// empty registry therefore fails at its first tool step with
/// `Tool not found: <id>` even though `get_real_tool` has a real executor for
/// that id all along — the tool-name/dispatch mismatch recorded as **TOOL-01**
/// in `docs/governance/REPOSITORY_INTELLIGENCE_WAVE_2026-09-28.md` §17.1. The
/// desktop shell papers over this by loading `mcp.tools.yaml` at startup; the
/// CLI never did, so `fs.read` — the id the plan template's own example step
/// names — was unreachable from a real turn.
///
/// This is the configuration-independent counterpart to
/// [`register_from_default_location`]:
///
/// * the source of truth is [`Catalogue::executable_ids`], so an id is
///   registered **iff** `get_real_tool(id)` returns an executor — never a
///   definition-only entry;
/// * it does not depend on the process working directory, so it behaves the
///   same from any directory and in any test;
/// * each id becomes a [`DynamicTool`], whose `call` routes through
///   [`crate::real_tools::dispatch`] — the `PermissionGate` and the evidence
///   sink are therefore on the path, not bypassed.
///
/// It is fill-in only: an id already present (from `mcp.tools.yaml`, or from a
/// previous call) is left exactly as it is, so a configured description and a
/// caller-supplied [`DynamicTool::with_runtime`] are never clobbered. Calling
/// it twice adds nothing the second time.
pub async fn register_executable_builtins(registry: &ToolRegistry) -> Vec<String> {
    let catalogue = Catalogue::canonical();

    let mut registered = Vec::new();
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
    for id in catalogue.executable_ids() {
        if registry.get(id).await.is_some() {
            continue;
        }
        let description = catalogue.get(id).map(|e| e.description.clone());
        tools.push(Arc::new(DynamicTool::new(McpToolConfig {
            id: id.to_string(),
            command: "builtin".to_string(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description,
        })));
        registered.push(id.to_string());
    }

    registry.register_many(tools).await;
    registered
}
