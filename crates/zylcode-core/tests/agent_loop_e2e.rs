//! End-to-end test for the Agent Loop

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use zylcode_core::agent::{AgentLoop, AgentState};
use zylcode_mcp::{ToolRegistry, DynamicTool, McpToolConfig, McpTransport};

#[tokio::test]
async fn test_agent_loop_end_to_end() -> Result<()> {
    // Create a tool registry
    let registry = Arc::new(ToolRegistry::new());
    
    // Register real tools
    let fs_read_config = McpToolConfig {
        id: "fs.read".to_string(),
        command: "read".to_string(),
        transport: McpTransport::Stdio,
        env: Default::default(),
        enabled: true,
        description: Some("Read file contents".to_string()),
    };
    registry.register(Arc::new(DynamicTool::new(fs_read_config))).await;
    
    let shell_config = McpToolConfig {
        id: "shell.execute".to_string(),
        command: "execute".to_string(),
        transport: McpTransport::Stdio,
        env: Default::default(),
        enabled: true,
        description: Some("Execute shell command".to_string()),
    };
    registry.register(Arc::new(DynamicTool::new(shell_config))).await;
    
    // Verify tools are registered
    let tools = registry.list().await;
    println!("Registered tools: {:?}", tools);
    
    // Create an agent loop
    let mut agent = AgentLoop::new(
        "Read Cargo.toml and echo completion message",
        PathBuf::from("."),
        None,
        registry,
    );
    
    // Run the agent loop
    let final_state = agent.run().await?;
    
    // Verify the final state
    assert_eq!(final_state, AgentState::Completed);
    
    // Verify that files were read
    assert!(!agent.session().context.files_read.is_empty());
    
    // Verify that commands were executed
    assert!(!agent.session().context.commands_executed.is_empty());
    
    // Verify that messages were recorded
    assert!(agent.session().messages.len() > 1);
    
    println!("✅ Agent loop completed successfully!");
    println!("   Files read: {:?}", agent.session().context.files_read);
    println!("   Commands executed: {:?}", agent.session().context.commands_executed.len());
    println!("   Messages recorded: {}", agent.session().messages.len());
    
    Ok(())
}

#[tokio::test]
async fn test_agent_loop_with_tool_execution() -> Result<()> {
    // Create a tool registry
    let registry = Arc::new(ToolRegistry::new());
    
    // Register real tools
    let fs_read_config = McpToolConfig {
        id: "fs.read".to_string(),
        command: "read".to_string(),
        transport: McpTransport::Stdio,
        env: Default::default(),
        enabled: true,
        description: Some("Read file contents".to_string()),
    };
    registry.register(Arc::new(DynamicTool::new(fs_read_config))).await;
    
    let shell_config = McpToolConfig {
        id: "shell.execute".to_string(),
        command: "execute".to_string(),
        transport: McpTransport::Stdio,
        env: Default::default(),
        enabled: true,
        description: Some("Execute shell command".to_string()),
    };
    registry.register(Arc::new(DynamicTool::new(shell_config))).await;
    
    // Create an agent loop with a specific task
    let mut agent = AgentLoop::new(
        "Execute echo command and read Cargo.toml",
        PathBuf::from("."),
        None,
        registry,
    );
    
    // Run the agent loop
    let final_state = agent.run().await?;
    
    // Verify the final state
    assert_eq!(final_state, AgentState::Completed);
    
    // Verify that we have tool results in messages
    let tool_messages: Vec<_> = agent.session().messages.iter()
        .filter(|m| m.role == zylcode_core::agent::MessageRole::Tool)
        .collect();
    
    assert!(!tool_messages.is_empty(), "Should have tool result messages");
    
    println!("✅ Agent loop with tool execution completed successfully!");
    println!("   Tool messages: {}", tool_messages.len());
    
    Ok(())
}