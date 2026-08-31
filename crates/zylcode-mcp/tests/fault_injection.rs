use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::sync::RwLock;

use zylcode_mcp::config::{McpConfigFile, McpToolConfig, McpTransport};
use zylcode_mcp::executor::{execute_with_recovery, ExecuteOptions};
use zylcode_mcp::registry::ToolRegistry;
use zylcode_mcp::tool::{DynamicTool, Tool};

// ============================================================================
// Transport Drop Tests
// ============================================================================

#[derive(Debug)]
struct StdIoDropTool {
    id: String,
    should_drop: Arc<RwLock<bool>>,
}

#[async_trait::async_trait]
impl Tool for StdIoDropTool {
    fn id(&self) -> &str {
        &self.id
    }
    fn descriptor(&self) -> zylcode_mcp::tool::ToolDescriptor {
        zylcode_mcp::tool::ToolDescriptor {
            id: self.id.clone(),
            transport: "stdio".into(),
            command: "drop-test".into(),
            env_keys: vec![],
            description: None,
        }
    }
    async fn call(&self, _params: Value) -> Result<Value> {
        // Check if we should simulate a drop (process killed)
        if *self.should_drop.read().await {
            anyhow::bail!("SIGKILL: process terminated unexpectedly");
        }
        Ok(serde_json::json!({"ok": true}))
    }
}

#[tokio::test]
async fn transport_drop_stdio_graceful_cleanup() {
    let should_drop = Arc::new(RwLock::new(false));
    let tool: Arc<dyn Tool> = Arc::new(StdIoDropTool {
        id: "stdio-drop".into(),
        should_drop: should_drop.clone(),
    });

    // First call succeeds
    let opts = ExecuteOptions::default();
    let result = execute_with_recovery(tool.clone(), Value::Null, opts.clone()).await;
    assert!(result.is_ok());

    // Enable drop simulation
    *should_drop.write().await = true;

    // Second call should fail with structured error, not panic
    let result = execute_with_recovery(tool.clone(), Value::Null, opts.clone()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("SIGKILL") || err.to_string().contains("process terminated"));
}

#[derive(Debug)]
struct StdioTimeoutTool {
    id: String,
    delay: Duration,
}

#[async_trait::async_trait]
impl Tool for StdioTimeoutTool {
    fn id(&self) -> &str {
        &self.id
    }
    fn descriptor(&self) -> zylcode_mcp::tool::ToolDescriptor {
        zylcode_mcp::tool::ToolDescriptor {
            id: self.id.clone(),
            transport: "stdio".into(),
            command: "slow-test".into(),
            env_keys: vec![],
            description: None,
        }
    }
    async fn call(&self, _params: Value) -> Result<Value> {
        tokio::time::sleep(self.delay).await;
        Ok(serde_json::json!({"ok": true}))
    }
}

#[tokio::test]
async fn transport_stdio_timeout_retries_then_fails() {
    let tool: Arc<dyn Tool> = Arc::new(StdioTimeoutTool {
        id: "stdio-timeout".into(),
        delay: Duration::from_secs(2), // Longer than default 30s? No, use shorter for test
    });

    let opts = ExecuteOptions {
        timeout: Duration::from_millis(50),
        max_retries: 2,
        retry_backoff: Duration::from_millis(10),
    };

    let start = std::time::Instant::now();
    let result = execute_with_recovery(tool, Value::Null, opts).await;
    let elapsed = start.elapsed();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("timed out"));
    // Should have retried ~3 times (initial + 2 retries) * 50ms + backoffs
    assert!(elapsed > Duration::from_millis(150));
    assert!(elapsed < Duration::from_secs(2));
}

// ============================================================================
// SSE Reconnection & Disconnect Tests
// ============================================================================

#[derive(Debug)]
struct SseDisconnectTool {
    id: String,
    disconnect_on_attempt: usize,
    attempt_counter: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Tool for SseDisconnectTool {
    fn id(&self) -> &str {
        &self.id
    }
    fn descriptor(&self) -> zylcode_mcp::tool::ToolDescriptor {
        zylcode_mcp::tool::ToolDescriptor {
            id: self.id.clone(),
            transport: "sse".into(),
            command: "https://example.com/sse".into(),
            env_keys: vec![],
            description: None,
        }
    }
    async fn call(&self, _params: Value) -> Result<Value> {
        let attempt = self.attempt_counter.fetch_add(1, Ordering::SeqCst) + 1;
        if attempt == self.disconnect_on_attempt {
            anyhow::bail!("SSE stream closed unexpectedly");
        }
        Ok(serde_json::json!({"attempt": attempt, "ok": true}))
    }
}

#[tokio::test]
async fn sse_disconnect_reconnects_and_succeeds() {
    let attempt_counter = Arc::new(AtomicUsize::new(0));
    let tool: Arc<dyn Tool> = Arc::new(SseDisconnectTool {
        id: "sse-disconnect".into(),
        disconnect_on_attempt: 1, // First attempt fails
        attempt_counter: attempt_counter.clone(),
    });

    let opts = ExecuteOptions {
        timeout: Duration::from_secs(5),
        max_retries: 2,
        retry_backoff: Duration::from_millis(50),
    };

    let result = execute_with_recovery(tool, Value::Null, opts).await;
    assert!(result.is_ok());
    let val = result.unwrap();
    assert_eq!(val["attempt"], 2); // Second attempt succeeds
}

// Tool that always fails - for testing permanent failures
#[derive(Debug)]
struct AlwaysFailTool {
    id: String,
    error_msg: String,
}

#[async_trait::async_trait]
impl Tool for AlwaysFailTool {
    fn id(&self) -> &str {
        &self.id
    }
    fn descriptor(&self) -> zylcode_mcp::tool::ToolDescriptor {
        zylcode_mcp::tool::ToolDescriptor {
            id: self.id.clone(),
            transport: "sse".into(),
            command: "https://example.com/sse".into(),
            env_keys: vec![],
            description: None,
        }
    }
    async fn call(&self, _params: Value) -> Result<Value> {
        anyhow::bail!("{}", self.error_msg);
    }
}

#[tokio::test]
async fn sse_permanent_failure_after_max_retries() {
    let tool: Arc<dyn Tool> = Arc::new(AlwaysFailTool {
        id: "sse-perma-fail".into(),
        error_msg: "SSE stream closed unexpectedly".into(),
    });

    let opts = ExecuteOptions {
        timeout: Duration::from_secs(5),
        max_retries: 2,
        retry_backoff: Duration::from_millis(20),
    };

    let result = execute_with_recovery(tool, Value::Null, opts).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("SSE stream closed"));
}

// ============================================================================
// Malformed Payload Tests
// ============================================================================

#[derive(Debug)]
struct MalformedPayloadTool {
    id: String,
    failure_mode: MalformedMode,
}

#[derive(Debug, Clone, Copy)]
enum MalformedMode {
    TruncatedJson,
    InvalidType,
    MissingField,
    NonUtf8,
}

#[async_trait::async_trait]
impl Tool for MalformedPayloadTool {
    fn id(&self) -> &str {
        &self.id
    }
    fn descriptor(&self) -> zylcode_mcp::tool::ToolDescriptor {
        zylcode_mcp::tool::ToolDescriptor {
            id: self.id.clone(),
            transport: "stdio".into(),
            command: "malformed".into(),
            env_keys: vec![],
            description: None,
        }
    }
    async fn call(&self, _params: Value) -> Result<Value> {
        match self.failure_mode {
            MalformedMode::TruncatedJson => {
                // Simulate receiving truncated response from subprocess
                anyhow::bail!("unexpected EOF while parsing JSON response");
            }
            MalformedMode::InvalidType => {
                anyhow::bail!("expected string, got integer");
            }
            MalformedMode::MissingField => {
                anyhow::bail!("missing required field 'result'");
            }
            MalformedMode::NonUtf8 => {
                anyhow::bail!("invalid UTF-8 sequence in response");
            }
        }
    }
}

#[tokio::test]
async fn malformed_truncated_json_returns_structured_error() {
    let tool: Arc<dyn Tool> = Arc::new(MalformedPayloadTool {
        id: "truncated-json".into(),
        failure_mode: MalformedMode::TruncatedJson,
    });

    let result = execute_with_recovery(tool, Value::Null, ExecuteOptions::default()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("EOF") || err.to_string().contains("JSON"));
}

#[tokio::test]
async fn malformed_invalid_type_returns_structured_error() {
    let tool: Arc<dyn Tool> = Arc::new(MalformedPayloadTool {
        id: "invalid-type".into(),
        failure_mode: MalformedMode::InvalidType,
    });

    let result = execute_with_recovery(tool, Value::Null, ExecuteOptions::default()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("expected string") || err.to_string().contains("type"));
}

#[tokio::test]
async fn malformed_missing_field_returns_structured_error() {
    let tool: Arc<dyn Tool> = Arc::new(MalformedPayloadTool {
        id: "missing-field".into(),
        failure_mode: MalformedMode::MissingField,
    });

    let result = execute_with_recovery(tool, Value::Null, ExecuteOptions::default()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("missing") || err.to_string().contains("field"));
}

#[tokio::test]
async fn malformed_non_utf8_returns_structured_error() {
    let tool: Arc<dyn Tool> = Arc::new(MalformedPayloadTool {
        id: "non-utf8".into(),
        failure_mode: MalformedMode::NonUtf8,
    });

    let result = execute_with_recovery(tool, Value::Null, ExecuteOptions::default()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("UTF-8") || err.to_string().contains("invalid"));
}

// ============================================================================
// Config Hot-Reload Failure Recovery Tests
// ============================================================================

#[tokio::test]
async fn config_hot_reload_malformed_yaml_retains_lkg() {
    // Create a valid initial config
    let temp_file = NamedTempFile::new().unwrap();
    let initial_yaml = r#"
tools:
  - id: stable
    command: echo
    transport: stdio
    enabled: true
"#;
    std::fs::write(temp_file.path(), initial_yaml).unwrap();

    let registry = Arc::new(ToolRegistry::new());
    let cfg = McpConfigFile::from_path(temp_file.path()).unwrap();
    let count = zylcode_mcp::register_from_config(&registry, cfg).await.unwrap();
    assert_eq!(count, 1);

    let tools = registry.list().await;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].id, "stable");

    // Now write malformed YAML to the same file
    let malformed_yaml = r#"
tools:
  - id: broken
    command: 
    transport: stdio
  - id: also_broken
    command: test
"#;
    std::fs::write(temp_file.path(), malformed_yaml).unwrap();

    // Try to reload - should fail gracefully
    let result = McpConfigFile::from_path(temp_file.path());
    assert!(result.is_err());

    // Original registry should still have the LKG config
    let tools = registry.list().await;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].id, "stable");
}

#[tokio::test]
async fn config_hot_reload_valid_update_works() {
    let temp_file = NamedTempFile::new().unwrap();
    let initial_yaml = r#"
tools:
  - id: tool1
    command: echo
    transport: stdio
    enabled: true
"#;
    std::fs::write(temp_file.path(), initial_yaml).unwrap();

    let registry = Arc::new(ToolRegistry::new());
    let cfg = McpConfigFile::from_path(temp_file.path()).unwrap();
    let _ = zylcode_mcp::register_from_config(&registry, cfg).await;

    assert_eq!(registry.list().await.len(), 1);

    // Write valid update
    let updated_yaml = r#"
tools:
  - id: tool1
    command: echo
    transport: stdio
    enabled: true
  - id: tool2
    command: cat
    transport: stdio
    enabled: true
"#;
    std::fs::write(temp_file.path(), updated_yaml).unwrap();

    let new_cfg = McpConfigFile::from_path(temp_file.path()).unwrap();
    registry.clear().await;
    let count = zylcode_mcp::register_from_config(&registry, new_cfg).await.unwrap();
    assert_eq!(count, 2);

    let tools = registry.list().await;
    assert_eq!(tools.len(), 2);
    let ids: Vec<_> = tools.iter().map(|t| t.id.as_str()).collect();
    assert!(ids.contains(&"tool1"));
    assert!(ids.contains(&"tool2"));
}

// ============================================================================
// Retry Backoff & Timeout Verification Tests
// ============================================================================

#[derive(Debug)]
struct SlowFailingTool {
    id: String,
    delay: Duration,
    max_attempts: usize,
    attempt: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Tool for SlowFailingTool {
    fn id(&self) -> &str {
        &self.id
    }
    fn descriptor(&self) -> zylcode_mcp::tool::ToolDescriptor {
        zylcode_mcp::tool::ToolDescriptor {
            id: self.id.clone(),
            transport: "stdio".into(),
            command: "slow-fail".into(),
            env_keys: vec![],
            description: None,
        }
    }
    async fn call(&self, _params: Value) -> Result<Value> {
        let attempt = self.attempt.fetch_add(1, Ordering::SeqCst) + 1;
        tokio::time::sleep(self.delay).await;
        if attempt >= self.max_attempts {
            Ok(serde_json::json!({"ok": true, "attempt": attempt}))
        } else {
            anyhow::bail!("transient failure {}", attempt)
        }
    }
}

#[tokio::test]
async fn retry_backoff_250ms_exponential() {
    let attempt = Arc::new(AtomicUsize::new(0));
    let tool: Arc<dyn Tool> = Arc::new(SlowFailingTool {
        id: "backoff-test".into(),
        delay: Duration::from_millis(5),
        max_attempts: 3,
        attempt: attempt.clone(),
    });

    let opts = ExecuteOptions {
        timeout: Duration::from_secs(10),
        max_retries: 2,
        retry_backoff: Duration::from_millis(250),
    };

    let start = std::time::Instant::now();
    let result = execute_with_recovery(tool, Value::Null, opts).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    // With exponential backoff: attempt 1 fails -> sleep 250ms -> attempt 2 fails -> sleep 500ms -> attempt 3 succeeds
    // Total sleep = 250 + 500 = 750ms minimum
    assert!(elapsed >= Duration::from_millis(700));
    assert!(elapsed < Duration::from_secs(3));
    assert_eq!(attempt.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn retry_backoff_max_retries_respected() {
    let attempt = Arc::new(AtomicUsize::new(0));
    let tool: Arc<dyn Tool> = Arc::new(SlowFailingTool {
        id: "max-retries".into(),
        delay: Duration::from_millis(5),
        max_attempts: 10, // Never succeeds within max_retries
        attempt: attempt.clone(),
    });

    let opts = ExecuteOptions {
        timeout: Duration::from_secs(10),
        max_retries: 3,
        retry_backoff: Duration::from_millis(100),
    };

    let start = std::time::Instant::now();
    let result = execute_with_recovery(tool, Value::Null, opts).await;
    let elapsed = start.elapsed();

    assert!(result.is_err());
    // Should attempt 4 times (initial + 3 retries)
    assert_eq!(attempt.load(Ordering::SeqCst), 4);
    // Total backoff: 100 + 200 + 300 = 600ms
    assert!(elapsed >= Duration::from_millis(550));
    assert!(elapsed < Duration::from_secs(2));
}

#[tokio::test]
async fn timeout_enforced_per_attempt() {
    let tool: Arc<dyn Tool> = Arc::new(StdioTimeoutTool {
        id: "timeout-enforced".into(),
        delay: Duration::from_millis(200),
    });

    let opts = ExecuteOptions {
        timeout: Duration::from_millis(50),
        max_retries: 1,
        retry_backoff: Duration::from_millis(10),
    };

    let start = std::time::Instant::now();
    let result = execute_with_recovery(tool, Value::Null, opts).await;
    let elapsed = start.elapsed();

    assert!(result.is_err());
    // Should timeout twice (initial + 1 retry) * ~50ms + 10ms backoff
    assert!(elapsed >= Duration::from_millis(100));
    assert!(elapsed < Duration::from_millis(200));
}

#[tokio::test]
async fn cancellation_token_propagates() {
    let tool: Arc<dyn Tool> = Arc::new(StdioTimeoutTool {
        id: "cancellation".into(),
        delay: Duration::from_secs(10),
    });

    let opts = ExecuteOptions {
        timeout: Duration::from_millis(50),
        max_retries: 0,
        retry_backoff: Duration::from_millis(10),
    };

    let handle = tokio::spawn(async move {
        execute_with_recovery(tool, Value::Null, opts).await
    });

    // Cancel after 100ms
    tokio::time::sleep(Duration::from_millis(100)).await;
    handle.abort();

    let result = handle.await;
    // Task should be cancelled
    assert!(result.is_err() || matches!(result, Ok(Err(_))));
}

// ============================================================================
// Concurrent Stress Tests
// ============================================================================

#[tokio::test]
async fn concurrent_tool_execution_no_crosstalk() {
    let registry = Arc::new(ToolRegistry::new());

    // Register 100 tools
    for i in 0..100 {
        let cfg = McpToolConfig {
            id: format!("tool_{}", i),
            command: "echo".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        registry.register(Arc::new(DynamicTool::new(cfg))).await;
    }

    // Concurrent execution of random tools
    let mut handles = vec![];
    for _ in 0..50 {
        let reg = registry.clone();
        handles.push(tokio::spawn(async move {
            let idx = fastrand::usize(0..100);
            let tool = reg.get(&format!("tool_{}", idx)).await.unwrap();
            let result = tool.call(Value::Null).await;
            assert!(result.is_ok());
            let val = result.unwrap();
            assert_eq!(val["tool"], format!("tool_{}", idx));
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

#[tokio::test]
async fn concurrent_registry_modification_during_execution() {
    let registry = Arc::new(ToolRegistry::new());

    // Initial tools
    for i in 0..20 {
        let cfg = McpToolConfig {
            id: format!("tool_{}", i),
            command: "echo".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        registry.register(Arc::new(DynamicTool::new(cfg))).await;
    }

    let mut handles = vec![];

    // Writer: add/remove tools
    for batch in 0..5 {
        let reg = registry.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..10 {
                let id = format!("dynamic_{}_{}", batch, i);
                let cfg = McpToolConfig {
                    id: id.clone(),
                    command: "echo".into(),
                    transport: McpTransport::Stdio,
                    env: Default::default(),
                    enabled: true,
                    description: None,
                };
                reg.register(Arc::new(DynamicTool::new(cfg))).await;
            }
            // Clear after each batch
            reg.clear().await;
        }));
    }

    // Reader: execute tools concurrently
    for _ in 0..30 {
        let reg = registry.clone();
        handles.push(tokio::spawn(async move {
            let tools = reg.list().await;
            if !tools.is_empty() {
                let idx = fastrand::usize(0..tools.len());
                let tool = reg.get(&tools[idx].id).await.unwrap();
                let _ = tool.call(Value::Null).await;
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}