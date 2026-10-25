//! Commissioning test for real provider inference
//!
//! This test requires a running Ollama instance with the `llama3` model.
//! It is marked `#[ignore]` so CI does not fail when Ollama is unavailable.
//! Run manually with: `cargo test -p zylcode-core --test commissioning_test -- --ignored`
//!
//! If Ollama IS running and this test fails, that is a real defect — the test
//! will panic with the actual error, not silently return Ok(()).

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use zylcode_core::agent::{AgentLoop, AgentState, RealModelClient};
use zylcode_core::router::{RouterConfig, TokenRouter};
use zylcode_mcp::{DynamicTool, McpToolConfig, McpTransport, ToolRegistry};

#[tokio::test]
#[ignore = "requires running Ollama instance with llama3 model"]
async fn test_real_provider_commissioning() -> Result<()> {
    let registry = Arc::new(ToolRegistry::new());

    let fs_read_config = McpToolConfig {
        id: "fs.read".to_string(),
        command: "read".to_string(),
        transport: McpTransport::Stdio,
        env: Default::default(),
        enabled: true,
        description: Some("Read file contents".to_string()),
    };
    registry
        .register(Arc::new(DynamicTool::new(fs_read_config)))
        .await;

    let config = RouterConfig {
        primary_provider: zylcode_core::router::ModelProvider::LocalOllama,
        primary_model: "llama3".to_string(),
        ..Default::default()
    };

    // If Ollama is not running, this fails — and that is a test failure when
    // the test is run manually, not a silent success.
    let router =
        Arc::new(TokenRouter::new(config).expect("TokenRouter::new failed — is Ollama running?"));

    let model_client = Arc::new(RealModelClient::new(router));
    let ledger = Arc::new(zylcode_core::memory_ledger::MemoryLedgerStore::new());

    let mut agent = AgentLoop::new(
        "Read the Cargo.toml file and tell me the package name",
        PathBuf::from("."),
        None,
        registry,
        model_client,
        ledger,
    );

    let final_state = agent
        .run()
        .await
        .expect("AgentLoop::run failed — real provider commissioning defect");

    assert_eq!(
        final_state,
        AgentState::Completed,
        "agent must reach Completed when Ollama is available; got {:?}",
        final_state
    );

    let tool_messages: Vec<_> = agent
        .session()
        .messages
        .iter()
        .filter(|m| m.role == zylcode_core::agent::MessageRole::Tool)
        .collect();

    assert!(
        !tool_messages.is_empty(),
        "agent must have executed at least one tool via the real provider"
    );

    Ok(())
}
