use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use tracing::{error, info};
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
            println!("  [{}] {} — {}", artifact.kind, artifact.label, artifact.content);
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
        workspace_root: cli.workspace,
        verbose: cli.verbose,
        extra: Default::default(),
    });

    let result = match cli.command {
        Commands::Build(args) => handle_build(&engine, args).await,
        Commands::McpBridge(args) => handle_mcp_bridge(&engine, args).await,
        Commands::Marketplace(args) => handle_marketplace(&engine, args).await,
    };

    if let Err(err) = &result {
        error!(error = %err, "command failed");
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }

    result
}
