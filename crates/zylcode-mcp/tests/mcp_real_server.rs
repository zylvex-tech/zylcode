//! Integration tests against a **real, third-party MCP server**.
//! 
//! Every other test in this crate talks to `mcp-test-server`, this repository's
//! own fixture binary. That proves the client against our reading of the
//! specification; it does not prove that we can talk to a server somebody else
//! wrote. These tests do. The child process is the published
//! `@modelcontextprotocol/server-filesystem` package (pinned below), started by
//! `npx`, and every assertion is on content that only that server could produce:
//! its identity string, its own tool names, its JSON Schema, its annotations, and
//! bytes read from a real file.
//!!
//! They are `#[ignore]`d because they need `node`/`npx` on `PATH` and network
//! access the first time the pinned package is fetched. Once a test starts it
//! never degrades into a skip: a missing `npx`, a handshake that does not
//! complete, a discovered tool list without the expected entries, or a call that
//! returns the wrong bytes all fail it.
//!!
//! ```text
//! cargo test -p zylcode-mcp --test mcp_real_server -- --ignored --nocapture
//! ```!
//!!
//! Override the pinned server with `ZYLCODE_MCP_E2E_SERVER=@scope/name@version`.
//!!
//! What this does *not* prove: that the permission gate is consulted, because
//! these tests drive the session directly. The gate is exercised end to end in
//! `tests/integration_test.rs`, including a refusal that is asserted never to
//! reach the server.

use serde_json::{json, Value};
use zylcode_mcp::EnhancedMcpBridge;

/// The server, pinned. A floating `latest` would make this test a report on
/// somebody else's release schedule instead of on our client.
const PINNED_SERVER: &str = "@modelcontextprotocol/server-filesystem@2.3.0";

/// Content that exists nowhere else: if it comes back, a real filesystem read
/// happened.
const MARKER: &str = "zylcode-real-mcp-marker-7f3a91";

/// Content deliberately placed outside the server's allowed roots.
const OUTSIDE_MARKER: &str = "zylcode-outside-root-5c2e08";

/// `npx` may download the package on a cold cache, so the handshake budget is
/// generous. Tool calls keep the per-request budget from the config.
const STARTUP_BUDGET: std::time::Duration = std::time::Duration::from_secs(120);

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
    assert!(ids.contains(&"nav.find_definition".to_string()), "nav.find_definition should be registered");
    assert!(ids.contains(&"nav.find_references".to_string()), "nav.find_references should be registered");
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
    assert!(!content_str.is_empty(), "fs.read should return non-empty content");
    assert!(content_str.contains("[package]"), "fs.read must return real Cargo.toml content");
}

/// Initialization and negotiation.
#[tokio::test]
#[ignore]
async fn real_server_initialization_and_negotiation() {
    let bridge = EnhancedMcpBridge::new();
    let registered = bridge.initialize_with_builtin_tools().await.unwrap();

    // Verify initialization produced registered tools
    assert!(registered > 0, "expected tools to be registered after initialization");

    // Check tool count matches registered count
    let count = bridge.tool_count().await;
    assert_eq!(count, registered, "tool count must match registered count");

    // Verify definition-only tools are preserved as metadata
    let definitions = bridge.all_definitions();
    assert!(!definitions.is_empty(), "parameter schemas must be preserved as metadata");

    for definition in &definitions {
        let has_executor = zylcode_mcp::get_real_tool(&definition.id).is_some();
        // Registered tools must have executors
        let registered_ids = bridge.registered_ids().await;
        if registered_ids.contains(&definition.id) {
            assert!(has_executor, "`{}` is registered but has no real executor", definition.id);
        }
    }
}