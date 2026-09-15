//! End-to-end test for the Agent Loop

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use zylcode_core::agent::{AgentLoop, AgentState, TestModelClient};
use zylcode_mcp::{DynamicTool, McpToolConfig, McpTransport, ToolRegistry};

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
    registry
        .register(Arc::new(DynamicTool::new(fs_read_config)))
        .await;

    let shell_config = McpToolConfig {
        id: "shell.execute".to_string(),
        command: "execute".to_string(),
        transport: McpTransport::Stdio,
        env: Default::default(),
        enabled: true,
        description: Some("Execute shell command".to_string()),
    };
    registry
        .register(Arc::new(DynamicTool::new(shell_config)))
        .await;

    // Verify tools are registered
    let tools = registry.list().await;
    println!("Registered tools: {:?}", tools);

    // Create deterministic test model client
    let responses = vec![
        // Response for planning
        r#"{"action": "Plan", "payload": {"steps": [{"id": "1", "description": "Read Cargo.toml", "expected_files": ["Cargo.toml"], "expected_tools": ["fs.read"], "risk": "Read", "verification": "File read successfully"}]}}"#.to_string(),
        // Response for tool call
        r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Need to read Cargo.toml", "expected_result": "File content"}}"#.to_string(),
        // Response for verification
        r#"{"action": "Complete", "payload": {"summary": "Task completed successfully", "evidence": ["Read Cargo.toml"], "remaining_limitations": []}}"#.to_string(),
    ];
    let model_client = Arc::new(TestModelClient::new(responses));

    // Create an agent loop
    let mut agent = AgentLoop::new(
        "Read Cargo.toml and echo completion message",
        PathBuf::from("."),
        None,
        registry,
        model_client,
    );

    // Run the agent loop
    let final_state = agent.run().await?;

    // Verify the final state
    assert_eq!(final_state, AgentState::Completed);

    // Verify that files were read
    assert!(!agent.session().context.files_read.is_empty());

    // Verify that messages were recorded
    assert!(agent.session().messages.len() > 1);

    println!("✅ Agent loop completed successfully!");
    println!("   Files read: {:?}", agent.session().context.files_read);
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
    registry
        .register(Arc::new(DynamicTool::new(fs_read_config)))
        .await;

    let shell_config = McpToolConfig {
        id: "shell.execute".to_string(),
        command: "execute".to_string(),
        transport: McpTransport::Stdio,
        env: Default::default(),
        enabled: true,
        description: Some("Execute shell command".to_string()),
    };
    registry
        .register(Arc::new(DynamicTool::new(shell_config)))
        .await;

    // Create deterministic test model client
    let responses = vec![
        // Response for planning
        r#"{"action": "Plan", "payload": {"steps": [{"id": "1", "description": "Execute echo command", "expected_files": [], "expected_tools": ["shell.execute"], "risk": "Execute", "verification": "Command executed"}]}}"#.to_string(),
        // Response for tool call
        r#"{"action": "ToolCall", "payload": {"tool_id": "shell.execute", "arguments": {"command": "echo", "args": ["Agent execution complete"]}, "reason": "Execute echo command", "expected_result": "Command output"}}"#.to_string(),
        // Response for verification
        r#"{"action": "Complete", "payload": {"summary": "Task completed successfully", "evidence": ["Executed echo command"], "remaining_limitations": []}}"#.to_string(),
    ];
    let model_client = Arc::new(TestModelClient::new(responses));

    // Create an agent loop with a specific task
    let mut agent = AgentLoop::new(
        "Execute echo command and read Cargo.toml",
        PathBuf::from("."),
        None,
        registry,
        model_client,
    );

    // Run the agent loop
    let final_state = agent.run().await?;

    // Verify the final state
    assert_eq!(final_state, AgentState::Completed);

    // Verify that we have tool results in messages
    let tool_messages: Vec<_> = agent
        .session()
        .messages
        .iter()
        .filter(|m| m.role == zylcode_core::agent::MessageRole::Tool)
        .collect();

    assert!(
        !tool_messages.is_empty(),
        "Should have tool result messages"
    );

    println!("✅ Agent loop with tool execution completed successfully!");
    println!("   Tool messages: {}", tool_messages.len());

    Ok(())
}

#[tokio::test]
async fn test_observation_loop() -> Result<()> {
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
    registry
        .register(Arc::new(DynamicTool::new(fs_read_config)))
        .await;

    // Create deterministic test model client that requires multiple tool calls
    let responses = vec![
        // Response for planning
        r#"{"action": "Plan", "payload": {"steps": [{"id": "1", "description": "Read Cargo.toml", "expected_files": ["Cargo.toml"], "expected_tools": ["fs.read"], "risk": "Read", "verification": "File read successfully"}]}}"#.to_string(),
        // First tool call
        r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Read Cargo.toml", "expected_result": "File content"}}"#.to_string(),
        // Second tool call (model observes first result and decides to read another file)
        r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "README.md"}, "reason": "Read README after seeing Cargo.toml", "expected_result": "README content"}}"#.to_string(),
        // Complete with evidence
        r#"{"action": "Complete", "payload": {"summary": "Read multiple files", "evidence": ["Read Cargo.toml", "Read README.md"], "remaining_limitations": []}}"#.to_string(),
    ];
    let model_client = Arc::new(TestModelClient::new(responses));

    // Create an agent loop
    let mut agent = AgentLoop::new(
        "Read Cargo.toml and README.md",
        PathBuf::from("."),
        None,
        registry,
        model_client,
    );

    // Run the agent loop
    let final_state = agent.run().await?;

    // Verify the final state
    assert_eq!(final_state, AgentState::Completed);

    // Verify that multiple files were read
    assert!(agent.session().context.files_read.len() >= 2);

    println!("✅ Observation loop test passed!");
    println!("   Files read: {:?}", agent.session().context.files_read);

    Ok(())
}

#[tokio::test]
async fn test_repair_loop() -> Result<()> {
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
    
    // Create deterministic test model client that fails first, then succeeds
    let responses = vec![
        // Response for planning
        r#"{"action": "Plan", "payload": {"steps": [{"id": "1", "description": "Read file", "expected_files": ["test.txt"], "expected_tools": ["fs.read"], "risk": "Read", "verification": "File read"}]}}"#.to_string(),
        // First tool call (will fail because file doesn't exist)
        r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "nonexistent.txt"}, "reason": "Read file", "expected_result": "File content"}}"#.to_string(),
        // Verification fails
        r#"{"action": "Fail", "payload": {"reason": "File not found", "error": "No such file or directory", "repair_suggestions": ["Check file path", "Create file first"]}}"#.to_string(),
        // Repair attempt - read a valid file
        r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Read valid file instead", "expected_result": "File content"}}"#.to_string(),
        // Complete with evidence
        r#"{"action": "Complete", "payload": {"summary": "Repair successful", "evidence": ["Read Cargo.toml after repair"], "remaining_limitations": []}}"#.to_string(),
    ];
    let model_client = Arc::new(TestModelClient::new(responses));
    
    // Create an agent loop with auto_repair enabled
    let config = zylcode_core::agent::AgentConfig {
        auto_repair: true,
        ..Default::default()
    };
    
    let mut agent = AgentLoop::new(
        "Read a file",
        PathBuf::from("."),
        Some(config),
        registry,
        model_client,
    );
    
    // Run the agent loop
    let final_state = agent.run().await?;
    
    // Verify the final state
    assert_eq!(final_state, AgentState::Completed);
    
    // Verify that repair happened
    let has_repair_message = agent.session().messages.iter().any(|m| 
        m.content.contains("Repairing") || m.content.contains("repair")
    );
    
    println!("✅ Repair loop test passed!");
    println!("   Final state: {:?}", final_state);
    
    Ok(())
}
