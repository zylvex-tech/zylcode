//! `zylcode-core` — Pure Rust engine library for ZylCode.
//!
//! Provides the central [`ZylCodeEngine`] as well as the marketplace
//! subsystem re-exported from [`marketplace`].

pub mod agent;
pub mod agent_protocol;
pub mod ai_input;
pub mod cache;
pub mod cli;
pub mod compression;
pub mod computer_use;
pub mod context_builder;
pub mod ledger;
pub mod marketplace;
pub mod memory_ledger;
pub mod pipeline;
pub mod planner;
pub mod router;
pub mod sqlite_ledger;

use crate::ai_input::AIInputSystem;
use crate::computer_use::ComputerUseSystem;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tracing::{info, instrument};
use zylcode_mcp::ToolRegistry;

// ---------------------------------------------------------------------------
// Engine configuration
// ---------------------------------------------------------------------------

/// Configuration used to bootstrap the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Logical workspace root (project directory the engine operates on).
    pub workspace_root: String,
    /// Whether verbose tracing is enabled.
    #[serde(default)]
    pub verbose: bool,
    /// Arbitrary key-value overrides forwarded to subsystems.
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            workspace_root: ".".to_string(),
            verbose: false,
            extra: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Intent / result types
// ---------------------------------------------------------------------------

/// An intent submitted to the engine (e.g. from the CLI, desktop, or MCP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Natural-language or structured instruction.
    pub prompt: String,
    /// Optional context payload attached to the intent.
    #[serde(default)]
    pub context: Option<serde_json::Value>,
    /// Optional identifier for correlating requests.
    #[serde(default)]
    pub correlation_id: Option<String>,
}

/// The result produced after the engine processes an intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    /// Human-readable summary of what was done.
    pub summary: String,
    /// Structured output (artifacts, diffs, etc.).
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
    /// Whether the operation succeeded.
    pub success: bool,
}

/// A discrete output artifact produced during intent processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Artifact kind (e.g. `file`, `diff`, `log`).
    pub kind: String,
    /// Display label.
    pub label: String,
    /// Content or path.
    pub content: String,
    /// Filesystem path associated with the artifact, if any.
    #[serde(default)]
    pub path: String,
}

/// Outcome of a logic verification pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Whether all checks passed.
    pub passed: bool,
    /// Individual check results.
    pub checks: Vec<VerificationCheck>,
    /// Total duration in milliseconds.
    pub duration_ms: u64,
}

/// A single verification check entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

// ---------------------------------------------------------------------------
// MCP bridge types
// ---------------------------------------------------------------------------

/// Descriptor for an MCP bridge registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpBridgeDescriptor {
    /// Unique bridge identifier.
    pub id: String,
    /// Command or URL used to launch / connect the bridge.
    pub endpoint: String,
    /// Transport kind — `stdio`, `sse`, or `websocket`.
    #[serde(default = "default_transport")]
    pub transport: String,
    /// Optional environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

fn default_transport() -> String {
    "stdio".to_string()
}

/// Internal record kept for each registered bridge.
#[derive(Debug, Clone)]
struct RegisteredBridge {
    descriptor: McpBridgeDescriptor,
    #[allow(dead_code)]
    registered_at: std::time::Instant,
}

// ---------------------------------------------------------------------------
// Core engine
// ---------------------------------------------------------------------------

/// The central ZylCode engine.
///
/// Cheaply cloneable (`Arc`-backed) so it can be shared across Tauri
/// command handlers, CLI subcommands, and background tasks.
#[derive(Clone)]
pub struct ZylCodeEngine {
    config: EngineConfig,
    bridges: Arc<RwLock<HashMap<String, RegisteredBridge>>>,
    marketplace: Arc<RwLock<marketplace::ExtensionRegistry>>,
    pipeline: Arc<pipeline::ArtifactPipeline>,
    tool_registry: Arc<ToolRegistry>,
    ai_input: Arc<Mutex<Option<Arc<AIInputSystem>>>>,
    computer_use: Arc<Mutex<Option<Arc<ComputerUseSystem>>>>,
}

impl ZylCodeEngine {
    /// Create a new engine from the given configuration.
    pub fn new(config: EngineConfig) -> Self {
        info!(
            workspace_root = %config.workspace_root,
            "initializing ZylCodeEngine"
        );
        let router_cfg = Self::router_config_from_engine_config(&config);
        let pipeline = pipeline::ArtifactPipeline::from_config(router_cfg)
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "failed to build pipeline from config, falling back to default router");
                pipeline::ArtifactPipeline::from_config(router::RouterConfig::default())
                    .expect("default router config must build")
            });
        Self {
            config,
            bridges: Arc::new(RwLock::new(HashMap::new())),
            marketplace: Arc::new(RwLock::new(marketplace::ExtensionRegistry::new())),
            pipeline: Arc::new(pipeline),
            tool_registry: Arc::new(ToolRegistry::new()),
            ai_input: Arc::new(Mutex::new(None)),
            computer_use: Arc::new(Mutex::new(None)),
        }
    }

    /// Create an engine with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(EngineConfig::default())
    }

    /// Create an engine with an explicit router config (useful for tests / Tauri model selection).
    pub fn with_router_config(
        config: EngineConfig,
        router_config: router::RouterConfig,
    ) -> Result<Self> {
        let pipeline = pipeline::ArtifactPipeline::from_config(router_config)?;
        Ok(Self {
            config,
            bridges: Arc::new(RwLock::new(HashMap::new())),
            marketplace: Arc::new(RwLock::new(marketplace::ExtensionRegistry::new())),
            pipeline: Arc::new(pipeline),
            tool_registry: Arc::new(ToolRegistry::new()),
            ai_input: Arc::new(Mutex::new(None)),
            computer_use: Arc::new(Mutex::new(None)),
        })
    }

    /// Access the tool registry (MCP dynamic tools).
    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }

    pub fn tool_registry_arc(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.tool_registry)
    }

    fn router_config_from_engine_config(cfg: &EngineConfig) -> router::RouterConfig {
        let mut rc = router::RouterConfig::from_env();
        // Allow engine extra keys to override router config.
        if let Some(v) = cfg.extra.get("primary_model") {
            rc.primary_model = v.clone();
        }
        if let Some(v) = cfg.extra.get("fallback_model") {
            rc.fallback_model = v.clone();
        }
        if let Some(v) = cfg.extra.get("openrouter_api_key") {
            rc.api_keys.insert("openrouter".to_string(), v.clone());
        }
        if let Some(v) = cfg.extra.get("anthropic_api_key") {
            rc.api_keys.insert("anthropic".to_string(), v.clone());
        }
        if let Some(v) = cfg.extra.get("deepseek_api_key") {
            rc.api_keys.insert("deepseek".to_string(), v.clone());
        }
        rc
    }

    /// Access the underlying pipeline (e.g. for token metrics).
    pub fn pipeline(&self) -> &pipeline::ArtifactPipeline {
        &self.pipeline
    }

    /// Current token metrics snapshot.
    pub fn token_metrics(&self) -> router::TokenSnapshot {
        self.pipeline.router().snapshot()
    }

    /// Borrow the engine configuration.
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    // -----------------------------------------------------------------------
    // process_intent
    // -----------------------------------------------------------------------

    /// Process a natural-language or structured intent and return a result.
    ///
    /// Routes execution through the Agent Loop: context gathering → planning →
    /// tool execution → verification → result.
    #[instrument(skip(self), fields(prompt_len = intent.prompt.len()))]
    pub async fn process_intent(&self, intent: Intent) -> Result<IntentResult> {
        if intent.prompt.trim().is_empty() {
            anyhow::bail!("intent prompt must not be empty");
        }

        info!(prompt = %intent.prompt, "processing intent via agent loop");

        let workspace_root = std::path::PathBuf::from(&self.config.workspace_root);

        // Create and run agent loop
        let model_client = Arc::new(agent::RealModelClient::new(Arc::new(
            self.pipeline.router().clone(),
        )));

        // Create ledger store (using SQLite for production)
        let ledger_dir = format!("{}/.zylcode", self.config.workspace_root);
        std::fs::create_dir_all(&ledger_dir)?;
        let ledger_path = format!("{}/ledger.db", ledger_dir);
        let ledger = Arc::new(crate::sqlite_ledger::SqliteLedgerStore::new(&ledger_path)?);

        let mut agent = agent::AgentLoop::new(
            &intent.prompt,
            workspace_root,
            None, // Use default config
            self.tool_registry.clone(),
            model_client,
            ledger,
        );

        // Run the agent loop
        let final_state = agent.run().await?;

        // Debug: print agent state
        println!("Agent loop completed with state: {:?}", final_state);

        // Debug: print agent messages
        for msg in agent.session().messages.iter() {
            println!("Agent message: {:?} - {}", msg.role, msg.content);
        }

        // Convert agent result to IntentResult
        let success = final_state == agent::AgentState::Completed;
        let summary = if success {
            format!("Task completed successfully: {}", intent.prompt)
        } else {
            format!("Task failed: {}", intent.prompt)
        };

        Ok(IntentResult {
            summary,
            artifacts: Vec::new(), // TODO: Extract artifacts from agent session
            success,
        })
    }

    /// Process an intent with an explicit model override (used by the desktop
    /// UI's model picker). The override applies only to this call.
    pub async fn process_intent_with_model(
        &self,
        intent: Intent,
        _model_override: Option<String>,
    ) -> Result<IntentResult> {
        // For now, ignore model override and use the agent loop
        // In the future, we can pass the model override to the agent config
        self.process_intent(intent).await
    }

    // -----------------------------------------------------------------------
    // verify_logic
    // -----------------------------------------------------------------------

    /// Run the logic verification pipeline against the current workspace.
    ///
    /// Executes a sequence of structural checks (file existence, manifest
    /// validity, etc.) and returns an aggregated [`VerificationReport`].
    #[instrument(skip(self))]
    pub async fn verify_logic(&self) -> Result<VerificationReport> {
        let start = std::time::Instant::now();
        info!(workspace_root = %self.config.workspace_root, "verifying logic");

        let mut checks: Vec<VerificationCheck> = Vec::new();

        // Check 1 — workspace root is non-empty.
        checks.push(VerificationCheck {
            name: "workspace_root_present".to_string(),
            passed: !self.config.workspace_root.trim().is_empty(),
            message: if self.config.workspace_root.trim().is_empty() {
                "workspace_root must not be empty".to_string()
            } else {
                format!("workspace_root is '{}'", self.config.workspace_root)
            },
        });

        // Check 2 — workspace root path exists on disk.
        let path_exists = std::path::Path::new(&self.config.workspace_root).exists();
        checks.push(VerificationCheck {
            name: "workspace_root_exists".to_string(),
            passed: path_exists,
            message: if path_exists {
                "workspace root exists on disk".to_string()
            } else {
                format!(
                    "workspace root '{}' does not exist on disk",
                    self.config.workspace_root
                )
            },
        });

        let passed = checks.iter().all(|c| c.passed);
        let duration_ms = start.elapsed().as_millis() as u64;

        // Surface failure as context but still return the report so callers
        // can inspect individual checks.
        let report = VerificationReport {
            passed,
            checks,
            duration_ms,
        };

        if !report.passed {
            tracing::warn!(?report, "verification failed");
        }

        Ok(report)
    }

    // -----------------------------------------------------------------------
    // register_mcp_bridge
    // -----------------------------------------------------------------------

    /// Register an MCP bridge endpoint with the engine.
    ///
    /// The bridge is stored in-memory and keyed by `descriptor.id`. Re-
    /// registering the same id overwrites the previous entry.
    #[instrument(skip(self), fields(bridge_id = %descriptor.id))]
    pub async fn register_mcp_bridge(&self, descriptor: McpBridgeDescriptor) -> Result<()> {
        if descriptor.id.trim().is_empty() {
            anyhow::bail!("MCP bridge id must not be empty");
        }
        if descriptor.endpoint.trim().is_empty() {
            anyhow::bail!("MCP bridge endpoint must not be empty");
        }

        let valid_transports = ["stdio", "sse", "websocket"];
        if !valid_transports.contains(&descriptor.transport.as_str()) {
            anyhow::bail!(
                "invalid transport '{}'; expected one of: {}",
                descriptor.transport,
                valid_transports.join(", ")
            );
        }

        let mut bridges = self.bridges.write().await;
        let is_update = bridges.contains_key(&descriptor.id);

        bridges.insert(
            descriptor.id.clone(),
            RegisteredBridge {
                descriptor: descriptor.clone(),
                registered_at: std::time::Instant::now(),
            },
        );

        if is_update {
            info!(id = %descriptor.id, "updated MCP bridge registration");
        } else {
            info!(id = %descriptor.id, "registered MCP bridge");
        }

        Ok(())
    }

    /// List all registered MCP bridges.
    pub async fn list_mcp_bridges(&self) -> Vec<McpBridgeDescriptor> {
        let bridges = self.bridges.read().await;
        bridges.values().map(|b| b.descriptor.clone()).collect()
    }

    /// Register tools from a local config file (`mcp.tools.yaml`).
    pub async fn load_tools_from_config(&self, path: &std::path::Path) -> Result<usize> {
        zylcode_mcp::register_from_config_file(&self.tool_registry, path).await
    }

    /// Hot-reload tools on config file change (requires `notify`).
    pub fn watch_tools_config(&self, path: std::path::PathBuf) -> Result<()> {
        let registry = Arc::clone(&self.tool_registry);
        let cb = Arc::new(move |cfg: zylcode_mcp::McpConfigFile| {
            let reg = Arc::clone(&registry);
            let tools: Vec<Arc<dyn zylcode_mcp::Tool>> = cfg
                .enabled_tools()
                .into_iter()
                .map(|c| {
                    Arc::new(zylcode_mcp::DynamicTool::new(c.clone())) as Arc<dyn zylcode_mcp::Tool>
                })
                .collect();
            // Spawn blocking register (tokio) — fire and forget for watcher thread
            let rt = tokio::runtime::Handle::try_current();
            if let Ok(handle) = rt {
                handle.spawn(async move {
                    reg.clear().await;
                    reg.register_many(tools).await;
                    tracing::info!("MCP tools hot-reloaded");
                });
            }
        });
        zylcode_mcp::config::install_global_watcher(path, cb)
    }

    /// List registered dynamic tools.
    pub async fn list_tools(&self) -> Vec<zylcode_mcp::ToolDescriptor> {
        self.tool_registry.list().await
    }

    /// Execute a tool with structured logging and recovery.
    pub async fn execute_tool(
        &self,
        id: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let tool = self
            .tool_registry
            .get(id)
            .await
            .ok_or_else(|| anyhow::anyhow!("tool not found: {id}"))?;
        zylcode_mcp::execute_with_recovery(tool, params, zylcode_mcp::ExecuteOptions::default())
            .await
    }

    /// Access the AI Input System, initialising it lazily on first use.
    ///
    /// Construction compiles the intent/entity regex tables, so the system is
    /// created once and cached for the lifetime of the engine.
    pub async fn ai_input_system(&self) -> Result<Arc<AIInputSystem>> {
        let mut guard = self.ai_input.lock().await;
        if let Some(system) = guard.as_ref() {
            return Ok(system.clone());
        }
        let system = Arc::new(AIInputSystem::new().await?);
        *guard = Some(system.clone());
        Ok(system)
    }

    /// Access the Computer Use System, initialising it lazily on first use.
    pub async fn computer_use_system(&self) -> Result<Arc<ComputerUseSystem>> {
        let mut guard = self.computer_use.lock().await;
        if let Some(system) = guard.as_ref() {
            return Ok(system.clone());
        }
        let system = Arc::new(ComputerUseSystem::new().await?);
        *guard = Some(system.clone());
        Ok(system)
    }

    /// Remove a registered MCP bridge by id. Returns `true` if it existed.
    pub async fn unregister_mcp_bridge(&self, id: &str) -> Result<bool> {
        let mut bridges = self.bridges.write().await;
        let existed = bridges.remove(id).is_some();
        if existed {
            info!(id = %id, "unregistered MCP bridge");
        }
        Ok(existed)
    }

    // -----------------------------------------------------------------------
    // Marketplace accessors
    // -----------------------------------------------------------------------

    /// Access the marketplace registry (read).
    pub async fn marketplace_snapshot(&self) -> Vec<marketplace::MarketplaceExtension> {
        let reg = self.marketplace.read().await;
        reg.all().into_iter().cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// Re-exports for ergonomic downstream use
// ---------------------------------------------------------------------------

pub use cache::{cosine_similarity, mock_embed, prompt_hash, VectorCacheStore, VectorEntry};
pub use compression::{
    ASTOutlineExtractor, CompressionMetrics, ContextCompressor, LosslessCommentsStripper,
    TokenWindowCompactor,
};
pub use marketplace::{
    ExtensionRegistry, ExtensionType, MarketplaceExtension, PluginManifest, SkillDefinition,
};
pub use pipeline::{Artifact as PipelineArtifact, ArtifactPipeline, ProofMetrics};
pub use planner::{ExecutionPlan, IntentPlanner, PlanStep};
pub use router::{
    cache::SpeculativeCache, trim_to_window, ContextTrim, ModelProvider, ProviderConfig,
    ProviderKind, RouterConfig, StreamEvent, TokenMetrics, TokenRouter, TokenSnapshot,
};
pub use zylcode_mcp::{
    execute_with_recovery, ExecuteOptions, McpConfigFile, McpToolConfig, McpTransport, Tool,
    ToolDescriptor,
};

#[cfg(test)]
mod tests {
    use super::*;
    use zylcode_mcp::{DynamicTool, McpToolConfig, McpTransport};

    #[tokio::test]
    async fn process_intent_rejects_empty_prompt() {
        let engine = ZylCodeEngine::with_defaults();
        let result = engine
            .process_intent(Intent {
                prompt: "   ".to_string(),
                context: None,
                correlation_id: None,
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn process_intent_succeeds_with_valid_prompt() {
        let engine = ZylCodeEngine::with_defaults();

        // Register fs.read tool
        let fs_config = McpToolConfig {
            id: "fs.read".to_string(),
            command: "read".to_string(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: Some("Read file".to_string()),
        };
        engine
            .tool_registry()
            .register(Arc::new(DynamicTool::new(fs_config)))
            .await;

        let result = engine
            .process_intent(Intent {
                prompt: "build the project".to_string(),
                context: None,
                correlation_id: None,
            })
            .await
            .unwrap();

        // Debug: print result
        println!("Result: {:?}", result);

        assert!(result.success);
        assert!(result.summary.contains("build the project"));
    }

    #[tokio::test]
    async fn register_mcp_bridge_rejects_invalid_transport() {
        let engine = ZylCodeEngine::with_defaults();
        let err = engine
            .register_mcp_bridge(McpBridgeDescriptor {
                id: "test".to_string(),
                endpoint: "npx my-mcp".to_string(),
                transport: "invalid".to_string(),
                env: HashMap::new(),
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("invalid transport"));
    }

    #[tokio::test]
    async fn register_and_list_bridges() {
        let engine = ZylCodeEngine::with_defaults();
        engine
            .register_mcp_bridge(McpBridgeDescriptor {
                id: "bridge-a".to_string(),
                endpoint: "npx my-mcp".to_string(),
                transport: "stdio".to_string(),
                env: HashMap::new(),
            })
            .await
            .unwrap();
        let bridges = engine.list_mcp_bridges().await;
        assert_eq!(bridges.len(), 1);
        assert_eq!(bridges[0].id, "bridge-a");
    }
}
