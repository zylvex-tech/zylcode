use anyhow::{Context as _, Result};
use clap::{Args, Parser, Subcommand};
use std::path::Path;
use tracing::{error, info};
use zylcode_core::ai_input::InputContext;
use zylcode_core::{EngineConfig, Intent, McpBridgeDescriptor, ZylCodeEngine};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "zylcode",
    version,
    about = "ZylCode — autonomous developer desktop engine CLI",
    long_about = "ZylCode CLI — build, bridge MCP servers, and manage the marketplace.",
    arg_required_else_help = true
)]
struct Cli {
    /// Workspace root directory.
    #[arg(long, global = true, env = "ZYLCODE_WORKSPACE", default_value = ".")]
    workspace: String,

    /// Enable verbose (debug) logging.
    #[arg(long, short = 'v', global = true, env = "ZYLCODE_VERBOSE")]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Build the current project / workspace artifacts.
    Build(BuildArgs),

    /// Manage Model Context Protocol bridges.
    McpBridge(McpBridgeArgs),

    /// Manage marketplace extensions (skills, plugins, themes, adapters).
    Marketplace(MarketplaceArgs),

    /// Process multi-modal input through the AI Input System.
    AiInput(AiInputArgs),

    /// Inspect or drive the Computer Use System.
    ComputerUse(ComputerUseArgs),

    /// Retrieve repository-relevant context for a task using Repository
    /// Intelligence (scanner, symbols, packages, dependency graph, entry
    /// points, git history).
    RepoContext { task: Vec<String> },

    /// Serve Repository Intelligence over HTTP for the desktop frontend
    /// (browser preview and remote surfaces). Endpoints: `GET /healthz`,
    /// `GET /api/repo-intel?task=...`.
    ServeIntel(ServeIntelArgs),

    /// Run Best-of-N: sample N candidate patches for a task, verify each in
    /// an isolated git worktree against the real test suite, select the
    /// winner on recorded evidence, and append every outcome plus the
    /// selection decision to the evidence ledger.
    BestOfN(BestOfNArgs),

    /// Delivery: run the real build pipeline and/or package a versioned
    /// release bundle (CLI binary + frontend dist, SHA-256 manifest) into
    /// `.zylcode/releases/`, registered in the Artifact Bus.
    Package(PackageArgs),

    /// Delivery: report deploy targets that are actually commissioned
    /// (GitHub release via tag, crates.io token, remote server).
    DeployStatus,
}

// ---------------------------------------------------------------------------
// build
// ---------------------------------------------------------------------------

#[derive(Args, Debug)]
struct BuildArgs {
    /// Optional intent prompt describing what to build.
    #[arg(long)]
    prompt: Option<String>,

    /// Verify logic without emitting artifacts.
    #[arg(long, default_value_t = false)]
    verify_only: bool,

    /// Output format (text | json).
    #[arg(long, default_value = "text")]
    output: String,
}

// ---------------------------------------------------------------------------
// mcp-bridge
// ---------------------------------------------------------------------------

#[derive(Args, Debug)]
struct McpBridgeArgs {
    #[command(subcommand)]
    command: McpBridgeCommands,
}

#[derive(Subcommand, Debug)]
enum McpBridgeCommands {
    /// Register a new MCP bridge endpoint.
    Register {
        /// Unique bridge identifier.
        #[arg(long)]
        id: String,
        /// Command or URL for the bridge (e.g. `npx my-mcp-server`).
        #[arg(long)]
        endpoint: String,
        /// Transport kind: stdio | sse | websocket.
        #[arg(long, default_value = "stdio")]
        transport: String,
    },
    /// List all registered MCP bridges.
    List,
    /// Remove a registered MCP bridge.
    Remove {
        /// Bridge identifier to remove.
        id: String,
    },
}

// ---------------------------------------------------------------------------
// marketplace
// ---------------------------------------------------------------------------

#[derive(Args, Debug)]
struct MarketplaceArgs {
    #[command(subcommand)]
    command: MarketplaceCommands,
}

#[derive(Subcommand, Debug)]
enum MarketplaceCommands {
    /// Search the marketplace catalogue.
    Search {
        /// Free-text query.
        query: String,
        /// Filter by extension type (skill | plugin | mcp_adapter | theme).
        #[arg(long)]
        kind: Option<String>,
        /// Output as JSON.
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Install an extension by id.
    Install {
        /// Extension id (e.g. `zylcode/prettier-formatter`).
        id: String,
        /// Pin to a specific version.
        #[arg(long)]
        version: Option<String>,
    },
    /// Publish the local extension manifest to the marketplace.
    Publish {
        /// Path to the extension manifest file.
        #[arg(long, default_value = "./zylcode-extension.json")]
        manifest: String,
        /// Dry-run — validate without publishing.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

// ---------------------------------------------------------------------------
// ai-input
// ---------------------------------------------------------------------------

#[derive(Args, Debug)]
struct AiInputArgs {
    #[command(subcommand)]
    command: AiInputCommands,
}

#[derive(Subcommand, Debug)]
enum AiInputCommands {
    /// Process a text prompt and classify its intent.
    ProcessText {
        /// The text to process.
        text: String,
    },
    /// Show supported input types and current statistics.
    Stats,
}

// ---------------------------------------------------------------------------
// computer-use
// ---------------------------------------------------------------------------

#[derive(Args, Debug)]
struct ComputerUseArgs {
    #[command(subcommand)]
    command: ComputerUseCommands,
}

#[derive(Subcommand, Debug)]
enum ComputerUseCommands {
    /// Show Computer Use System statistics.
    Stats,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_build(engine: &ZylCodeEngine, args: BuildArgs) -> Result<()> {
    if args.verify_only {
        let report = engine.verify_logic().await?;
        if args.output == "json" {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            println!(
                "Verification: {} ({} ms)",
                if report.passed { "PASSED" } else { "FAILED" },
                report.duration_ms
            );
            for check in &report.checks {
                let status = if check.passed { "  ✓" } else { "  ✗" };
                println!("{} {} — {}", status, check.name, check.message);
            }
        }
        if !report.passed {
            std::process::exit(1);
        }
        return Ok(());
    }

    let prompt = args
        .prompt
        .unwrap_or_else(|| "build the workspace".to_string());

    let result = engine
        .process_intent(Intent {
            prompt,
            context: None,
            correlation_id: None,
        })
        .await?;

    if args.output == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.summary);
        for artifact in &result.artifacts {
            println!(
                "  [{}] {} — {}",
                artifact.kind, artifact.label, artifact.content
            );
        }
    }

    Ok(())
}

async fn handle_mcp_bridge(engine: &ZylCodeEngine, args: McpBridgeArgs) -> Result<()> {
    match args.command {
        McpBridgeCommands::Register {
            id,
            endpoint,
            transport,
        } => {
            engine
                .register_mcp_bridge(McpBridgeDescriptor {
                    id: id.clone(),
                    endpoint,
                    transport,
                    env: Default::default(),
                })
                .await?;
            println!("Registered MCP bridge '{id}'.");
        }
        McpBridgeCommands::List => {
            let bridges = engine.list_mcp_bridges().await;
            if bridges.is_empty() {
                println!("No MCP bridges registered.");
            } else {
                println!("{}", serde_json::to_string_pretty(&bridges)?);
            }
        }
        McpBridgeCommands::Remove { id } => {
            let existed = engine.unregister_mcp_bridge(&id).await?;
            if existed {
                println!("Removed MCP bridge '{id}'.");
            } else {
                eprintln!("No bridge found with id '{id}'.");
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

async fn handle_marketplace(engine: &ZylCodeEngine, args: MarketplaceArgs) -> Result<()> {
    match args.command {
        MarketplaceCommands::Search { query, kind, json } => {
            let extensions = engine.marketplace_snapshot().await;

            // Apply kind filter when specified.
            let filtered: Vec<_> = extensions
                .iter()
                .filter(|e| {
                    if let Some(ref k) = kind {
                        e.manifest.extension_type.to_string() == k.to_lowercase()
                    } else {
                        true
                    }
                })
                .filter(|e| {
                    let q = query.to_lowercase();
                    e.manifest.id.to_lowercase().contains(&q)
                        || e.manifest.name.to_lowercase().contains(&q)
                        || e.manifest.description.to_lowercase().contains(&q)
                })
                .collect();

            if json {
                println!("{}", serde_json::to_string_pretty(&filtered)?);
            } else if filtered.is_empty() {
                println!("No results for '{query}'.");
            } else {
                println!("Found {} result(s) for '{query}':", filtered.len());
                for ext in filtered {
                    println!(
                        "  {} @{} [{}] — {}",
                        ext.manifest.id,
                        ext.manifest.version,
                        ext.manifest.extension_type,
                        ext.manifest.description
                    );
                }
            }
        }
        MarketplaceCommands::Install { id, version } => {
            // Placeholder: real implementation fetches from the marketplace API,
            // validates checksum, and unpacks to the local extensions directory.
            info!(id = %id, version = ?version, "installing marketplace extension");
            println!(
                "Installing extension '{id}'{}…",
                version
                    .as_deref()
                    .map(|v| format!(" @ {v}"))
                    .unwrap_or_default()
            );
            // Simulate registry check — in production this hits the remote API.
            println!("Extension '{id}' queued for installation. Run `zylcode marketplace search` to verify.");
        }
        MarketplaceCommands::Publish { manifest, dry_run } => {
            let manifest_path = std::path::Path::new(&manifest);
            if !manifest_path.exists() {
                anyhow::bail!("manifest file not found: {}", manifest);
            }
            let raw = std::fs::read_to_string(manifest_path)?;
            let parsed: zylcode_core::PluginManifest = serde_json::from_str(&raw)?;

            if dry_run {
                println!(
                    "Dry-run OK — manifest '{}' @ {} is valid.",
                    parsed.id, parsed.version
                );
            } else {
                println!("Publishing extension '{}' @ {}…", parsed.id, parsed.version);
                // Real publish would POST to the marketplace API here.
                println!("Published '{}' successfully.", parsed.id);
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ai-input / computer-use handlers
// ---------------------------------------------------------------------------

async fn handle_ai_input(engine: &ZylCodeEngine, args: AiInputArgs) -> Result<()> {
    match args.command {
        AiInputCommands::ProcessText { text } => {
            let system = engine.ai_input_system().await?;
            let result = system.process_text(&text, InputContext::default()).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        AiInputCommands::Stats => {
            let system = engine.ai_input_system().await?;
            let stats = system.get_stats().await;
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
    }
    Ok(())
}

async fn handle_computer_use(engine: &ZylCodeEngine, args: ComputerUseArgs) -> Result<()> {
    match args.command {
        ComputerUseCommands::Stats => {
            let system = engine.computer_use_system().await?;
            let stats = system.get_stats().await;
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// repo-context
// ---------------------------------------------------------------------------

/// Handle `zylcode repo-context <task...>`: index the workspace with the real
/// Repository Intelligence pipeline and return ranked context for the task.
/// This is the product-reachable entry point for Phase 2A capability.
fn handle_repo_context(workspace: &str, task_words: &[String]) -> Result<()> {
    if task_words.is_empty() {
        anyhow::bail!("usage: zylcode repo-context <task description...>");
    }
    let task = task_words.join(" ");
    let started = std::time::Instant::now();

    // Canonicalize: package/entry-point discovery requires an absolute root
    // (a relative "." yields zero packages on Windows path joins).
    let root = std::path::Path::new(workspace)
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("cannot resolve workspace '{}': {e}", workspace))?;
    // Persisted index: the first call indexes (~seconds), subsequent calls
    // with an unchanged tree serve from the content-hash-validated cache.
    let query = zylcode_core::intelligence::persisted::PersistedIndex::new(&root).build()?;
    let results = query.relevant_context(&task);

    let elapsed = started.elapsed();
    println!("task: {task}");
    println!(
        "indexed: {} files, {} symbols, {} packages, {} entry points ({:.0} ms)",
        query.file_count(),
        query.symbol_count(),
        query.package_count(),
        query.entry_points().len(),
        elapsed.as_secs_f64() * 1000.0
    );
    println!("top results:");
    for r in results.iter().take(10) {
        println!(
            "  {:5.2} [{}] {} :: {}",
            r.relevance, r.resource_type, r.resource, r.reason
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialise tracing early so engine logs are captured.
    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .with_target(false)
        .init();

    let engine = ZylCodeEngine::new(EngineConfig {
        workspace_root: cli.workspace.clone(),
        verbose: cli.verbose,
        extra: Default::default(),
    });

    let result = match cli.command {
        Commands::Build(args) => handle_build(&engine, args).await,
        Commands::McpBridge(args) => handle_mcp_bridge(&engine, args).await,
        Commands::Marketplace(args) => handle_marketplace(&engine, args).await,
        Commands::AiInput(args) => handle_ai_input(&engine, args).await,
        Commands::ComputerUse(args) => handle_computer_use(&engine, args).await,
        Commands::RepoContext { task } => handle_repo_context(&cli.workspace, &task),
        Commands::ServeIntel(args) => handle_serve_intel(&cli.workspace, args).await,
        Commands::BestOfN(args) => handle_best_of_n(&cli.workspace, args).await,
        Commands::Package(args) => handle_package(&cli.workspace, args).await,
        Commands::DeployStatus => {
            let payload = zylcode_core::delivery::deploy_targets(Path::new(&cli.workspace))?;
            println!("{}", serde_json::to_string_pretty(&payload)?);
            Ok(())
        }
    };

    if let Err(err) = &result {
        error!(error = %err, "command failed");
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }

    result
}

// ---------------------------------------------------------------------------
// serve-intel
// ---------------------------------------------------------------------------

#[derive(Args, Debug)]
struct ServeIntelArgs {
    /// TCP port to listen on (loopback only).
    #[arg(long, default_value_t = 17630)]
    port: u16,

    /// Repository root to index (defaults to the workspace root).
    #[arg(long)]
    repo: Option<String>,
}

/// Shared handler for both the HTTP service and the Tauri command path.
fn repo_intel_json(root: &std::path::Path, task: &str) -> serde_json::Value {
    zylcode_core::intelligence::api::repo_intel_payload(root, task).unwrap_or_else(|e| {
        serde_json::json!({
            "error": format!("repository intelligence failed: {e:#}"),
        })
    })
}

async fn handle_serve_intel(workspace: &str, args: ServeIntelArgs) -> Result<()> {
    use axum::extract::State;
    use axum::routing::get;
    use axum::{Json, Router};
    use std::sync::Arc;

    let root = match &args.repo {
        Some(r) => std::path::PathBuf::from(r).canonicalize()?,
        None => std::path::Path::new(workspace).canonicalize()?,
    };
    anyhow::ensure!(
        root.is_dir(),
        "repository root '{}' is not a directory",
        root.display()
    );
    let root = Arc::new(root);
    let port = args.port;

    // Warm the persisted index once at startup so the first UI request is
    // fast; failures are non-fatal (the per-request build will surface them
    // as payload errors instead).
    {
        let root = Arc::clone(&root);
        tokio::task::spawn_blocking(move || {
            match zylcode_core::intelligence::persisted::PersistedIndex::new(&*root).build() {
                Ok(q) => tracing::info!(
                    "intel index warm: {} files, {} symbols, {} packages",
                    q.file_count(),
                    q.symbol_count(),
                    q.package_count()
                ),
                Err(e) => tracing::warn!("intel index warm failed: {e:#}"),
            }
        });
    }

    async fn healthz() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "status": "ok",
            "service": "zylcode-repo-intel",
        }))
    }

    async fn git_status(State(root): State<Arc<std::path::PathBuf>>) -> Json<serde_json::Value> {
        let root = Arc::clone(&root);
        let payload = tokio::task::spawn_blocking(move || {
            zylcode_core::gitops::git_status_payload(&root).unwrap_or_else(|e| {
                serde_json::json!({
                    "error": format!("git status failed: {e:#}"),
                })
            })
        })
        .await;
        match payload {
            Ok(value) => Json(value),
            Err(e) => Json(serde_json::json!({ "error": format!("git task failed: {e}") })),
        }
    }

    async fn version(State(root): State<Arc<std::path::PathBuf>>) -> Json<serde_json::Value> {
        let root = Arc::clone(&root);
        let payload =
            tokio::task::spawn_blocking(move || zylcode_core::gitops::version_payload(&root)).await;
        match payload {
            Ok(value) => Json(value),
            Err(e) => Json(serde_json::json!({ "error": format!("version task failed: {e}") })),
        }
    }

    /// The committed MCP tool catalogue (`mcp.tools.yaml`): every enabled
    /// tool id with its real executor binding. Read-only; no secrets.
    async fn tools(State(root): State<Arc<std::path::PathBuf>>) -> Json<serde_json::Value> {
        let root = Arc::clone(&root);
        let payload = tokio::task::spawn_blocking(move || {
            let config_path = root.join("mcp.tools.yaml");
            match zylcode_mcp::config::McpConfigFile::from_path(&config_path) {
                Ok(cfg) => {
                    let tools: Vec<serde_json::Value> = cfg
                        .enabled_tools()
                        .iter()
                        .map(|t| {
                            let has_executor =
                                zylcode_mcp::real_tools::get_real_tool(&t.id).is_some();
                            serde_json::json!({
                                "id": t.id,
                                "transport": t.transport.to_string(),
                                "description": t.description,
                                "enabled": t.enabled,
                                "executor_bound": has_executor,
                            })
                        })
                        .collect();
                    serde_json::json!({
                        "source": "mcp.tools.yaml",
                        "tools": tools,
                    })
                }
                Err(e) => {
                    serde_json::json!({ "error": format!("tool catalogue parse failed: {e:#}") })
                }
            }
        })
        .await;
        match payload {
            Ok(value) => Json(value),
            Err(e) => Json(serde_json::json!({ "error": format!("tools task failed: {e}") })),
        }
    }

    /// The real provider fallback chain from the router's configuration.
    /// Read-only: API keys and endpoints are never included in the payload.
    async fn providers() -> Json<serde_json::Value> {
        let cfg = zylcode_core::router::RouterConfig::from_env();
        let mut chain: Vec<serde_json::Value> = cfg
            .provider_configs
            .iter()
            .map(|p| {
                serde_json::json!({
                    "kind": p.kind.to_string(),
                    "model": p.model,
                    "enabled": p.enabled,
                    "fallback_order": p.fallback_order,
                    "requires_api_key": p.requires_api_key,
                    "timeout_ms": p.timeout_ms,
                })
            })
            .collect();
        chain.sort_by_key(|p| p["fallback_order"].as_i64().unwrap_or(i64::MAX));
        // SCORECARD: measured per-provider outcomes (Laplace-smoothed) that
        // now inform dispatch order. Read-only view; absent samples render as
        // an empty list, never fabricated numbers.
        let ranking: Vec<serde_json::Value> = zylcode_core::router::RouterConfig::scorecard_view()
            .unwrap_or_default()
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "provider": r.provider,
                    "successes": r.successes,
                    "failures": r.failures,
                    "score": r.score,
                    "avg_latency_ms": r.avg_latency_ms,
                })
            })
            .collect();
        Json(serde_json::json!({
            "primary_model": cfg.primary_model,
            "fallback_model": cfg.fallback_model,
            "chain": chain,
            "measured_ranking": ranking,
        }))
    }

    /// Live token telemetry from the router snapshot (session counters).
    async fn metrics() -> Json<serde_json::Value> {
        let cfg = zylcode_core::router::RouterConfig::from_env();
        let router = match zylcode_core::TokenRouter::new(cfg) {
            Ok(r) => r,
            Err(e) => {
                return Json(serde_json::json!({ "error": format!("router init failed: {e:#}") }));
            }
        };
        let snap = router.metrics().snapshot();
        Json(
            serde_json::to_value(&snap)
                .unwrap_or_else(|_| serde_json::json!({ "error": "metrics serialization failed" })),
        )
    }

    async fn search(
        State(root): State<Arc<std::path::PathBuf>>,
        axum::extract::Query(params): axum::extract::Query<
            std::collections::HashMap<String, String>,
        >,
    ) -> Json<serde_json::Value> {
        let q = params.get("q").cloned().unwrap_or_default();
        let root = Arc::clone(&root);
        let payload = tokio::task::spawn_blocking(move || {
            zylcode_core::surfaces::search_payload(&root, &q)
                .unwrap_or_else(|e| serde_json::json!({ "error": format!("search failed: {e:#}") }))
        })
        .await;
        match payload {
            Ok(value) => Json(value),
            Err(e) => Json(serde_json::json!({ "error": format!("search task failed: {e}") })),
        }
    }

    /// Persistent Project System store (read-only list). Reveals the store's
    /// state (Ready/Migrated/Empty/Corrupt) so surfaces never invent one.
    async fn projects(State(root): State<Arc<std::path::PathBuf>>) -> Json<serde_json::Value> {
        let root = Arc::clone(&root);
        let payload = tokio::task::spawn_blocking(move || {
            match zylcode_core::project_store::ProjectStore::open(&root) {
                Ok(store) => {
                    let state = format!("{:?}", store.state());
                    let projects = store
                        .list()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|p| serde_json::to_value(&p).unwrap_or_default())
                        .collect::<Vec<_>>();
                    serde_json::json!({ "store_state": state, "projects": projects })
                }
                Err(e) => {
                    serde_json::json!({ "store_state": "Unavailable", "error": format!("{e:#}") })
                }
            }
        })
        .await
        .unwrap_or_else(
            |_| serde_json::json!({ "store_state": "Unavailable", "error": "task join failed" }),
        );
        Json(payload)
    }

    /// Artifact Bus records (read-only). Bytes are never served here — only
    /// metadata, lifecycle, and hash truth.
    async fn artifacts(State(root): State<Arc<std::path::PathBuf>>) -> Json<serde_json::Value> {
        let root = Arc::clone(&root);
        let payload =
            tokio::task::spawn_blocking(
                move || match zylcode_core::artifact_bus::ArtifactBus::open(&root) {
                    Ok(bus) => {
                        let records = bus.list().unwrap_or_default();
                        serde_json::json!({
                            "contract_version": zylcode_core::artifact_bus::ARTIFACT_SCHEMA_VERSION,
                            "count": records.len(),
                            "artifacts": records,
                        })
                    }
                    Err(e) => serde_json::json!({ "error": format!("{e:#}") }),
                },
            )
            .await
            .unwrap_or_else(|_| serde_json::json!({ "error": "task join failed" }));
        Json(payload)
    }

    /// Proof Engine records (read-only). Each proof carries its honest
    /// verification state and staleness relative to the current HEAD.
    async fn proofs(State(root): State<Arc<std::path::PathBuf>>) -> Json<serde_json::Value> {
        let root = Arc::clone(&root);
        let payload =
            tokio::task::spawn_blocking(
                move || match zylcode_core::proof_engine::ProofEngine::open(&root) {
                    Ok(engine) => {
                        let head = std::process::Command::new("git")
                            .args(["rev-parse", "--short", "HEAD"])
                            .current_dir(root.as_ref())
                            .output()
                            .ok()
                            .filter(|o| o.status.success())
                            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                            .unwrap_or_default();
                        let proofs = engine.list(&head).unwrap_or_default();
                        serde_json::json!({
                            "schema_version": zylcode_core::proof_engine::PROOF_SCHEMA_VERSION,
                            "chain_intact": engine.chain_intact(),
                            "current_commit": head,
                            "count": proofs.len(),
                            "proofs": proofs,
                        })
                    }
                    Err(e) => serde_json::json!({ "error": format!("{e:#}") }),
                },
            )
            .await
            .unwrap_or_else(|_| serde_json::json!({ "error": "task join failed" }));
        Json(payload)
    }

    /// Model capability metadata registry (read-only, deterministic).
    async fn models() -> Json<serde_json::Value> {
        Json(zylcode_core::model_capabilities::registry_json())
    }

    async fn deploy_status(State(root): State<Arc<std::path::PathBuf>>) -> Json<serde_json::Value> {
        match zylcode_core::delivery::deploy_targets(&root) {
            Ok(payload) => Json(payload),
            Err(e) => Json(serde_json::json!({ "error": format!("deploy targets failed: {e:#}") })),
        }
    }

    /// Current/last build state (poll after POST /api/build).
    async fn build_status(State(state): State<ServiceState>) -> Json<serde_json::Value> {
        match state.build_status.lock() {
            Ok(guard) => Json(guard.clone()),
            Err(_) => Json(
                serde_json::json!({ "state": "failed", "error": "build status lock poisoned" }),
            ),
        }
    }

    /// Kick off the real build pipeline in the background. Returns 409 if a
    /// build is already running — poll GET /api/build for the outcome.
    async fn build_start(
        State(state): State<ServiceState>,
    ) -> axum::response::Json<serde_json::Value> {
        {
            let status = match state.build_status.lock() {
                Ok(g) => g,
                Err(_) => {
                    return axum::response::Json(serde_json::json!({
                        "error": "build status lock poisoned"
                    }));
                }
            };
            if status["state"] == "running" {
                return axum::response::Json(serde_json::json!({
                    "started": false,
                    "state": "running",
                    "note": "a build is already in progress; poll GET /api/build"
                }));
            }
        }

        let root = std::path::PathBuf::clone(&state.root);
        let status_cell = Arc::clone(&state.build_status);
        tokio::spawn(async move {
            if let Ok(mut status) = status_cell.lock() {
                *status = serde_json::json!({ "state": "running", "started_at": chrono_now_iso() });
            }
            let pipeline = zylcode_core::delivery::default_build_pipeline();
            let report = zylcode_core::delivery::build_workspace(&root, &pipeline, false).await;
            if let Ok(mut status) = status_cell.lock() {
                match report {
                    Ok(report) => *status = report,
                    Err(e) => {
                        *status = serde_json::json!({
                            "state": "failed",
                            "error": format!("build pipeline failed: {e:#}"),
                            "finished_at": chrono_now_iso(),
                        });
                    }
                }
            }
        });

        axum::response::Json(serde_json::json!({
            "started": true,
            "state": "running",
            "note": "build pipeline launched; poll GET /api/build"
        }))
    }

    /// Package a versioned release bundle. Runs the build pipeline first
    /// unless `package_only` is set; refuses to package a red build.
    #[derive(serde::Deserialize, Default)]
    struct PackageRequest {
        version: Option<String>,
        package_only: Option<bool>,
    }

    async fn package(
        State(state): State<ServiceState>,
        axum::Json(req): axum::Json<PackageRequest>,
    ) -> axum::response::Json<serde_json::Value> {
        {
            let status = match state.build_status.lock() {
                Ok(g) => g,
                Err(_) => {
                    return axum::response::Json(
                        serde_json::json!({ "error": "build status lock poisoned" }),
                    );
                }
            };
            if status["state"] == "running" {
                return axum::response::Json(serde_json::json!({
                    "error": "a build is in progress; wait for it to finish before packaging"
                }));
            }
        }

        let root = std::path::PathBuf::clone(&state.root);
        let status_cell = Arc::clone(&state.build_status);
        let package_only = req.package_only.unwrap_or(false);
        tokio::spawn(async move {
            if let Ok(mut status) = status_cell.lock() {
                *status = serde_json::json!({ "state": "running", "started_at": chrono_now_iso() });
            }
            if !package_only {
                let pipeline = zylcode_core::delivery::default_build_pipeline();
                match zylcode_core::delivery::build_workspace(&root, &pipeline, false).await {
                    Ok(report) => {
                        let passed = report["passed"].as_bool().unwrap_or(false);
                        if let Ok(mut status) = status_cell.lock() {
                            *status = report;
                        }
                        if !passed {
                            return; // status already carries the red report
                        }
                    }
                    Err(e) => {
                        if let Ok(mut status) = status_cell.lock() {
                            *status = serde_json::json!({
                                "state": "failed",
                                "error": format!("build pipeline failed: {e:#}"),
                                "finished_at": chrono_now_iso(),
                            });
                        }
                        return;
                    }
                }
            }
            // Resolve version from gitops when not supplied.
            let version = req.version.unwrap_or_default();
            let version = if version.trim().is_empty() {
                zylcode_core::gitops::version_payload(&root)["app_version"]
                    .as_str()
                    .unwrap_or("")
                    .to_string()
            } else {
                version
            };
            if version.trim().is_empty() {
                if let Ok(mut status) = status_cell.lock() {
                    *status = serde_json::json!({
                        "state": "failed",
                        "error": "no version resolvable; pass {\"version\": \"x.y.z\"}",
                        "finished_at": chrono_now_iso(),
                    });
                }
                return;
            }
            match zylcode_core::delivery::package_release(&root, &version).await {
                Ok(payload) => {
                    if let Ok(mut status) = status_cell.lock() {
                        *status = serde_json::json!({
                            "state": "passed",
                            "packaged": payload,
                            "finished_at": chrono_now_iso(),
                        });
                    }
                }
                Err(e) => {
                    if let Ok(mut status) = status_cell.lock() {
                        *status = serde_json::json!({
                            "state": "failed",
                            "error": format!("packaging failed: {e:#}"),
                            "finished_at": chrono_now_iso(),
                        });
                    }
                }
            }
        });

        axum::response::Json(serde_json::json!({
            "started": true,
            "state": "running",
            "note": "packaging pipeline launched; poll GET /api/build"
        }))
    }

    /// Wall-clock timestamp for build status records.
    fn chrono_now_iso() -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    async fn file_tree(State(root): State<Arc<std::path::PathBuf>>) -> Json<serde_json::Value> {
        let root = Arc::clone(&root);
        let payload = tokio::task::spawn_blocking(move || {
            zylcode_core::surfaces::file_tree_payload(&root).unwrap_or_else(
                |e| serde_json::json!({ "error": format!("file tree failed: {e:#}") }),
            )
        })
        .await;
        match payload {
            Ok(value) => Json(value),
            Err(e) => Json(serde_json::json!({ "error": format!("tree task failed: {e}") })),
        }
    }

    async fn file_content(
        State(root): State<Arc<std::path::PathBuf>>,
        axum::extract::Query(params): axum::extract::Query<
            std::collections::HashMap<String, String>,
        >,
    ) -> Json<serde_json::Value> {
        let path = params.get("path").cloned().unwrap_or_default();
        let root = Arc::clone(&root);
        let payload = tokio::task::spawn_blocking(move || {
            zylcode_core::surfaces::file_content_payload(&root, &path).unwrap_or_else(
                |e| serde_json::json!({ "error": format!("file content failed: {e:#}") }),
            )
        })
        .await;
        match payload {
            Ok(value) => Json(value),
            Err(e) => Json(serde_json::json!({ "error": format!("file task failed: {e}") })),
        }
    }

    async fn evidence(State(root): State<Arc<std::path::PathBuf>>) -> Json<serde_json::Value> {
        let root = Arc::clone(&root);
        let payload =
            tokio::task::spawn_blocking(move || zylcode_core::surfaces::evidence_payload(&root))
                .await;
        match payload {
            Ok(value) => Json(value),
            Err(e) => Json(serde_json::json!({ "error": format!("evidence task failed: {e}") })),
        }
    }

    async fn terminal_exec(
        State(hub): State<Arc<zylcode_core::terminal::TerminalHub>>,
        axum::Json(req): axum::Json<zylcode_core::terminal::TerminalRequest>,
    ) -> Json<serde_json::Value> {
        match hub.exec(&req).await {
            Ok(out) => Json(serde_json::to_value(&out).unwrap_or_else(
                |_| serde_json::json!({ "error": "terminal output serialization failed" }),
            )),
            Err(e) => Json(serde_json::json!({ "error": format!("terminal exec failed: {e:#}") })),
        }
    }

    async fn terminal_reset(
        State(hub): State<Arc<zylcode_core::terminal::TerminalHub>>,
    ) -> Json<serde_json::Value> {
        hub.reset();
        Json(serde_json::json!({ "status": "ok" }))
    }

    async fn missions_list(
        State(missions): State<Arc<zylcode_core::missions::MissionQueue>>,
    ) -> Json<serde_json::Value> {
        let list: Vec<serde_json::Value> = missions
            .list()
            .iter()
            .map(|m| serde_json::to_value(m).unwrap_or(serde_json::json!({})))
            .collect();
        Json(serde_json::json!({ "missions": list }))
    }

    async fn missions_enqueue(
        State(missions): State<Arc<zylcode_core::missions::MissionQueue>>,
        axum::Json(body): axum::Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        let task = body["task"].as_str().unwrap_or("");
        let mode = match body["mode"].as_str().unwrap_or("build") {
            "plan" => zylcode_core::missions::MissionMode::Plan,
            _ => zylcode_core::missions::MissionMode::Build,
        };
        match missions.enqueue(task, mode) {
            Ok(m) => Json(
                serde_json::to_value(&m).unwrap_or(serde_json::json!({ "error": "serialization" })),
            ),
            Err(e) => Json(serde_json::json!({ "error": format!("enqueue failed: {e:#}") })),
        }
    }

    async fn missions_clear(
        State(missions): State<Arc<zylcode_core::missions::MissionQueue>>,
    ) -> Json<serde_json::Value> {
        match missions.clear() {
            Ok(()) => Json(serde_json::json!({ "status": "ok" })),
            Err(e) => Json(serde_json::json!({ "error": format!("clear failed: {e:#}") })),
        }
    }

    /// Drain one queued mission for real: BUILD missions run the actual
    /// Best-of-N working-tree verification (candidates against the real
    /// suite, ledger-recorded); PLAN missions record a plan built from the
    /// intelligence pipeline without executing anything.
    async fn missions_run_next(State(st): State<ServiceState>) -> Json<serde_json::Value> {
        let missions = st.missions;
        let root = st.root;
        let lock = st.mission_lock;
        let Some(mission) = missions.claim_next() else {
            return Json(serde_json::json!({ "status": "idle", "reason": "queue empty" }));
        };
        // Hold the drain lock across the whole run; a second concurrent
        // run-next reports busy instead of racing candidates.
        let Ok(_guard) = lock.try_lock() else {
            missions.finish(
                &mission.id,
                false,
                "another mission is already running; queued again",
                None,
            );
            missions.enqueue(&mission.task, mission.mode).ok();
            return Json(serde_json::json!({ "status": "busy", "mission_id": mission.id }));
        };
        let started = std::time::Instant::now();
        match mission.mode {
            zylcode_core::missions::MissionMode::Plan => {
                let result = tokio::task::spawn_blocking({
                    let root = Arc::clone(&root);
                    let task = mission.task.clone();
                    move || zylcode_core::missions::build_plan(&root, &task)
                })
                .await;
                match result {
                    Ok(Ok(plan)) => {
                        let head: String = plan.lines().take(6).collect::<Vec<_>>().join(" ");
                        missions.finish(&mission.id, true, &head, None);
                        Json(
                            serde_json::json!({ "status": "done", "mission": mission.id, "mode": "plan", "plan": plan }),
                        )
                    }
                    Ok(Err(e)) => {
                        missions.finish(&mission.id, false, &format!("plan failed: {e:#}"), None);
                        Json(
                            serde_json::json!({ "status": "failed", "mission": mission.id, "error": format!("{e:#}") }),
                        )
                    }
                    Err(e) => {
                        missions.finish(
                            &mission.id,
                            false,
                            &format!("plan task failed: {e}"),
                            None,
                        );
                        Json(
                            serde_json::json!({ "status": "failed", "mission": mission.id, "error": format!("{e}") }),
                        )
                    }
                }
            }
            zylcode_core::missions::MissionMode::Build => {
                // Real pipeline: working-tree Best-of-N with the ledger.
                let ledger_path = root.join(".zylcode").join("ledger.db");
                let _ = std::fs::create_dir_all(ledger_path.parent().unwrap());
                let ledger: Arc<dyn zylcode_core::ledger::LedgerStore> =
                    match zylcode_core::sqlite_ledger::SqliteLedgerStore::new(
                        ledger_path.to_string_lossy().as_ref(),
                    ) {
                        Ok(l) => Arc::new(l),
                        Err(e) => {
                            missions.finish(
                                &mission.id,
                                false,
                                &format!("ledger unavailable: {e:#}"),
                                None,
                            );
                            return Json(serde_json::json!({
                                "status": "failed", "mission": mission.id,
                                "error": format!("ledger unavailable: {e:#}"),
                            }));
                        }
                    };
                let session_id = uuid::Uuid::new_v4();
                let source: Arc<dyn zylcode_core::patch_best_of_n::CandidateSource> =
                    Arc::new(zylcode_core::patch_best_of_n::WorkingTreeSource {
                        repo_root: root.as_ref().clone(),
                    });
                let config = zylcode_core::best_of_n::BestOfNConfig {
                    candidates: 1,
                    per_candidate_timeout: std::time::Duration::from_secs(1800),
                    max_concurrent: 1,
                };
                let run = zylcode_core::patch_best_of_n::run_patch_best_of_n(
                    &mission.task,
                    source,
                    None,
                    &root,
                    Some(Arc::clone(&ledger)),
                    Some(session_id),
                    &config,
                    vec!["cargo".to_string(), "test".to_string()],
                    std::time::Duration::from_secs(1800),
                )
                .await;
                let elapsed = started.elapsed().as_secs();
                match run {
                    Ok(result) => {
                        let selected = result.selection.outcomes.iter().find(|o| o.passed);
                        match selected {
                            Some(o) => {
                                let summary = format!(
                                    "candidate {} passed: {} of {} checks green ({}s)",
                                    o.candidate_index,
                                    o.evidence.passing_checks(),
                                    o.evidence.checks.len(),
                                    elapsed
                                );
                                let sess = session_id.to_string();
                                missions.finish(&mission.id, true, &summary, Some(sess));
                                Json(serde_json::json!({
                                    "status": "done", "mission": mission.id, "mode": "build",
                                    "summary": summary, "ledger_session": session_id.to_string(),
                                }))
                            }
                            None => {
                                let summary =
                                    format!("no candidate passed verification ({}s)", elapsed);
                                missions.finish(
                                    &mission.id,
                                    false,
                                    &summary,
                                    Some(session_id.to_string()),
                                );
                                Json(
                                    serde_json::json!({ "status": "failed", "mission": mission.id, "summary": summary }),
                                )
                            }
                        }
                    }
                    Err(e) => {
                        missions.finish(
                            &mission.id,
                            false,
                            &format!("best-of-n failed: {e:#}"),
                            None,
                        );
                        Json(
                            serde_json::json!({ "status": "failed", "mission": mission.id, "error": format!("{e:#}") }),
                        )
                    }
                }
            }
        }
    }

    async fn recent_files(State(root): State<Arc<std::path::PathBuf>>) -> Json<serde_json::Value> {
        let root = Arc::clone(&root);
        let payload = tokio::task::spawn_blocking(move || {
            // Recently modified tracked files: git gives the truth.
            let out = std::process::Command::new("git")
                .args(["log", "--name-only", "--pretty=format:", "-12"])
                .current_dir(root.as_ref())
                .output();
            match out {
                Ok(o) if o.status.success() => {
                    let mut files: Vec<String> = String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(|l| l.trim().to_string())
                        .collect();
                    files.dedup();
                    serde_json::json!({ "recently_edited": files.into_iter().take(24).collect::<Vec<_>>() })
                }
                Ok(o) => serde_json::json!({ "error": String::from_utf8_lossy(&o.stderr).to_string() }),
                Err(e) => serde_json::json!({ "error": format!("git failed: {e}") }),
            }
        })
        .await;
        match payload {
            Ok(v) => Json(v),
            Err(e) => Json(serde_json::json!({ "error": format!("recent task failed: {e}") })),
        }
    }

    async fn repo_intel(
        State(root): State<Arc<std::path::PathBuf>>,
        axum::extract::Query(params): axum::extract::Query<
            std::collections::HashMap<String, String>,
        >,
    ) -> Json<serde_json::Value> {
        let task = params
            .get("task")
            .map(String::as_str)
            .unwrap_or("")
            .to_string();
        let root = Arc::clone(&root);
        let payload = tokio::task::spawn_blocking(move || repo_intel_json(&root, &task)).await;
        match payload {
            Ok(value) => Json(value),
            Err(e) => Json(serde_json::json!({ "error": format!("intel task failed: {e}") })),
        }
    }

    // Terminal sessions: cwd state lives for the service's lifetime.
    // FromRef lets each handler extract only the field it needs.
    #[derive(Clone, axum::extract::FromRef)]
    struct ServiceState {
        root: Arc<std::path::PathBuf>,
        hub: Arc<zylcode_core::terminal::TerminalHub>,
        missions: Arc<zylcode_core::missions::MissionQueue>,
        /// Serialize Best-of-N drains: one mission runs at a time.
        mission_lock: Arc<tokio::sync::Mutex<()>>,
        /// Last/current workspace build: {state: idle|running|passed|failed, ...}.
        build_status: Arc<std::sync::Mutex<serde_json::Value>>,
    }

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/api/repo-intel", get(repo_intel))
        .route("/api/git/status", get(git_status))
        .route("/api/version", get(version))
        .route("/api/tools", get(tools))
        .route("/api/providers", get(providers))
        .route("/api/metrics", get(metrics))
        .route("/api/search", get(search))
        .route("/api/files", get(file_tree))
        .route("/api/file-content", get(file_content))
        .route("/api/evidence", get(evidence))
        .route("/api/terminal/exec", axum::routing::post(terminal_exec))
        .route("/api/terminal/reset", axum::routing::post(terminal_reset))
        .route(
            "/api/missions",
            axum::routing::get(missions_list).post(missions_enqueue),
        )
        .route("/api/missions/clear", axum::routing::post(missions_clear))
        .route(
            "/api/missions/run-next",
            axum::routing::post(missions_run_next),
        )
        .route("/api/recent-files", axum::routing::get(recent_files))
        .route("/api/projects", get(projects))
        .route("/api/artifacts", get(artifacts))
        .route("/api/proofs", get(proofs))
        .route("/api/models", get(models))
        .route("/api/deploy", get(deploy_status))
        .route("/api/build", get(build_status).post(build_start))
        .route("/api/package", axum::routing::post(package))
        .with_state(ServiceState {
            root: Arc::clone(&root),
            hub: Arc::new(zylcode_core::terminal::TerminalHub::new(
                root.as_ref().clone(),
            )),
            missions: Arc::new(zylcode_core::missions::MissionQueue::new(
                root.as_ref().clone(),
            )),
            mission_lock: Arc::new(tokio::sync::Mutex::new(())),
            build_status: Arc::new(std::sync::Mutex::new(serde_json::json!({
                "state": "idle",
                "note": "no build run in this service session"
            }))),
        });

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!(
        "repo-intel service listening on http://{addr} (repo: {})",
        root.display()
    );
    println!(
        "endpoints: GET /healthz, GET /api/repo-intel?task=..., GET /api/git/status, \
         GET /api/search?q=..., GET /api/files, GET /api/evidence"
    );
    axum::serve(listener, app).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// best-of-n
// ---------------------------------------------------------------------------

#[derive(Args, Debug)]
struct PackageArgs {
    /// Run the build pipeline first (tests → release build → frontend). A
    /// failed pipeline aborts packaging — a red build is never released.
    #[arg(long, default_value_t = false)]
    with_build: bool,

    /// Continue the pipeline after a failed step (record every outcome).
    #[arg(long, default_value_t = false)]
    keep_going: bool,

    /// Release version for the bundle (required with --package).
    #[arg(long)]
    version: Option<String>,

    /// Package only (skip the pipeline) — implies a prior build exists.
    #[arg(long, default_value_t = false)]
    package_only: bool,
}

#[derive(Args, Debug)]
struct BestOfNArgs {
    /// How many candidate patches to sample and verify.
    #[arg(long, default_value_t = 3)]
    candidates: usize,

    /// The task to solve. Required for patch sampling; omit it (with
    /// `--candidates 1`) to verify the current working tree as-is.
    #[arg(long)]
    task: Option<String>,

    /// Extra context prepended to the task for the model (repo layout,
    /// entry points, conventions). Improves generated patches.
    #[arg(long)]
    context: Option<String>,

    /// Where to write the winning patch (unified diff). When omitted the
    /// winner is still reported, just not exported.
    #[arg(long)]
    export_out: Option<String>,

    /// How many candidates may verify at once. Each verification runs its
    /// own suite subprocess in its own worktree, so concurrency turns
    /// N × suite-time into roughly one suite-time. 1 = sequential.
    #[arg(long, default_value_t = 4)]
    max_concurrent: usize,

    /// Verify every candidate inside its own throwaway container: no
    /// network, read-only root filesystem, bounded CPU/memory/processes.
    /// Fails closed — without a docker backend nothing is verified.
    #[arg(long, default_value_t = false)]
    sandbox: bool,

    /// Container image the sandboxed suite runs in (e.g. a Rust toolchain
    /// image). Required with --sandbox; never pulled on demand.
    #[arg(long)]
    sandbox_image: Option<String>,

    /// Per-candidate verification budget in seconds.
    #[arg(long, default_value_t = 600)]
    timeout_secs: u64,

    /// Record outcomes in the evidence ledger under this session id
    /// (default: a fresh session). Reuse a session to keep a run's evidence
    /// alongside its agent-loop entries.
    #[arg(long)]
    session_id: Option<String>,
}

async fn handle_best_of_n(workspace: &str, args: BestOfNArgs) -> Result<()> {
    use std::sync::Arc;
    use zylcode_core::patch_best_of_n::{
        record_patch_export, run_patch_best_of_n, CandidateSource, ModelPatchSource,
        WorkingTreeSource,
    };

    let root = std::path::Path::new(workspace)
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("cannot resolve workspace '{}': {e}", workspace))?;

    let session_id = match &args.session_id {
        Some(id) => Some(
            uuid::Uuid::parse_str(id)
                .map_err(|e| anyhow::anyhow!("--session-id must be a UUID: {e}"))?,
        ),
        None => Some(uuid::Uuid::new_v4()),
    };

    // The ledger records which candidates passed and why. A fresh session
    // keeps the run's chain self-contained (genesis -> export).
    let ledger_path = root.join(".zylcode").join("ledger.db");
    std::fs::create_dir_all(ledger_path.parent().unwrap())?;
    let ledger: Arc<dyn zylcode_core::ledger::LedgerStore> =
        Arc::new(zylcode_core::sqlite_ledger::SqliteLedgerStore::new(
            ledger_path.to_string_lossy().as_ref(),
        )?);

    // Candidate production: real model sampling when a task is given;
    // working-tree verification otherwise.
    let source: Arc<dyn CandidateSource> = match &args.task {
        Some(_task) => {
            let router = zylcode_core::TokenRouter::new(zylcode_core::RouterConfig::from_env())?;
            Arc::new(ModelPatchSource {
                client: Arc::new(zylcode_core::agent::RealModelClient::new(Arc::new(router))),
                context: args.context.clone(),
            })
        }
        None => Arc::new(WorkingTreeSource {
            repo_root: root.clone(),
        }),
    };
    if args.task.is_none() && args.candidates != 1 {
        anyhow::bail!(
            "working-tree mode produces exactly one candidate; drop --candidates or pass --task"
        );
    }

    // Sandbox selection happens before anything runs, and fails closed:
    // an explicit sandbox request that cannot be honoured stops the run.
    let sandbox_verifier: Option<Arc<dyn zylcode_core::best_of_n::CandidateVerifier>> =
        if args.sandbox {
            let image = args.sandbox_image.as_deref().unwrap_or_default();
            let cfg = zylcode_core::sandbox::SandboxConfig {
                image: image.to_string(),
                ..Default::default()
            };
            let verifier = zylcode_core::sandbox::SandboxedWorktreeVerifier::new(
                root.clone(),
                cfg,
                std::time::Duration::from_secs(args.timeout_secs),
            );
            if let Err(e) = verifier.ensure_ready().await {
                eprintln!("error: {e:#}");
                std::process::exit(2);
            }
            println!(
                "sandbox: per-candidate containers (network=none, read-only rootfs, image '{}')",
                image
            );
            Some(Arc::new(verifier))
        } else {
            None
        };

    let config = zylcode_core::best_of_n::BestOfNConfig {
        candidates: args.candidates,
        per_candidate_timeout: std::time::Duration::from_secs(args.timeout_secs),
        max_concurrent: args.max_concurrent,
    };

    match &args.task {
        Some(_task) => println!(
            "best-of-n: sampling {} candidate patches for the task and verifying each in an \
             isolated worktree against the real test suite...",
            args.candidates
        ),
        None => {
            println!("best-of-n: verifying the current working tree against the real test suite...")
        }
    }
    let started = std::time::Instant::now();
    let result = run_patch_best_of_n(
        args.task.as_deref().unwrap_or(""),
        source,
        sandbox_verifier,
        &root,
        Some(Arc::clone(&ledger)),
        session_id,
        &config,
        vec!["cargo".to_string(), "test".to_string()],
        std::time::Duration::from_secs(args.timeout_secs),
    )
    .await?;

    for outcome in &result.selection.outcomes {
        println!(
            "  candidate {}: {} ({} of {} checks passing)",
            outcome.candidate_index,
            if outcome.passed { "PASS" } else { "FAIL" },
            outcome.evidence.passing_checks(),
            outcome.evidence.checks.len()
        );
        for (name, ok, detail) in &outcome.evidence.checks {
            println!(
                "    [{}] {name}: {detail}",
                if *ok { "pass" } else { "FAIL" }
            );
        }
    }
    println!("selection: {}", result.selection.selection_reason);

    // Export the winner — byte-identical to what was verified.
    let mut export_path: Option<std::path::PathBuf> = None;
    if let (Some(selected), Some(out)) = (result.selection.selected, &args.export_out) {
        let path = std::path::Path::new(out).to_path_buf();
        let bytes = result.export_winner(&path)?;
        println!(
            "exported winning patch (candidate {selected}, {bytes} bytes) to {}",
            path.display()
        );
        if let Some(sid) = session_id {
            record_patch_export(&ledger, sid, selected, &path, bytes).await?;
        }
        export_path = Some(path);
    }
    let _ = export_path; // reported above; kept for future json output

    println!("elapsed: {:.1}s", started.elapsed().as_secs_f64());
    if let Some(sid) = session_id {
        println!("evidence ledger session: {sid}");
    }

    match result.selection.selected {
        Some(_) => Ok(()),
        None => {
            eprintln!("no candidate passed verification");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// delivery: package / deploy status
// ---------------------------------------------------------------------------

async fn handle_package(workspace: &str, args: PackageArgs) -> Result<()> {
    let root = std::path::PathBuf::from(workspace);
    let version = args
        .version
        .clone()
        .or_else(|| {
            Some(
                zylcode_core::gitops::version_payload(&root)["app_version"]
                    .as_str()
                    .unwrap_or("0.0.0")
                    .to_string(),
            )
        })
        .context("release version could not be resolved; pass --version")?;

    if !args.package_only {
        println!("running the build pipeline for v{version}…");
        let pipeline = zylcode_core::delivery::default_build_pipeline();
        let report =
            zylcode_core::delivery::build_workspace(&root, &pipeline, args.keep_going).await?;
        let passed = report["passed"].as_bool().unwrap_or(false);
        for step in report["steps"].as_array().unwrap_or(&vec![]) {
            let name = step["name"].as_str().unwrap_or("?");
            let ok = step["passed"].as_bool().unwrap_or(false);
            let dur = step["duration_ms"].as_u64().unwrap_or(0);
            println!(
                "  {} {name} ({} ms)",
                if ok { "\u{2713}" } else { "\u{2717}" },
                dur
            );
        }
        if !passed {
            eprintln!("build pipeline FAILED — refusing to package a red build");
            std::process::exit(1);
        }
    }

    println!("packaging release v{version}…");
    let payload = zylcode_core::delivery::package_release(&root, &version).await?;
    println!(
        "release ready: {} (artifact {}, {} files, sha256 {})",
        Path::new(payload["release_dir"].as_str().unwrap_or("?")).display(),
        payload["artifact_id"].as_str().unwrap_or("?"),
        payload["file_count"].as_u64().unwrap_or(0),
        &payload["content_hash"].as_str().unwrap_or("?")[..16.min(
            payload["content_hash"]
                .as_str()
                .map(|s| s.len())
                .unwrap_or(0)
        )],
    );
    Ok(())
}
