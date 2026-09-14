use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use zylcode_mcp::{
    EnhancedMcpBridge, SkillsSystem, PluginMarketplace,
    ExecutionContext,
};

/// Comprehensive integration test for Phase 1 implementation
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tracing::info!("Starting Phase 1 Integration Test");

    // Test 1: Enhanced MCP Bridge with 100+ tools
    test_enhanced_mcp_bridge().await?;

    // Test 2: Skills System
    test_skills_system().await?;

    // Test 3: Plugin Marketplace
    test_plugin_marketplace().await?;

    // Test 4: Integration between systems
    test_system_integration().await?;

    tracing::info!("Phase 1 Integration Test completed successfully!");
    Ok(())
}

/// Test Enhanced MCP Bridge with 100+ tools
async fn test_enhanced_mcp_bridge() -> Result<()> {
    tracing::info!("Testing Enhanced MCP Bridge...");

    let bridge = EnhancedMcpBridge::new();
    let tool_count = bridge.initialize_with_builtin_tools().await?;

    tracing::info!("Initialized MCP bridge with {} tools", tool_count);
    assert!(tool_count >= 100, "Expected at least 100 tools, got {}", tool_count);

    // Test tool categories
    let categories = bridge.get_categories().await;
    tracing::info!("Tool categories: {:?}", categories);
    assert!(categories.len() >= 8, "Expected at least 8 categories");

    // Test development tools
    let dev_tools = bridge.get_tools_in_category("development").await;
    assert!(dev_tools.is_some(), "Development category should exist");
    let dev_tools = dev_tools.unwrap();
    tracing::info!("Development tools: {} tools", dev_tools.len());
    assert!(dev_tools.len() >= 25, "Expected at least 25 development tools");

    // Test tool execution
    let params = serde_json::json!({
        "message": "test commit",
        "files": ["src/main.rs"]
    });

    let result = bridge.execute_tool("git.commit", params).await?;
    tracing::info!("Git commit result: {:?}", result);
    assert_eq!(result["tool"], "git.commit");
    assert_eq!(result["result"]["success"], true);

    // Test AI/ML tools
    let ai_params = serde_json::json!({
        "prompt": "Write a function to sort an array",
        "model": "gpt-4"
    });

    let ai_result = bridge.execute_tool("openai.complete", ai_params).await?;
    tracing::info!("OpenAI completion result: {:?}", ai_result);
    assert_eq!(ai_result["tool"], "openai.complete");

    // Test execution stats
    let (total, successful, failed, duration) = bridge.get_stats().await;
    tracing::info!("Execution stats: total={}, successful={}, failed={}, duration={}ms", 
        total, successful, failed, duration);
    assert!(total >= 2, "Expected at least 2 tool executions");

    tracing::info!("Enhanced MCP Bridge test passed!");
    Ok(())
}

/// Test Skills System
async fn test_skills_system() -> Result<()> {
    tracing::info!("Testing Skills System...");

    let system = SkillsSystem::new();
    let skill_count = system.initialize_with_builtin_skills().await?;

    tracing::info!("Initialized skills system with {} skills", skill_count);
    assert!(skill_count >= 5, "Expected at least 5 skills");

    // Test skill categories
    let categories = system.list_categories().await;
    tracing::info!("Skill categories: {:?}", categories);
    assert!(categories.contains(&"development".to_string()));
    assert!(categories.contains(&"ai-ml".to_string()));
    assert!(categories.contains(&"security".to_string()));

    // Test skill execution
    let input = serde_json::json!({
        "files": ["src/main.rs"],
        "options": {"auto_fix": false}
    });

    let context = ExecutionContext {
        user_id: Some("user123".to_string()),
        project_id: Some("project456".to_string()),
        permissions: vec!["filesystem.read".to_string()],
        environment: HashMap::new(),
    };

    let result = system.execute_skill("code-review", input, context).await?;
    tracing::info!("Code review result: {:?}", result);
    assert_eq!(result["skill"], "code-review");
    assert_eq!(result["result"]["success"], true);

    // Test skill definition
    let definition = system.get_skill_definition("code-review").await;
    assert!(definition.is_some(), "Code review skill should exist");
    let definition = definition.unwrap();
    tracing::info!("Code review skill: {} v{}", definition.name, definition.version);
    assert_eq!(definition.name, "Code Review");

    // Test execution history
    let history = system.get_execution_history(10).await;
    tracing::info!("Execution history: {} records", history.len());
    assert!(history.len() >= 1, "Expected at least 1 execution record");

    tracing::info!("Skills System test passed!");
    Ok(())
}

/// Test Plugin Marketplace
async fn test_plugin_marketplace() -> Result<()> {
    tracing::info!("Testing Plugin Marketplace...");

    let marketplace = PluginMarketplace::new();
    let plugin_count = marketplace.initialize_with_preshipped_plugins().await?;

    tracing::info!("Initialized plugin marketplace with {} plugins", plugin_count);
    assert!(plugin_count >= 5, "Expected at least 5 plugins");

    // Test plugin categories
    let categories = marketplace.list_categories().await;
    tracing::info!("Plugin categories: {:?}", categories);
    assert!(categories.contains(&"ai-models".to_string()));
    assert!(categories.contains(&"productivity".to_string()));
    assert!(categories.contains(&"development".to_string()));

    // Test plugin installation
    let config = serde_json::json!({
        "default_model": "gpt-4"
    });

    marketplace.install_plugin("ai-model-provider", config).await?;
    tracing::info!("Installed ai-model-provider plugin");

    let installed = marketplace.get_installed_plugins().await;
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].plugin_id, "ai-model-provider");

    // Test plugin execution
    let params = serde_json::json!({
        "prompt": "Explain quantum computing",
        "model": "gpt-4"
    });

    let result = marketplace.execute_plugin_command(
        "ai-model-provider",
        "complete",
        params,
    ).await?;

    tracing::info!("AI model provider result: {:?}", result);
    assert_eq!(result["plugin"], "ai-model-provider");
    assert_eq!(result["result"]["success"], true);

    // Test plugin search
    let search_results = marketplace.search_plugins("git").await;
    tracing::info!("Search results for 'git': {} plugins", search_results.len());
    assert!(search_results.len() >= 1, "Expected at least 1 git-related plugin");

    // Test marketplace stats
    let (total_plugins, total_installations, revenue, active_users) = marketplace.get_stats().await;
    tracing::info!("Marketplace stats: plugins={}, installations={}, revenue=${}, users={}", 
        total_plugins, total_installations, revenue, active_users);
    assert!(total_plugins >= 5, "Expected at least 5 total plugins");

    tracing::info!("Plugin Marketplace test passed!");
    Ok(())
}

/// Test integration between systems
async fn test_system_integration() -> Result<()> {
    tracing::info!("Testing System Integration...");

    // Initialize all systems
    let bridge = EnhancedMcpBridge::new();
    let skills = SkillsSystem::new();
    let marketplace = PluginMarketplace::new();

    let bridge_count = bridge.initialize_with_builtin_tools().await?;
    let skills_count = skills.initialize_with_builtin_skills().await?;
    let marketplace_count = marketplace.initialize_with_preshipped_plugins().await?;

    tracing::info!("Initialized systems: bridge={} tools, {} skills, {} plugins",
        bridge_count, skills_count, marketplace_count);

    // Test cross-system workflow
    tracing::info!("Testing cross-system workflow...");

    // 1. Use MCP bridge to analyze code
    let analysis_params = serde_json::json!({
        "files": ["src/main.rs"],
        "analysis_type": "complexity"
    });

    let analysis_result = bridge.execute_tool("eslint.lint", analysis_params).await?;
    tracing::info!("Code analysis result: {:?}", analysis_result);

    // 2. Use skills system to review code
    let review_input = serde_json::json!({
        "files": ["src/main.rs"],
        "options": {"auto_fix": false}
    });

    let review_context = ExecutionContext {
        user_id: Some("user123".to_string()),
        project_id: Some("project456".to_string()),
        permissions: vec!["filesystem.read".to_string()],
        environment: HashMap::new(),
    };

    let review_result = skills.execute_skill("code-review", review_input, review_context).await?;
    tracing::info!("Code review result: {:?}", review_result);

    // 3. Use plugin marketplace to get AI assistance
    let ai_params = serde_json::json!({
        "prompt": "Suggest improvements for this code",
        "context": review_result
    });

    let ai_result = marketplace.execute_plugin_command(
        "ai-model-provider",
        "suggest",
        ai_params,
    ).await?;

    tracing::info!("AI suggestion result: {:?}", ai_result);

    // Test system statistics
    let (bridge_total, bridge_successful, bridge_failed, bridge_duration) = bridge.get_stats().await;
    let (marketplace_plugins, marketplace_installations, marketplace_revenue, marketplace_users) = marketplace.get_stats().await;

    tracing::info!("System Statistics:");
    tracing::info!("  MCP Bridge: {} total calls, {} successful, {} failed, {}ms total", 
        bridge_total, bridge_successful, bridge_failed, bridge_duration);
    tracing::info!("  Marketplace: {} plugins, {} installations, ${} revenue, {} users",
        marketplace_plugins, marketplace_installations, marketplace_revenue, marketplace_users);

    // Verify all systems are working
    assert!(bridge_total >= 2, "Expected at least 2 bridge calls");
    assert!(marketplace_installations >= 1, "Expected at least 1 plugin installation");

    tracing::info!("System Integration test passed!");
    Ok(())
}