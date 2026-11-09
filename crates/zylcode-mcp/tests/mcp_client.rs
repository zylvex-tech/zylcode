//! Integration tests for the MCP client.
//!
//! Every test here spawns `mcp-test-server`, a real child process, and speaks
//! the protocol to it over stdio. Nothing is mocked: the handshake is a real
//! handshake, the pagination is real pagination, the crash is a real exit.
//!
//! What is deliberately asserted, beyond the happy path:
//!
//! * a server that selects a protocol version this client has not implemented is
//!   refused with the versions it advertises, never silently downgraded;
//! * a server that writes junk to stdout produces an error, not a desynchronised
//!   session;
//! * an unannotated tool is **not** treated as read-only;
//! * a refusal from the permission gate records evidence and never reaches the
//!   server (proved by the server's own observation log being empty);
//! * a dead server is reported dead, and the supervisor reconnects for the next
//!   call rather than replaying a call that may already have executed.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use zylcode_mcp::evidence::{JsonlEvidenceSink, ToolRuntime};
use zylcode_mcp::mcp_client::config::McpServerConfig;
use zylcode_mcp::mcp_client::registry::{McpRegistry, ServerState};
use zylcode_mcp::mcp_client::session::{ClientInfo, McpSession, ProtocolVersionRejected};
use zylcode_mcp::mcp_client::transport::StdioTransport;
use zylcode_mcp::permission::PermissionGate;
use zylcode_mcp::real_tools::{dispatch_with, RealTool, RiskLevel, ToolContext};

fn client() -> ClientInfo {
    ClientInfo {
        name: "zylcode-test".to_string(),
        version: "0.0.0".to_string(),
    }
}

fn test_server_path() -> &'static str {
    env!("CARGO_BIN_EXE_mcp-test-server")
}

fn config(id: &str, mode: Option<&str>) -> McpServerConfig {
    let mut cfg = McpServerConfig::stdio(id, test_server_path());
    if let Some(mode) = mode {
        cfg.args.push(format!("--mode={mode}"));
    }
    // Generous by default so a slow CI machine does not turn a real assertion
    // into a flake; the timeout test sets its own, smaller value.
    cfg.timeout_ms = 15_000;
    cfg
}

fn context() -> ToolContext {
    ToolContext {
        working_directory: PathBuf::from("."),
        environment: HashMap::new(),
        timeout: Duration::from_secs(5),
        session_id: None,
        actor: Some("test-suite".to_string()),
        approval_required: false,
    }
}

async fn connect(cfg: &McpServerConfig) -> anyhow::Result<McpSession> {
    let options = cfg.stdio_options()?;
    let transport = Box::new(StdioTransport::spawn(&options)?);
    McpSession::connect(transport, client(), Duration::from_millis(cfg.timeout_ms)).await
}

// ---------------------------------------------------------------------------
// Handshake
// ---------------------------------------------------------------------------

#[tokio::test]
async fn handshake_negotiates_a_version_and_reports_server_identity() {
    let cfg = config("happy", None);
    let session = connect(&cfg).await.expect("handshake must succeed");
    let handshake = session.handshake();
    assert_eq!(handshake.protocol_version, "2025-06-18");
    assert_eq!(handshake.server_info.name, "zylcode-test-server");
    assert_eq!(handshake.server_info.version, "1.0.0");
    assert!(
        handshake.instructions.is_some(),
        "instructions supplied by the server must be retained for the model"
    );
}

#[tokio::test]
async fn initialized_notification_precedes_the_first_request() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("methods.log");
    let mut cfg = config("ordering", None);
    cfg.env.insert(
        "ZYLCODE_TEST_SERVER_LOG".to_string(),
        log.to_string_lossy().to_string(),
    );

    let mut session = connect(&cfg).await.expect("handshake must succeed");
    session.list_all_tools().await.expect("tools must list");

    let observed = std::fs::read_to_string(&log).expect("server log must exist");
    let methods: Vec<&str> = observed.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        methods.first(),
        Some(&"initialize"),
        "initialize must be the first method, got {methods:?}"
    );
    assert_eq!(
        methods.get(1),
        Some(&"notifications/initialized"),
        "the spec requires `notifications/initialized` before any other request, got {methods:?}"
    );
    assert!(
        methods.contains(&"tools/list"),
        "discovery must follow the notification"
    );
}

#[tokio::test]
async fn unsupported_protocol_version_is_refused_with_the_versions_offered() {
    let cfg = config("future", Some("wrong-version"));
    let error = connect(&cfg)
        .await
        .expect_err("2026-07-28 is not implemented");
    let rejected = error
        .downcast_ref::<ProtocolVersionRejected>()
        .expect("the error must name the protocol problem, not a generic failure");
    assert!(
        rejected.offered.contains(&"2025-06-18".to_string()),
        "the client must state what it does implement"
    );
    assert!(
        rejected.to_string().contains("2026-07-28"),
        "the message must name the version the server selected: {rejected}"
    );
}

#[tokio::test]
async fn malformed_output_is_an_error_not_a_desynchronised_session() {
    let cfg = config("junk", Some("malformed"));
    let result = connect(&cfg).await;
    assert!(
        result.is_err(),
        "a server writing non-JSON to stdout must produce an error"
    );
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_list_pagination_is_followed_to_the_last_page() {
    let cfg = config("paging", None);
    let mut session = connect(&cfg).await.unwrap();
    let tools = session
        .list_all_tools()
        .await
        .expect("paging must complete");
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["alpha", "beta", "gamma"],
        "both pages must be merged in order"
    );
}

#[tokio::test]
async fn annotations_map_to_risk_and_unannotated_tools_fail_closed() {
    let registry = McpRegistry::new(vec![config("risk", None)], client());
    let slot = registry.slot("risk").expect("slot exists");
    slot.ensure_ready().await.expect("server must start");
    let tools = slot.remote_tools().await;
    assert_eq!(tools.len(), 3);

    let by_name = |name: &str| {
        tools
            .iter()
            .find(|t| t.definition().name == name)
            .unwrap_or_else(|| panic!("tool `{name}` must be discovered"))
    };

    assert_eq!(
        by_name("alpha").risk_level(),
        RiskLevel::Write,
        "an unannotated tool must be approval-requiring, not read-only"
    );
    assert_eq!(by_name("beta").risk_level(), RiskLevel::Read);
    assert_eq!(by_name("gamma").risk_level(), RiskLevel::Destructive);

    assert_eq!(
        by_name("alpha").id(),
        "risk__alpha",
        "remote ids must be namespaced so they cannot shadow built-ins"
    );
}

// ---------------------------------------------------------------------------
// Calls
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_call_returns_content_and_structured_output() {
    let cfg = config("call", None);
    let mut session = connect(&cfg).await.unwrap();
    let outcome = session
        .call_tool("alpha", serde_json::json!({ "n": 1 }))
        .await
        .expect("call must succeed");
    assert!(!outcome.is_error);
    assert!(
        outcome.structured.is_some(),
        "structuredContent must be retained when the server supplies it"
    );
}

#[tokio::test]
async fn unknown_tool_surfaces_the_server_error() {
    let cfg = config("unknown", None);
    let mut session = connect(&cfg).await.unwrap();
    let error = session
        .call_tool("missing", serde_json::json!({}))
        .await
        .expect_err("an unknown tool must fail");
    assert!(
        error.to_string().contains("Unknown tool"),
        "the server's message must survive: {error}"
    );
}

#[tokio::test]
async fn slow_server_times_out_and_cancellation_reaches_the_server() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("methods.log");
    let mut cfg = config("slow", Some("slow"));
    // The floor the config validator enforces; the fixture sleeps 2s.
    cfg.timeout_ms = 1_000;
    cfg.env.insert(
        "ZYLCODE_TEST_SERVER_LOG".to_string(),
        log.to_string_lossy().to_string(),
    );

    let mut session = connect(&cfg).await.expect("handshake must succeed");
    let error = session
        .call_tool("alpha", serde_json::json!({}))
        .await
        .expect_err("a 2s tool must not fit in a 1s budget");
    assert!(
        error.to_string().to_lowercase().contains("timeout")
            || error.to_string().to_lowercase().contains("timed out"),
        "the failure must name the timeout: {error}"
    );

    // Cancellation is an explicit notification, never inferred from a dropped
    // stream. Assert the frame actually reaches the child. The id is synthetic
    // because a timed-out call does not expose its id; this asserts the wire
    // path, not correlation.
    session
        .cancel(&zylcode_mcp::mcp_client::RequestId::Number(9999), "test")
        .await
        .expect("cancellation must be sendable");
    // The fixture is single-threaded and still sleeping inside `tools/call`, so
    // it cannot read the cancellation until that sleep finishes. Waiting is the
    // honest way to observe the notification; reading immediately would only
    // prove that the test runs faster than the server.
    tokio::time::sleep(Duration::from_millis(2_500)).await;
    let observed = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        observed.contains("notifications/cancelled"),
        "the cancellation notification must reach the server, saw: {observed}"
    );
}

#[tokio::test]
async fn a_dead_server_is_reported_and_the_next_call_reconnects() {
    let mut cfg = config("crashy", Some("die-after-call"));
    cfg.restart.max_attempts = 2;
    cfg.restart.initial_backoff_ms = 50;
    cfg.restart.max_backoff_ms = 60;
    let registry = McpRegistry::new(vec![cfg], client());
    let slot = registry.slot("crashy").expect("slot exists");

    slot.ensure_ready().await.expect("first start");
    let first = slot
        .call_tool("alpha", serde_json::json!({}))
        .await
        .expect("first call must succeed");
    assert!(!first.is_error);

    // The child exits after answering. The next call must fail — it must NOT be
    // replayed, because the server may already have executed it.
    let second = slot.call_tool("alpha", serde_json::json!({})).await;
    assert!(second.is_err(), "a call against a dead server must fail");

    // ...and the supervisor must recover for the next invocation.
    slot.ensure_ready().await.expect("restart must succeed");
    let third = slot
        .call_tool("alpha", serde_json::json!({}))
        .await
        .expect("call after reconnect must succeed");
    assert!(!third.is_error);
}

#[tokio::test]
async fn failed_start_is_terminal_and_reported_for_the_inspector() {
    let mut cfg = McpServerConfig::stdio("broken", "zylcode-no-such-binary-for-tests");
    cfg.restart.max_attempts = 1;
    cfg.restart.initial_backoff_ms = 50;
    cfg.restart.max_backoff_ms = 60;
    let registry = McpRegistry::new(vec![cfg], client());
    let slot = registry.slot("broken").unwrap();
    let _ = slot.ensure_ready().await;
    let snapshot = slot.snapshot().await;
    assert_eq!(snapshot.state, ServerState::Failed);
    assert!(snapshot.last_error.is_some());
    assert!(snapshot.protocol_version.is_none());
}

// ---------------------------------------------------------------------------
// Gated dispatch: the point of the adapter
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_refused_remote_tool_records_evidence_and_never_reaches_the_server() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("methods.log");
    let evidence_path = dir.path().join("evidence.jsonl");

    let mut cfg = config("gated", None);
    cfg.env.insert(
        "ZYLCODE_TEST_SERVER_LOG".to_string(),
        log.to_string_lossy().to_string(),
    );
    let registry = McpRegistry::new(vec![cfg], client());
    let slot = registry.slot("gated").unwrap();
    slot.ensure_ready().await.unwrap();
    let tool = slot
        .remote_tools()
        .await
        .into_iter()
        .find(|t| t.definition().name == "alpha")
        .expect("alpha must exist");

    let runtime = ToolRuntime::new(
        std::sync::Arc::new(PermissionGate::restrictive()),
        std::sync::Arc::new(JsonlEvidenceSink::new(&evidence_path)),
    );

    let outcome = dispatch_with(
        tool.id(),
        tool.as_ref(),
        serde_json::json!({ "should": "not run" }),
        &context(),
        &runtime,
    )
    .await;

    assert!(
        outcome.result.is_err(),
        "a restrictive gate must refuse an unannotated (write-class) remote tool"
    );
    assert!(!outcome.executed());

    let records = JsonlEvidenceSink::new(&evidence_path)
        .read_all()
        .expect("evidence must be readable");
    assert_eq!(records.len(), 1, "a refusal is still evidence");
    assert_eq!(records[0].tool_id, "gated__alpha");
    assert!(
        records[0].approval_decision.is_some(),
        "the gate's decision must be recorded"
    );
    assert_eq!(records[0].actor.as_deref(), Some("test-suite"));

    let observed = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        !observed.contains("tools/call"),
        "a refused tool must never reach the server, saw: {observed}"
    );
}

#[tokio::test]
async fn an_allowed_remote_tool_executes_and_records_the_result() {
    let dir = tempfile::tempdir().unwrap();
    let evidence_path = dir.path().join("evidence.jsonl");

    let registry = McpRegistry::new(vec![config("allowed", None)], client());
    let slot = registry.slot("allowed").unwrap();
    slot.ensure_ready().await.unwrap();
    let tool = slot
        .remote_tools()
        .await
        .into_iter()
        .find(|t| t.definition().name == "beta")
        .expect("beta must exist");

    let runtime = ToolRuntime::new(
        std::sync::Arc::new(PermissionGate::permissive()),
        std::sync::Arc::new(JsonlEvidenceSink::new(&evidence_path)),
    );

    let outcome = dispatch_with(
        tool.id(),
        tool.as_ref(),
        serde_json::json!({}),
        &context(),
        &runtime,
    )
    .await;

    assert!(
        outcome.executed(),
        "permissive policy must allow a read tool"
    );
    let result = outcome.result.expect("result");
    assert!(result.success);
    assert_eq!(
        result.output.get("server").and_then(|v| v.as_str()),
        Some("allowed")
    );

    let records = JsonlEvidenceSink::new(&evidence_path)
        .read_all()
        .expect("evidence must be readable");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].tool_id, "allowed__beta");
    assert!(
        records[0].end_time.is_some(),
        "a record must be placeable in time"
    );
}
