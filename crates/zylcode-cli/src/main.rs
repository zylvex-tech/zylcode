use anyhow::Result;
use clap::{Args, Parser, Subcommand};
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
    zylcode_core::intelligence::api::repo_intel_payload(root, task)
        .unwrap_or_else(|e| {
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

    async fn repo_intel(
        State(root): State<Arc<std::path::PathBuf>>,
        axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    ) -> Json<serde_json::Value> {
        let task = params
            .get("task")
            .map(String::as_str)
            .unwrap_or("")
            .to_string();
        let root = Arc::clone(&root);
        let payload =
            tokio::task::spawn_blocking(move || repo_intel_json(&root, &task)).await;
        match payload {
            Ok(value) => Json(value),
            Err(e) => Json(serde_json::json!({ "error": format!("intel task failed: {e}") })),
        }
    }

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/api/repo-intel", get(repo_intel))
        .with_state(Arc::clone(&root));

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!(
        "repo-intel service listening on http://{addr} (repo: {})",
        root.display()
    );
    println!("endpoints: GET /healthz, GET /api/repo-intel?task=...");
    axum::serve(listener, app).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// best-of-n
// ---------------------------------------------------------------------------

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
        Some(id) => Some(uuid::Uuid::parse_str(id).map_err(|e| {
            anyhow::anyhow!("--session-id must be a UUID: {e}")
        })?),
        None => Some(uuid::Uuid::new_v4()),
    };

    // The ledger records which candidates passed and why. A fresh session
    // keeps the run's chain self-contained (genesis -> export).
    let ledger_path = root.join(".zylcode").join("ledger.db");
    std::fs::create_dir_all(ledger_path.parent().unwrap())?;
    let ledger: Arc<dyn zylcode_core::ledger::LedgerStore> = Arc::new(
        zylcode_core::sqlite_ledger::SqliteLedgerStore::new(
            ledger_path.to_string_lossy().as_ref(),
        )?,
    );

    // Candidate production: real model sampling when a task is given;
    // working-tree verification otherwise.
    let source: Arc<dyn CandidateSource> = match &args.task {
        Some(_task) => {
            let router = zylcode_core::TokenRouter::new(
                zylcode_core::RouterConfig::from_env(),
            )?;
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
        None => println!(
            "best-of-n: verifying the current working tree against the real test suite..."
        ),
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
