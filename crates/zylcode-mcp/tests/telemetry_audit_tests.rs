//! Telemetry and audit verification tests for zylcode-mcp.

use anyhow::Result;
use serde_json::Value;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::Duration;
use tempfile::NamedTempFile;
use zylcode_mcp::audit::{AuditConfig, AuditLogger, AuditSeverity};
use zylcode_mcp::executor::{execute_with_recovery_telemetry, ExecuteOptions};
use zylcode_mcp::telemetry::{Telemetry, TelemetryConfig};
use zylcode_mcp::tool::{DynamicTool, Tool};
use zylcode_mcp::config::{McpToolConfig, McpTransport};

// ============================================================================
// Test Helpers
// ============================================================================

#[derive(Debug)]
struct TestTool {
    id: String,
    should_fail: Arc<AtomicUsize>,
    max_failures: usize,
    delay: Duration,
}

#[async_trait::async_trait]
impl Tool for TestTool {
    fn id(&self) -> &str {
        &self.id
    }
    fn descriptor(&self) -> zylcode_mcp::tool::ToolDescriptor {
        zylcode_mcp::tool::ToolDescriptor {
            id: self.id.clone(),
            transport: "stdio".into(),
            command: "test".into(),
            env_keys: vec![],
            description: None,
        }
    }
    async fn call(&self, _params: Value) -> Result<Value> {
        let fails = self.should_fail.fetch_add(1, Ordering::SeqCst);
        if fails < self.max_failures {
            tokio::time::sleep(self.delay).await;
            anyhow::bail!("transient failure {}", fails);
        }
        Ok(serde_json::json!({"ok": true, "attempt": fails + 1}))
    }
}

#[derive(Debug)]
struct FailingTool {
    id: String,
    error_msg: String,
}

#[async_trait::async_trait]
impl Tool for FailingTool {
    fn id(&self) -> &str {
        &self.id
    }
    fn descriptor(&self) -> zylcode_mcp::tool::ToolDescriptor {
        zylcode_mcp::tool::ToolDescriptor {
            id: self.id.clone(),
            transport: "sse".into(),
            command: "https://example.com".into(),
            env_keys: vec![],
            description: None,
        }
    }
    async fn call(&self, _params: Value) -> Result<Value> {
        anyhow::bail!("{}", self.error_msg);
    }
}

// ============================================================================
// Telemetry Tests
// ============================================================================

#[tokio::test]
async fn telemetry_init_shutdown() {
    let config = TelemetryConfig {
        service_name: "test-telemetry".into(),
        otlp_endpoint: None,
        sample_ratio: 1.0,
        export_timeout_secs: 5,
    };
    let telemetry = Telemetry::init(config).unwrap();
    telemetry.shutdown().await.unwrap();
}

#[tokio::test]
async fn telemetry_tool_invoke_span() {
    let config = TelemetryConfig {
        service_name: "test-tool-span".into(),
        otlp_endpoint: None,
        sample_ratio: 1.0,
        export_timeout_secs: 5,
    };
    let telemetry = Arc::new(Telemetry::init(config).unwrap());

    let tool: Arc<dyn Tool> = Arc::new(DynamicTool::new(McpToolConfig {
        id: "test-tool".into(),
        command: "echo".into(),
        transport: McpTransport::Stdio,
        env: Default::default(),
        enabled: true,
        description: None,
    }));

    let result = execute_with_recovery_telemetry(
        tool,
        serde_json::json!({"test": "data"}),
        ExecuteOptions::default(),
        Some(telemetry.clone()),
        None,
        Some("test-caller".into()),
    ).await;

    assert!(result.is_ok());
    telemetry.shutdown().await.unwrap();
}

#[tokio::test]
async fn telemetry_retry_spans() {
    let config = TelemetryConfig {
        service_name: "test-retry-spans".into(),
        otlp_endpoint: None,
        sample_ratio: 1.0,
        export_timeout_secs: 5,
    };
    let telemetry = Arc::new(Telemetry::init(config).unwrap());

    let tool: Arc<dyn Tool> = Arc::new(TestTool {
        id: "retry-tool".into(),
        should_fail: Arc::new(AtomicUsize::new(0)),
        max_failures: 2, // Fail twice, succeed on 3rd
        delay: Duration::from_millis(10),
    });

    let opts = ExecuteOptions {
        timeout: Duration::from_secs(5),
        max_retries: 3,
        retry_backoff: Duration::from_millis(50),
    };

    let result = execute_with_recovery_telemetry(
        tool,
        serde_json::json!({"input": "test"}),
        opts,
        Some(telemetry.clone()),
        None,
        Some("retry-caller".into()),
    ).await;

    assert!(result.is_ok());
    let val = result.unwrap();
    assert_eq!(val["attempt"], 3); // 0-indexed, so 3rd attempt
    
    telemetry.shutdown().await.unwrap();
}

#[tokio::test]
async fn telemetry_sse_transient_retry() {
    let config = TelemetryConfig {
        service_name: "test-sse-retry".into(),
        otlp_endpoint: None,
        sample_ratio: 1.0,
        export_timeout_secs: 5,
    };
    let telemetry = Arc::new(Telemetry::init(config).unwrap());

    let tool: Arc<dyn Tool> = Arc::new(TestTool {
        id: "sse-transient".into(),
        should_fail: Arc::new(AtomicUsize::new(0)),
        max_failures: 1,
        delay: Duration::from_millis(5),
    });

    let opts = ExecuteOptions {
        timeout: Duration::from_secs(5),
        max_retries: 2,
        retry_backoff: Duration::from_millis(20),
    };

    let result = execute_with_recovery_telemetry(
        tool,
        serde_json::json!({}),
        opts,
        Some(telemetry.clone()),
        None,
        None,
    ).await;

    assert!(result.is_ok());
    telemetry.shutdown().await.unwrap();
}

// ============================================================================
// Audit Logger Tests
// ============================================================================

#[test]
fn audit_logger_tool_invoke_and_result() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        min_severity: AuditSeverity::Info,
        include_hashes: true,
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    // Log tool invoke
    logger.log_tool_invoke("test-tool", "stdio", Some("caller-1"), b"input data", None, None).unwrap();
    
    // Log tool result
    logger.log_tool_result("test-tool", "stdio", b"output data", 100, None).unwrap();
    
    // Verify chain integrity
    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty(), "Audit chain should be intact");
}

#[test]
fn audit_logger_retry_and_error() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        min_severity: AuditSeverity::Info,
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    logger.log_tool_invoke("retry-tool", "stdio", None, b"in", None, None).unwrap();
    logger.log_retry("retry-tool", 1, 3, "transient error").unwrap();
    logger.log_retry("retry-tool", 2, 3, "another error").unwrap();
    logger.log_tool_result("retry-tool", "stdio", b"success", 50, None).unwrap();

    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty());
}

#[test]
fn audit_logger_sse_reconnect() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    logger.log_transport_reconnect("sse", 1, Some("connection dropped")).unwrap();
    logger.log_transport_reconnect("sse", 2, None).unwrap();

    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty());
}

#[test]
fn audit_logger_circuit_trip() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    logger.log_circuit_trip("critical-tool", "open").unwrap();

    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty());
}

#[test]
fn audit_logger_config_reload() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    logger.log_config_reload(10, None).unwrap();
    logger.log_config_reload(8, Some("validation failed")).unwrap();

    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty());
}

#[test]
fn audit_logger_payload_hash() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        include_hashes: true,
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    let input = b"test input payload";
    let output = b"test output payload";
    
    logger.log_tool_invoke("hash-test", "stdio", None, input, None, None).unwrap();
    logger.log_tool_result("hash-test", "stdio", output, 10, None).unwrap();

    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty());
}

#[test]
fn audit_logger_tamper_detection() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        include_hashes: true,
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    logger.log_tool_invoke("tamper-test", "stdio", None, b"input", None, None).unwrap();
    logger.log_tool_result("tamper-test", "stdio", b"output", 50, None).unwrap();

    // Read and tamper with the file
    let mut content = std::fs::read_to_string(temp_file.path()).unwrap();
    content = content.replace("tamper-test", "TAMPERED");
    std::fs::write(temp_file.path(), content).unwrap();

    // Verify should detect tampering
    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(!broken.is_empty(), "Should detect tampering");
}

#[test]
fn audit_logger_severity_filter() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        min_severity: AuditSeverity::Warning, // Only Warning and above
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    // Info should be filtered out
    logger.log_tool_invoke("filter-test", "stdio", None, b"in", None, None).unwrap();
    
    // Warning should be logged
    logger.log_retry("filter-test", 1, 3, "warning").unwrap();

    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty());
    
    // Verify only one entry (the retry) was written
    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("retry_attempt"));
}

#[test]
fn audit_logger_circuit_trip_critical() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        min_severity: AuditSeverity::Info,
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    logger.log_circuit_trip("circuit-tool", "open").unwrap();

    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty());

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("circuit_trip"));
    assert!(content.contains("critical"));
}

// ============================================================================
// Integration Tests: Telemetry + Audit + Executor
// ============================================================================

#[tokio::test]
async fn executor_with_telemetry_and_audit() {
    // Setup telemetry
    let telemetry_config = TelemetryConfig {
        service_name: "integration-test".into(),
        otlp_endpoint: None,
        sample_ratio: 1.0,
        export_timeout_secs: 5,
    };
    let telemetry = Arc::new(Telemetry::init(telemetry_config).unwrap());

    // Setup audit
    let temp_file = NamedTempFile::new().unwrap();
    let audit_config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        include_hashes: true,
        ..Default::default()
    };
    let audit = Arc::new(AuditLogger::new(audit_config).unwrap());

    // Create a tool that fails once then succeeds
    let tool: Arc<dyn Tool> = Arc::new(TestTool {
        id: "integration-tool".into(),
        should_fail: Arc::new(AtomicUsize::new(0)),
        max_failures: 1,
        delay: Duration::from_millis(5),
    });

    let opts = ExecuteOptions {
        timeout: Duration::from_secs(5),
        max_retries: 2,
        retry_backoff: Duration::from_millis(50),
    };

    let result = execute_with_recovery_telemetry(
        tool,
        serde_json::json!({"integration": "test"}),
        opts,
        Some(telemetry.clone()),
        Some(audit.clone()),
        Some("integration-caller".into()),
    ).await;

    assert!(result.is_ok());
    let val = result.unwrap();
    assert_eq!(val["attempt"], 2);

    // Verify audit chain
    let broken = audit.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty(), "Audit chain should be intact after integration test");

    // Verify log contains expected events
    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("tool_invoke"));
    assert!(content.contains("retry_attempt"));
    assert!(content.contains("tool_result"));

    telemetry.shutdown().await.unwrap();
}

#[tokio::test]
async fn executor_with_audit_only() {
    let temp_file = NamedTempFile::new().unwrap();
    let audit_config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        ..Default::default()
    };
    let audit = Arc::new(AuditLogger::new(audit_config).unwrap());

    let tool: Arc<dyn Tool> = Arc::new(DynamicTool::new(McpToolConfig {
        id: "audit-only-tool".into(),
        command: "echo".into(),
        transport: McpTransport::Stdio,
        env: Default::default(),
        enabled: true,
        description: None,
    }));

    let result = execute_with_recovery_telemetry(
        tool,
        serde_json::json!({"audit": "only"}),
        ExecuteOptions::default(),
        None, // no telemetry
        Some(audit.clone()),
        Some("audit-caller".into()),
    ).await;

    assert!(result.is_ok());

    let broken = audit.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty());

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("tool_invoke"));
    assert!(content.contains("tool_result"));
    assert!(content.contains("audit-only-tool"));
}

#[tokio::test]
async fn executor_failure_audit_log() {
    let temp_file = NamedTempFile::new().unwrap();
    let audit_config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        ..Default::default()
    };
    let audit = Arc::new(AuditLogger::new(audit_config).unwrap());

    let tool: Arc<dyn Tool> = Arc::new(FailingTool {
        id: "always-fail".into(),
        error_msg: "permanent failure".into(),
    });

    let opts = ExecuteOptions {
        timeout: Duration::from_secs(5),
        max_retries: 2,
        retry_backoff: Duration::from_millis(10),
    };

    let result = execute_with_recovery_telemetry(
        tool,
        serde_json::json!({}),
        opts,
        None,
        Some(audit.clone()),
        None,
    ).await;

    assert!(result.is_err());

    let broken = audit.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty());

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("tool_invoke"));
    assert!(content.contains("retry_attempt"));
    assert!(content.contains("tool_result"));
    assert!(content.contains("permanent failure"));
}

// ============================================================================
// Hash Consistency Tests
// ============================================================================

#[test]
fn audit_hash_payload_consistency() {
    let payload = b"consistent test payload";
    let hash1 = zylcode_mcp::audit::AuditLogger::hash_payload(payload);
    let hash2 = zylcode_mcp::audit::AuditLogger::hash_payload(payload);
    
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64); // SHA-256 hex
}

// ============================================================================
// Concurrent Execution Tests
// ============================================================================

#[tokio::test]
async fn concurrent_telemetry_spans() {
    let config = TelemetryConfig {
        service_name: "concurrent-test".into(),
        otlp_endpoint: None,
        sample_ratio: 1.0,
        export_timeout_secs: 5,
    };
    let telemetry = Arc::new(Telemetry::init(config).unwrap());

    let mut handles = vec![];

    for i in 0..10 {
        let telemetry = telemetry.clone();
        handles.push(tokio::spawn(async move {
            let tool: Arc<dyn Tool> = Arc::new(DynamicTool::new(McpToolConfig {
                id: format!("concurrent-tool-{}", i),
                command: "echo".into(),
                transport: McpTransport::Stdio,
                env: Default::default(),
                enabled: true,
                description: None,
            }));

            let _ = execute_with_recovery_telemetry(
                tool,
                serde_json::json!({"index": i}),
                ExecuteOptions::default(),
                Some(telemetry),
                None,
                None,
            ).await.unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

#[tokio::test]
async fn concurrent_audit_logging() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        ..Default::default()
    };
    let logger = Arc::new(AuditLogger::new(config).unwrap());

    let mut handles = vec![];

    for i in 0..20 {
        let logger = logger.clone();
        handles.push(tokio::spawn(async move {
            logger.log_tool_invoke(&format!("tool-{}", i), "stdio", None, b"input", None, None).unwrap();
            logger.log_tool_result(&format!("tool-{}", i), "stdio", b"output", 10, None).unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    logger.flush().unwrap();

    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty(), "Audit chain should be intact under concurrent load");
}

// ============================================================================
// Verification Rung Telemetry Tests
// ============================================================================

#[test]
fn audit_verification_rung_granted() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        min_severity: AuditSeverity::Info,
        include_hashes: true,
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    // Log a verification-rung event that was granted
    logger
        .log_verification_rung(
            "rung_1",
            "agent-alpha",
            "tool-x",
            "Allow",
            true,
        )
        .unwrap();

    // Verify chain integrity
    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty(), "Audit chain should be intact");

    // Verify log content
    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("verification_rung"), "Should contain event type");
    assert!(content.contains("rung_1"), "Should contain rung identifier");
    assert!(content.contains("agent-alpha"), "Should contain agent_id");
    assert!(content.contains("tool-x"), "Should contain tool_id");
    assert!(content.contains("Allow"), "Should contain decision");
    // granted=true => Info severity
    assert!(content.contains("info"), "Granted event should have info severity");
}

#[test]
fn audit_verification_rung_denied() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        min_severity: AuditSeverity::Info,
        include_hashes: true,
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    // Log a verification-rung event that was denied
    logger
        .log_verification_rung(
            "rung_2",
            "agent-beta",
            "tool-y",
            "DenyNoRule",
            false,
        )
        .unwrap();

    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty(), "Audit chain should be intact");

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("verification_rung"));
    assert!(content.contains("rung_2"));
    assert!(content.contains("agent-beta"));
    assert!(content.contains("tool-y"));
    assert!(content.contains("DenyNoRule"));
    // granted=false => Warning severity
    assert!(content.contains("warning"), "Denied event should have warning severity");
}

#[test]
fn audit_verification_rung_severity_filtering() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        min_severity: AuditSeverity::Warning, // Only Warning and above
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    // granted=true => Info => should be filtered out
    logger
        .log_verification_rung("rung_1", "agent-a", "tool-a", "Allow", true)
        .unwrap();

    // granted=false => Warning => should be logged
    logger
        .log_verification_rung("rung_2", "agent-b", "tool-b", "DenyNoRule", false)
        .unwrap();

    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(broken.is_empty());

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    // Only the denied event should appear
    assert!(content.contains("rung_2"), "Denied event should be logged");
    assert!(!content.contains("rung_1"), "Granted event should be filtered out");
}

#[test]
fn audit_verification_rung_chain_integrity() {
    let temp_file = NamedTempFile::new().unwrap();
    let config = AuditConfig {
        log_path: temp_file.path().to_path_buf(),
        include_hashes: true,
        ..Default::default()
    };
    let logger = AuditLogger::new(config).unwrap();

    // Log a sequence of verification-rung events
    logger
        .log_verification_rung("rung_1", "agent-1", "tool-1", "Allow", true)
        .unwrap();
    logger
        .log_verification_rung("rung_2", "agent-2", "tool-2", "DenyNoRule", false)
        .unwrap();
    logger
        .log_verification_rung("rung_3", "agent-3", "tool-3", "Allow", true)
        .unwrap();

    let broken = logger.verify_chain(temp_file.path()).unwrap();
    assert!(
        broken.is_empty(),
        "Chain should be intact across multiple rung events, broken at: {:?}",
        broken
    );
}