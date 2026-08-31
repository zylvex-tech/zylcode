#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

fn main() {
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
        .expect("error while running ZylCode desktop");
}
