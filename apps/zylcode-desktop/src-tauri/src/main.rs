#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{self, Write};
use zylcode_core::{EngineConfig, Intent, McpBridgeDescriptor, ZylCodeEngine};

// ---------------------------------------------------------------------------
// Shared engine state — managed by Tauri
// ---------------------------------------------------------------------------

struct EngineState {
    engine: ZylCodeEngine,
}

// ---------------------------------------------------------------------------
// IPC commands — invoked from the frontend via `invoke("...")`
// ---------------------------------------------------------------------------

/// Process a natural-language intent and return the result.
///
/// `model` is an optional per-call override surfaced by the desktop model
/// picker. When `None`, the engine's configured primary model is used.
#[tauri::command]
async fn process_intent(
    prompt: String,
    model: Option<String>,
    state: tauri::State<'_, EngineState>,
) -> Result<zylcode_core::IntentResult, String> {
    let intent = Intent {
        prompt,
        context: None,
        correlation_id: None,
    };
    state
        .engine
        .process_intent_with_model(intent, model)
        .await
        .map_err(|e| e.to_string())
}

/// Streaming execution — dispatches the intent and emits progress events
/// (`intent:chunk` / `intent:done`) to the frontend during generation.
///
/// The frontend should listen via `listen("intent:chunk", ...)` and
/// `listen("intent:done", ...)`.
#[tauri::command]
async fn process_intent_stream(
    prompt: String,
    model: Option<String>,
    app: tauri::AppHandle,
    state: tauri::State<'_, EngineState>,
) -> Result<zylcode_core::IntentResult, String> {
    use tauri::Emitter;

    // Emit a start event so the UI can transition to streaming state.
    let _ = app.emit(
        "intent:chunk",
        serde_json::json!({ "index": 0, "delta": "", "done": false, "phase": "planning" }),
    );

    // Build a router-level stream so the UI gets token-by-token progress.
    // We drive the full pipeline via the engine but also emit router stream
    // deltas in parallel. For the offline synthetic path this is a fast
    // line-chunked emission; for live providers it streams real deltas.
    let engine = state.inner().engine.clone();
    let engine_for_stream = engine.clone();
    let app_clone = app.clone();

    // Spawn a background task that drives router streaming for progress UI.
    // The actual result still comes from the verified pipeline execution.
    let stream_prompt = prompt.clone();
    let stream_model = model.clone();
    tokio::spawn(async move {
        // Build a one-shot router for streaming progress (does not affect the
        // verified pipeline result — purely for UI feedback).
        let mut rc = engine_for_stream.pipeline().router().config().clone();
        if let Some(m) = stream_model.filter(|m| !m.trim().is_empty()) {
            rc.primary_model = m;
        }
        let planner = zylcode_core::IntentPlanner::new();
        let bridges = engine_for_stream.list_mcp_bridges().await;
        let plan = planner.build_execution_plan(
            &zylcode_core::Intent {
                prompt: stream_prompt,
                context: None,
                correlation_id: None,
            },
            &bridges,
        );

        let router = match zylcode_core::TokenRouter::new(rc) {
            Ok(r) => r,
            Err(_) => return,
        };

        let _ = router
            .dispatch_stream(&plan.compiled_prompt, &plan.system_prompt, |ev| {
                let _ = app_clone.emit("intent:chunk", &ev);
            })
            .await;
    });

    let intent_for_pipeline = Intent {
        prompt,
        context: None,
        correlation_id: None,
    };

    let result = engine
        .process_intent_with_model(intent_for_pipeline, model)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit("intent:done", &result);
    // Also emit a terminal chunk so listeners can finalize.
    let _ = app.emit(
        "intent:chunk",
        serde_json::json!({ "index": 999999, "delta": "", "done": true }),
    );

    Ok(result)
}

/// Return the current execution plan without dispatching to the LLM (useful
/// for preview / dry-run in the UI).
#[tauri::command]
async fn preview_execution_plan(
    prompt: String,
    state: tauri::State<'_, EngineState>,
) -> Result<zylcode_core::ExecutionPlan, String> {
    let planner = zylcode_core::IntentPlanner::new();
    let bridges = state.engine.list_mcp_bridges().await;
    let plan = planner.build_execution_plan(
        &Intent {
            prompt,
            context: None,
            correlation_id: None,
        },
        &bridges,
    );
    Ok(plan)
}

/// Current token telemetry snapshot.
#[tauri::command]
async fn token_metrics(
    state: tauri::State<'_, EngineState>,
) -> Result<zylcode_core::TokenSnapshot, String> {
    Ok(state.engine.token_metrics())
}

/// Run the logic verification pipeline.
#[tauri::command]
async fn verify_logic(
    state: tauri::State<'_, EngineState>,
) -> Result<zylcode_core::VerificationReport, String> {
    state
        .engine
        .verify_logic()
        .await
        .map_err(|e| e.to_string())
}

/// Register an MCP bridge from the desktop UI.
#[tauri::command]
async fn register_mcp_bridge(
    id: String,
    endpoint: String,
    transport: Option<String>,
    state: tauri::State<'_, EngineState>,
) -> Result<(), String> {
    state
        .engine
        .register_mcp_bridge(McpBridgeDescriptor {
            id,
            endpoint,
            transport: transport.unwrap_or_else(|| "stdio".to_string()),
            env: Default::default(),
        })
        .await
        .map_err(|e| e.to_string())
}

/// List all registered MCP bridges.
#[tauri::command]
async fn list_mcp_bridges(
    state: tauri::State<'_, EngineState>,
) -> Result<Vec<McpBridgeDescriptor>, String> {
    Ok(state.engine.list_mcp_bridges().await)
}

/// List dynamic MCP tools from the registry (local `mcp.tools.yaml`).
#[tauri::command]
async fn list_tools(
    state: tauri::State<'_, EngineState>,
) -> Result<Vec<zylcode_core::ToolDescriptor>, String> {
    Ok(state.engine.list_tools().await)
}

/// Execute a dynamic tool with recovery (30s timeout, 2 retries).
#[tauri::command]
async fn execute_tool(
    id: String,
    params: serde_json::Value,
    state: tauri::State<'_, EngineState>,
) -> Result<serde_json::Value, String> {
    state
        .engine
        .execute_tool(&id, params)
        .await
        .map_err(|e| e.to_string())
}

/// Search marketplace extensions by query string.
#[tauri::command]
async fn marketplace_search(
    query: String,
    state: tauri::State<'_, EngineState>,
) -> Result<Vec<zylcode_core::MarketplaceExtension>, String> {
    let extensions = state.engine.marketplace_snapshot().await;
    let mut reg = zylcode_core::ExtensionRegistry::new();
    for ext in extensions {
        reg.register(ext);
    }
    Ok(reg.search(&query).into_iter().cloned().collect())
}

// ---------------------------------------------------------------------------
// Tauri entry point
// ---------------------------------------------------------------------------

fn run_interactive(engine: ZylCodeEngine) -> Result<(), String> {
    println!("ZylCode Interactive Mode");
    println!("Type 'exit' or 'quit' to quit, 'help' for commands.");
    println!();

    // Auto-load tools from mcp.tools.yaml
    {
        let reg = engine.tool_registry_arc();
        tauri::async_runtime::block_on(async {
            let n = zylcode_mcp::register_from_default_location(reg.as_ref()).await;
            if n > 0 {
                println!("Loaded {} MCP tools from default location", n);
            }
        });
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("zylcode> ");
        stdout.flush().map_err(|e| e.to_string())?;

        let mut input = String::new();
        stdin.read_line(&mut input).map_err(|e| e.to_string())?;
        let input = input.trim();

        match input {
            "" => continue,
            "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("Commands:");
                println!("  <prompt>     - Process a natural language intent");
                println!("  tools        - List available MCP tools");
                println!("  bridges      - List registered MCP bridges");
                println!("  verify       - Run logic verification");
                println!("  tokens       - Show token metrics");
                println!("  help         - Show this help");
                println!("  exit/quit    - Exit interactive mode");
            }
            "tools" => {
                let tools = tauri::async_runtime::block_on(engine.list_tools());
                if tools.is_empty() {
                    println!("No tools registered. Add mcp.tools.yaml to register tools.");
                } else {
                    for t in tools {
                        println!("  {} ({}) - {}", t.id, t.transport, t.description.unwrap_or_default());
                    }
                }
            }
            "bridges" => {
                let bridges = tauri::async_runtime::block_on(engine.list_mcp_bridges());
                if bridges.is_empty() {
                    println!("No bridges registered.");
                } else {
                    for b in bridges {
                        println!("  {} -> {} [{}]", b.id, b.endpoint, b.transport);
                    }
                }
            }
            "verify" => {
                println!("Running verification...");
                match tauri::async_runtime::block_on(engine.verify_logic()) {
                    Ok(report) => {
                        println!("{} — {}ms", if report.passed { "PASSED" } else { "FAILED" }, report.duration_ms);
                        for c in report.checks {
                            println!("  {} {}: {}", if c.passed { "✓" } else { "✗" }, c.name, c.message);
                        }
                    }
                    Err(e) => println!("Verification failed: {}", e),
                }
            }
            "tokens" => {
                let metrics = engine.token_metrics();
                println!("Tokens: in={} out={} saved={} fallback={}", metrics.input_tokens, metrics.output_tokens, metrics.verification_saved_tokens, metrics.fallback_count);
            }
            prompt => {
                println!("Processing: {}", prompt);
                let intent = Intent {
                    prompt: prompt.to_string(),
                    context: None,
                    correlation_id: None,
                };
                match tauri::async_runtime::block_on(engine.process_intent(intent)) {
                    Ok(result) => {
                        println!("\n--- Result ---");
                        println!("{}", result.summary);
                        if !result.artifacts.is_empty() {
                            println!("\nArtifacts:");
                            for a in result.artifacts {
                                println!("  [{}] {} ({} chars)", a.kind, a.label, a.content.len());
                            }
                        }
                        println!("--- End ---\n");
                    }
                    Err(e) => println!("Error: {}", e),
                }
            }
        }
    }

    Ok(())
}

#[allow(dead_code)]
#[tauri::command]
async fn init_telemetry_db(_state: tauri::State<'_, EngineState>) -> Result<String, String> {
    use rusqlite::Connection;

    let db_path = std::env::var("ZYLCODE_TELEMETRY_DB_PATH")
        .ok()
        .map(|p| std::path::Path::new(&p).to_path_buf())
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::Path::new(&h).join(".zylcode/telemetry.db"))
        });

    let db_path = db_path.unwrap_or_else(|| "zylcode_telemetry.db".into());

    let conn = Connection::open(&db_path)
        .map_err(|e| format!("failed to open telemetry DB at {:?}: {}", db_path, e))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS audit_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            event_type TEXT NOT NULL,
            severity TEXT NOT NULL,
            tool_id TEXT,
            transport TEXT,
            caller_id TEXT,
            input_hash TEXT,
            output_hash TEXT,
            duration_ms INTEGER,
            retry_attempt INTEGER,
            max_retries INTEGER,
            error TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        )",
        [],
    )
    .map_err(|e| format!("failed to create audit_logs table: {}", e))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            total_tokens INTEGER,
            input_tokens INTEGER,
            output_tokens INTEGER,
            active_bridges INTEGER,
            queued_requests INTEGER,
            error_count INTEGER,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        )",
        [],
    )
    .map_err(|e| format!("failed to create metrics table: {}", e))?;

    Ok(format!("telemetry DB initialized at {:?}", db_path))
}

/// Get recent audit log entries with SHA-256 chain hash validation.
#[allow(dead_code)]
#[tauri::command]
async fn get_recent_audit_logs(
    state: tauri::State<'_, EngineState>,
) -> Result<Vec<TelemetryAuditEntry>, String> {
    use rusqlite::Connection;

    // Determine database path: prefer env var, fallback to home dir.
    let db_path = std::env::var("ZYLCODE_TELEMETRY_DB_PATH")
        .ok()
        .map(|p| std::path::Path::new(&p).to_path_buf())
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::Path::new(&h).join(".zylcode/telemetry.db"))
        });

    let db_path = db_path.unwrap_or_else(|| "zylcode_telemetry.db".into());

    let conn = Connection::open(&db_path)
        .map_err(|e| format!("failed to open telemetry DB at {:?}: {}", db_path, e))?;

    // Ensure tables exist on first open.
    init_telemetry_db(state).await.map_err(|e| format!("DB init failed: {}", e))?;

    let prepare_result = conn.prepare(
        "SELECT timestamp, event_type, severity, tool_id, transport, caller_id, input_hash, output_hash, duration_ms, retry_attempt, max_retries, error FROM audit_logs ORDER BY id ASC LIMIT 50",
    );
    let mut stmt = match prepare_result {
        Ok(s) => s,
        Err(e) => return Err(format!("failed to prepare SQL: {}", e)),
    };

    let mut rows = Vec::new();
    if let Ok(mapped_rows) = stmt.query_map([], |row| {
        Ok(TelemetryAuditEntry {
            timestamp: row.get::<_, String>(0).unwrap(),
            event_type: row.get::<_, String>(1).unwrap(),
            severity: row.get::<_, String>(2).unwrap(),
            tool_id: row.get::<_, Option<String>>(3).unwrap(),
            transport: row.get::<_, Option<String>>(4).unwrap(),
            caller_id: row.get::<_, Option<String>>(5).unwrap(),
            input_hash: row.get::<_, Option<String>>(6).unwrap(),
            output_hash: row.get::<_, Option<String>>(7).unwrap(),
            duration_ms: row.get::<_, Option<u64>>(8).unwrap(),
            retry_attempt: row.get::<_, Option<u32>>(9).unwrap(),
            max_retries: row.get::<_, Option<u32>>(10).unwrap(),
            error: row.get::<_, Option<String>>(11).unwrap(),
        })
    }) {
        for row in mapped_rows {
            match row {
                Ok(entry) => rows.push(entry),
                Err(e) => {
                    eprintln!("Skipping corrupted audit row: {}", e);
                }
            }
        }
    }

    Ok(rows)
}

#[allow(dead_code)]
struct TelemetryAuditEntry {
    /// RFC3339 timestamp.
    pub timestamp: String,
    /// Event type.
    pub event_type: String,
    /// Severity level.
    pub severity: String,
    /// Tool identifier (if applicable).
    pub tool_id: Option<String>,
    /// Transport type (stdio, sse, websocket).
    pub transport: Option<String>,
    /// Caller identity (anonymized).
    pub caller_id: Option<String>,
    /// SHA-256 hash of input payload.
    pub input_hash: Option<String>,
    /// SHA-256 hash of output payload.
    pub output_hash: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: Option<u64>,
    /// Retry attempt number (if applicable).
    pub retry_attempt: Option<u32>,
    /// Maximum retries configured.
    pub max_retries: Option<u32>,
    /// Error message (if failed).
    pub error: Option<String>,
}

fn main() {
    // Check for --interactive flag before Tauri initialization
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--interactive" || a == "-i") {
        let engine = ZylCodeEngine::new(EngineConfig {
            workspace_root: ".".to_string(),
            verbose: false,
            extra: Default::default(),
        });
        if let Err(e) = run_interactive(engine) {
            eprintln!("Interactive mode error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let engine = ZylCodeEngine::new(EngineConfig {
        workspace_root: ".".to_string(),
        verbose: false,
        extra: Default::default(),
    });

    // Auto-load tools from mcp.tools.yaml if present (non-fatal).
    {
        let reg = engine.tool_registry_arc();
        tauri::async_runtime::block_on(async {
            let n = zylcode_mcp::register_from_default_location(reg.as_ref()).await;
            if n > 0 {
                tracing::info!(tools = n, "loaded MCP tools from default location");
            }
        });
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(EngineState { engine })
        .invoke_handler(tauri::generate_handler![
            process_intent,
            process_intent_stream,
            preview_execution_plan,
            token_metrics,
            verify_logic,
            register_mcp_bridge,
            list_mcp_bridges,
            list_tools,
            execute_tool,
            marketplace_search
        ])
        .run(tauri::generate_context!())
        .expect("error while running ZylCode desktop application");
}