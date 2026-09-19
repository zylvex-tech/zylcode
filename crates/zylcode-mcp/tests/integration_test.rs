//! Cross-system integration tests for `zylcode-mcp`.
//!
//! # Why this file was rewritten
//!
//! It previously declared `#[tokio::main] async fn main()` and no `#[test]`
//! functions. `cargo test` compiles integration files with the test harness,
//! which takes over `main` and registers nothing:
//!
//! ```text
//! $ cargo test -p zylcode-mcp --test integration_test -- --list
//! 0 tests, 0 benchmarks
//! ```
//!
//! Every assertion in it — including `tool_count >= 100` and
//! `dev_tools.len() >= 25` — had therefore never executed. An assertion that
//! cannot fire is worse than a failing one: it looks like coverage and
//! enforces nothing.
//!
//! The assertions below are semantic invariants, not quantity targets. Tool
//! quantity is not a correctness invariant; see
//! `docs/governance/TOOL_CATALOGUE_TRUTH_TABLE.md`.

use std::collections::HashMap;

use zylcode_mcp::skills_system::ExecutionContext;
use zylcode_mcp::{EnhancedMcpBridge, PluginMarketplace, SkillsSystem};

fn execution_context() -> ExecutionContext {
    ExecutionContext {
        user_id: Some("user123".to_string()),
        project_id: Some("project456".to_string()),
        permissions: vec!["filesystem.read".to_string()],
        environment: HashMap::new(),
    }
}

// ---------------------------------------------------------------------------
// Enhanced MCP Bridge — catalogue / executor correspondence
// ---------------------------------------------------------------------------

/// Every tool the bridge registers must have a real executor.
#[tokio::test]
async fn bridge_registers_only_tools_with_real_executors() {
    let bridge = EnhancedMcpBridge::new();
    let registered = bridge.initialize_with_builtin_tools().await.unwrap();

    assert_eq!(bridge.tool_count().await, registered);

    let ids = bridge.registered_ids().await;
    assert_eq!(ids.len(), registered, "count must match the registry");
    for id in &ids {
        assert!(
            zylcode_mcp::get_real_tool(id).is_some(),
            "`{id}` is registered but has no real executor"
        );
    }
}

/// Definition-only entries are preserved as metadata and are never registered.
#[tokio::test]
async fn bridge_keeps_definitions_as_metadata_only() {
    let bridge = EnhancedMcpBridge::new();
    bridge.initialize_with_builtin_tools().await.unwrap();

    let registered: std::collections::HashSet<String> =
        bridge.registered_ids().await.into_iter().collect();

    let definitions = bridge.all_definitions();
    assert!(
        !definitions.is_empty(),
        "parameter schemas must be preserved as metadata"
    );

    for definition in &definitions {
        let has_executor = zylcode_mcp::get_real_tool(&definition.id).is_some();
        assert_eq!(
            registered.contains(&definition.id),
            has_executor,
            "`{}`: registered={} but has_executor={}",
            definition.id,
            registered.contains(&definition.id),
            has_executor
        );
        assert!(
            definition.parameters.is_object(),
            "`{}` lost its parameter schema",
            definition.id
        );
    }
}

/// Nothing reports success for work it did not perform.
#[tokio::test]
async fn bridge_never_fabricates_success_for_unsupported_tools() {
    let bridge = EnhancedMcpBridge::new();
    bridge.initialize_with_builtin_tools().await.unwrap();

    for id in [
        "openai.complete",
        "docker.build",
        "kubernetes.deploy",
        "eslint.lint",
    ] {
        let outcome = bridge
            .execute_tool(id, serde_json::json!({ "prompt": "anything" }))
            .await;
        assert!(
            outcome.is_err(),
            "`{id}` has no executor and must fail closed, not report success"
        );
    }
}

/// Unknown ids fail closed.
#[tokio::test]
async fn bridge_unknown_tool_id_fails_closed() {
    let bridge = EnhancedMcpBridge::new();
    bridge.initialize_with_builtin_tools().await.unwrap();
    assert!(bridge
        .execute_tool("no.such.tool", serde_json::json!({}))
        .await
        .is_err());
}

/// A real, read-only execution actually reads a file.
///
/// `git.commit` is deliberately *not* exercised here: it would create a real
/// commit in the working repository. Binding behaviour is covered by the
/// adversarial tests below and in `real_tools::tests`.
#[tokio::test]
async fn bridge_executes_a_real_read_only_tool() {
    let bridge = EnhancedMcpBridge::new();
    bridge.initialize_with_builtin_tools().await.unwrap();

    let result = bridge
        .execute_tool("fs.read", serde_json::json!({ "path": "Cargo.toml" }))
        .await
        .expect("fs.read has a real executor");

    assert_eq!(result["tool"], "fs.read");
    assert_eq!(result["executed"], true);
    let content = result["output"]["content"]
        .as_str()
        .expect("fs.read returns file content");
    assert!(
        content.contains("[package]"),
        "fs.read must return the real file, got: {content:.80}"
    );
}

/// The dispatch binding holds through the bridge: `git.commit` cannot push.
///
/// A permissive gate is used deliberately. The gate is consulted *before* the
/// executor, so the restrictive default would mask the binding violation with a
/// permission denial. Both refusals are correct; this test is about the binding.
#[tokio::test]
async fn bridge_git_commit_cannot_execute_push() {
    let bridge =
        EnhancedMcpBridge::with_runtime(zylcode_mcp::ToolRuntime::permissive_without_evidence());
    bridge.initialize_with_builtin_tools().await.unwrap();

    let err = bridge
        .execute_tool("git.commit", serde_json::json!({ "subcommand": "push" }))
        .await
        .expect_err("git.commit must not run git push");
    assert!(
        err.to_string().contains("may not perform"),
        "expected a binding refusal, got: {err}"
    );
}

/// The default gate refuses an unapproved write-class tool.
#[tokio::test]
async fn bridge_default_gate_refuses_unapproved_write() {
    let bridge = EnhancedMcpBridge::new();
    bridge.initialize_with_builtin_tools().await.unwrap();

    let err = bridge
        .execute_tool(
            "git.commit",
            serde_json::json!({ "message": "must not run" }),
        )
        .await
        .expect_err("an unapproved GitWrite tool must not run");
    assert!(
        err.to_string().contains("permission denied"),
        "expected a permission refusal, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// Skills System
// ---------------------------------------------------------------------------

#[tokio::test]
async fn skills_system_initialises_and_executes() {
    let system = SkillsSystem::new();
    let skill_count = system.initialize_with_builtin_skills().await.unwrap();
    assert!(skill_count > 0, "expected the builtin skills to load");

    let categories = system.list_categories().await;
    for expected in ["development", "ai-ml", "security"] {
        assert!(
            categories.contains(&expected.to_string()),
            "missing skill category `{expected}`"
        );
    }

    let input = serde_json::json!({
        "files": ["src/main.rs"],
        "options": {"auto_fix": false}
    });
    let result = system
        .execute_skill("code-review", input, execution_context())
        .await
        .unwrap();
    assert_eq!(result["skill"], "code-review");

    let definition = system
        .get_skill_definition("code-review")
        .await
        .expect("code-review skill is defined");
    assert_eq!(definition.name, "Code Review");

    let history = system.get_execution_history(10).await;
    assert!(!history.is_empty(), "executing a skill must record history");
}

// ---------------------------------------------------------------------------
// Plugin Marketplace
// ---------------------------------------------------------------------------

#[tokio::test]
async fn plugin_marketplace_installs_and_executes() {
    let marketplace = PluginMarketplace::new();
    let plugin_count = marketplace
        .initialize_with_preshipped_plugins()
        .await
        .unwrap();
    assert!(plugin_count > 0, "expected the preshipped plugins to load");

    let categories = marketplace.list_categories().await;
    for expected in ["ai-models", "productivity", "development"] {
        assert!(
            categories.contains(&expected.to_string()),
            "missing plugin category `{expected}`"
        );
    }

    marketplace
        .install_plugin(
            "ai-model-provider",
            serde_json::json!({ "default_model": "gpt-4" }),
        )
        .await
        .unwrap();

    let installed = marketplace.get_installed_plugins().await;
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].plugin_id, "ai-model-provider");

    let result = marketplace
        .execute_plugin_command(
            "ai-model-provider",
            "complete",
            serde_json::json!({ "prompt": "Explain quantum computing", "model": "gpt-4" }),
        )
        .await
        .unwrap();
    assert_eq!(result["plugin"], "ai-model-provider");

    let search_results = marketplace.search_plugins("git").await;
    assert!(!search_results.is_empty(), "expected a git-related plugin");

    let (total_plugins, _installations, _revenue, _users) = marketplace.get_stats().await;
    assert!(total_plugins > 0);
}

// ---------------------------------------------------------------------------
// Cross-system
// ---------------------------------------------------------------------------

#[tokio::test]
async fn systems_compose_without_fabricated_success() {
    let bridge = EnhancedMcpBridge::new();
    let skills = SkillsSystem::new();
    let marketplace = PluginMarketplace::new();

    let bridge_count = bridge.initialize_with_builtin_tools().await.unwrap();
    let skills_count = skills.initialize_with_builtin_skills().await.unwrap();
    let marketplace_count = marketplace
        .initialize_with_preshipped_plugins()
        .await
        .unwrap();

    assert_eq!(bridge.tool_count().await, bridge_count);
    assert!(skills_count > 0);
    assert!(marketplace_count > 0);

    // 1. A real read through the tool registry.
    bridge
        .execute_tool("fs.read", serde_json::json!({ "path": "Cargo.toml" }))
        .await
        .expect("fs.read executes for real");

    // 2. A definition-only tool fails closed rather than inventing a result.
    assert!(
        bridge
            .execute_tool(
                "eslint.lint",
                serde_json::json!({ "files": ["src/main.rs"] })
            )
            .await
            .is_err(),
        "a definition-only tool must not fabricate a result"
    );

    // 3. The skills system composes with the same execution context.
    let review = skills
        .execute_skill(
            "code-review",
            serde_json::json!({ "files": ["src/main.rs"], "options": {"auto_fix": false} }),
            execution_context(),
        )
        .await
        .unwrap();
    assert_eq!(review["skill"], "code-review");

    // 4. The marketplace composes on top of that result.
    marketplace
        .install_plugin(
            "ai-model-provider",
            serde_json::json!({ "default_model": "gpt-4" }),
        )
        .await
        .unwrap();
    let suggestion = marketplace
        .execute_plugin_command(
            "ai-model-provider",
            "suggest",
            serde_json::json!({ "prompt": "Suggest improvements", "context": review }),
        )
        .await
        .unwrap();
    assert_eq!(suggestion["plugin"], "ai-model-provider");

    // 5. Execution statistics reflect real attempts.
    let (total, successful, _failed, _duration) = bridge.get_stats().await;
    assert!(total >= 2, "expected the bridge to record its calls");
    assert!(
        successful >= 1,
        "the successful read must be counted as a success"
    );

    let (plugins, installations, _revenue, _users) = marketplace.get_stats().await;
    assert!(plugins > 0);
    assert!(installations >= 1);
}
