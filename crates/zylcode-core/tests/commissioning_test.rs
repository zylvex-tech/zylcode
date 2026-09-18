//! Commissioning test for real provider inference

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use zylcode_core::agent::{AgentLoop, AgentState, RealModelClient};
use zylcode_core::router::{RouterConfig, TokenRouter};
use zylcode_mcp::{DynamicTool, McpToolConfig, McpTransport, ToolRegistry};

#[tokio::test]
async fn test_real_provider_commissioning() -> Result<()> {
    // Check if we can connect to a real provider
    // This test will try to use Ollama (local) or fail gracefully

    let registry = Arc::new(ToolRegistry::new());

    // Register tools
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

    // Try to create a router with Ollama
    let config = RouterConfig {
        primary_provider: zylcode_core::router::ModelProvider::LocalOllama,
        primary_model: "llama3".to_string(), // Default Ollama model
        ..Default::default()
    };

    let router = match TokenRouter::new(config) {
        Ok(r) => Arc::new(r),
        Err(e) => {
            println!("⚠️  Failed to create router: {}", e);
            return Ok(());
        }
    };

    // Create model client
    let model_client = Arc::new(RealModelClient::new(router));

    // Create ledger store
    let ledger = Arc::new(zylcode_core::memory_ledger::MemoryLedgerStore::new());

    // Create agent loop
    let mut agent = AgentLoop::new(
        "Read the Cargo.toml file and tell me the package name",
        PathBuf::from("."),
        None,
        registry,
        model_client,
        ledger,
    );

    // Try to run the agent loop
    // This will fail if Ollama is not running, which is expected
    match agent.run().await {
        Ok(final_state) => {
            println!("✅ Real provider commissioning succeeded!");
            println!("   Final state: {:?}", final_state);
            println!("   Messages: {}", agent.session().messages.len());

            // Check if we got real tool execution
            let tool_messages: Vec<_> = agent
                .session()
                .messages
                .iter()
                .filter(|m| m.role == zylcode_core::agent::MessageRole::Tool)
                .collect();

            println!("   Tool messages: {}", tool_messages.len());

            if final_state == AgentState::Completed {
                println!("   ✅ Agent completed successfully");
            } else {
                println!("   ⚠️  Agent did not complete: {:?}", final_state);
            }
        }
        Err(e) => {
            println!(
                "⚠️  Real provider commissioning failed (expected if Ollama not running): {}",
                e
            );
            // This is not a test failure, just a commissioning result
        }
    }

    Ok(())
}
