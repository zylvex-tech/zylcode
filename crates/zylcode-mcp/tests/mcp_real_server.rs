//! Integration tests for the MCP bridge surface.
//!
//! These tests drive [`EnhancedMcpBridge`] directly rather than a third-party
//! server: initialization must register the built-in tool set, the navigation
//! tools must be present, `fs.read` must return the real contents of
//! `Cargo.toml`, and every registered definition must have a real executor.
//!
//! They are `#[ignore]`d because they assume the crate is run from the
//! workspace root, where `Cargo.toml` exists.
//!
//! ```text
//! cargo test -p zylcode-mcp --test mcp_real_server -- --ignored --nocapture
//! ```

use zylcode_mcp::EnhancedMcpBridge;
/// Every tool the bridge registers must have a real executor.
#[tokio::test]
#[ignore]
async fn real_server_initialize_and_tools_list() {
    let bridge = EnhancedMcpBridge::new();
    let count = bridge.initialize_with_builtin_tools().await.unwrap();

    // Verify the bridge has tools registered
    assert!(count > 0, "expected built-in tools to register");

    // Check that navigation tools are registered
    let ids = bridge.registered_ids().await;
    assert!(
        ids.contains(&"nav.find_definition".to_string()),
        "nav.find_definition should be registered"
    );
    assert!(
        ids.contains(&"nav.find_references".to_string()),
        "nav.find_references should be registered"
    );
}

/// A real, read-only execution actually reads a file.
#[tokio::test]
#[ignore]
async fn real_server_file_read() {
    let bridge = EnhancedMcpBridge::new();
    bridge.initialize_with_builtin_tools().await.unwrap();

    // Execute fs.read on Cargo.toml - this is a real execution against the real file
    let result = bridge
        .execute_tool("fs.read", serde_json::json!({ "path": "Cargo.toml" }))
        .await;

    assert!(result.is_ok(), "fs.read should succeed for existing file");
    let content = result.unwrap();
    let content_str = content
        .get("output")
        .and_then(|o| o.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("");

    // Verify we got actual file content (not fabricated)
    assert!(
        !content_str.is_empty(),
        "fs.read should return non-empty content"
    );
    assert!(
        content_str.contains("[package]"),
        "fs.read must return real Cargo.toml content"
    );
}

/// Initialization and negotiation.
#[tokio::test]
#[ignore]
async fn real_server_initialization_and_negotiation() {
    let bridge = EnhancedMcpBridge::new();
    let registered = bridge.initialize_with_builtin_tools().await.unwrap();

    // Verify initialization produced registered tools
    assert!(
        registered > 0,
        "expected tools to be registered after initialization"
    );

    // Check tool count matches registered count
    let count = bridge.tool_count().await;
    assert_eq!(count, registered, "tool count must match registered count");

    // Verify definition-only tools are preserved as metadata
    let definitions = bridge.all_definitions();
    assert!(
        !definitions.is_empty(),
        "parameter schemas must be preserved as metadata"
    );

    for definition in &definitions {
        let has_executor = zylcode_mcp::get_real_tool(&definition.id).is_some();
        // Registered tools must have executors
        let registered_ids = bridge.registered_ids().await;
        if registered_ids.contains(&definition.id) {
            assert!(
                has_executor,
                "`{}` is registered but has no real executor",
                definition.id
            );
        }
    }
}
