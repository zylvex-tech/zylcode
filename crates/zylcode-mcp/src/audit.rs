//! Tamper-evident audit logging for zylcode-mcp — structured JSON audit trail with payload hashing.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Audit event types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    ToolInvoke,
    ToolResult,
    TransportReconnect,
    RetryAttempt,
    CircuitTrip,
    ConfigReload,
    ConfigValidationError,
    AuthFailure,
    /// Verification-ladder event (Rung 3 permission gate evaluation).
    VerificationRung,
}

/// Audit event severity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// RFC3339 timestamp.
    pub timestamp: String,
    /// Monotonic sequence number for ordering.
    pub sequence: u64,
    /// Event type.
    pub event_type: AuditEventType,
    /// Severity level.
    pub severity: AuditSeverity,
    /// Tool identifier (if applicable).
    pub tool_id: Option<String>,
    /// Transport type (stdio, sse, websocket).
    pub transport: Option<String>,
    /// Caller identity (anonymized).
    pub caller_id: Option<String>,
    /// SHA-256 hash of input payload.
    pub input_hash: Option<String>,
    /// SHA-256 hash of output payload.
    pub output_hash: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: Option<u64>,
    /// Retry attempt number (if applicable).
    pub retry_attempt: Option<u32>,
    /// Maximum retries configured.
    pub max_retries: Option<u32>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Additional context as JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    /// Chain hash for tamper evidence (hash of previous entry + this entry).
    pub chain_hash: String,
}

/// Configuration for audit logger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Audit log file path.
    pub log_path: PathBuf,
    /// Maximum file size before rotation (bytes).
    #[serde(default = "default_max_size")]
    pub max_file_size: u64,
    /// Maximum number of rotated files to keep.
    #[serde(default = "default_max_files")]
    pub max_files: u32,
    /// Minimum severity to log.
    #[serde(default = "default_min_severity")]
    pub min_severity: AuditSeverity,
    /// Whether to include payload hashes.
    #[serde(default = "default_true")]
    pub include_hashes: bool,
}

fn default_max_size() -> u64 {
    100 * 1024 * 1024 // 100 MB
}
fn default_max_files() -> u32 {
    10
}
fn default_min_severity() -> AuditSeverity {
    AuditSeverity::Info
}
fn default_true() -> bool {
    true
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            log_path: PathBuf::from("audit.log"),
            max_file_size: default_max_size(),
            max_files: default_max_files(),
            min_severity: default_min_severity(),
            include_hashes: true,
        }
    }
}

/// Thread-safe audit logger with chain hashing for tamper evidence.
pub struct AuditLogger {
    inner: Arc<Mutex<AuditLoggerInner>>,
}

struct AuditLoggerInner {
    config: AuditConfig,
    writer: BufWriter<File>,
    sequence: u64,
    last_chain_hash: String,
    current_size: u64,
}

impl AuditLogger {
    /// Create a new audit logger with the given configuration.
    #[allow(dead_code)]
    pub fn new(config: AuditConfig) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = config.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open or create the log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_path)?;

        let _current_size = file.metadata()?.len();
        let writer = BufWriter::new(file);

        // Read last chain hash from existing file if any
        let last_chain_hash = Self::read_last_chain_hash(&config.log_path).unwrap_or_default();

        Ok(Self {
            inner: Arc::new(Mutex::new(AuditLoggerInner {
                config,
                writer,
                sequence: 0,
                last_chain_hash,
                current_size: 0,
            })),
        })
    }

    /// Read the last chain hash from the audit log file.
    fn read_last_chain_hash(path: &Path) -> Result<String> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut last_hash = String::new();
        for line in reader.lines() {
            let line = line?;
            if let Ok(entry) = serde_json::from_str::<AuditEntry>(&line) {
                last_hash = entry.chain_hash;
            }
        }
        Ok(last_hash)
    }

    /// Compute SHA-256 hash of input.
    pub fn hash_payload(payload: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(payload);
        format!("{:x}", hasher.finalize())
    }

    /// Compute chain hash: SHA-256(previous_chain_hash + current_entry_json).
    #[allow(dead_code)]
    fn compute_chain_hash(&self, previous_hash: &str, entry_json: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(previous_hash.as_bytes());
        hasher.update(entry_json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute chain hash static: SHA-256(previous_hash + entry_json).
    #[allow(dead_code)]
    fn compute_chain_hash_static(previous_hash: &str, entry_json: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(previous_hash.as_bytes());
        hasher.update(entry_json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Rotate log file.
    pub fn log(&self, entry: AuditEntry) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();

        // Check severity filter
        if entry.severity < inner.config.min_severity {
            return Ok(());
        }

        inner.sequence += 1;
        let mut entry = entry;
        entry.sequence = inner.sequence;

        // Compute chain hash
        let entry_json = serde_json::to_string(&AuditEntry {
            chain_hash: String::new(), // placeholder
            ..entry.clone()
        })?;
        entry.chain_hash = inner.compute_chain_hash(&inner.last_chain_hash, &entry_json);
        inner.last_chain_hash = entry.chain_hash.clone();

        // Serialize and write
        let line = serde_json::to_string(&entry)? + "\n";
        inner.writer.write_all(line.as_bytes())?;
        inner.writer.flush()?;
        inner.current_size += line.len() as u64;

        // Rotate if needed
        if inner.current_size >= inner.config.max_file_size {
            inner.rotate()?;
        }

        Ok(())
    }

    /// Log a tool invocation.
    pub fn log_tool_invoke(
        &self,
        tool_id: &str,
        transport: &str,
        caller_id: Option<&str>,
        input: &[u8],
        retry_attempt: Option<u32>,
        max_retries: Option<u32>,
    ) -> Result<()> {
        let inner = self.inner.lock().unwrap();
        let input_hash = if inner.config.include_hashes {
            Some(Self::hash_payload(input))
        } else {
            None
        };

        let entry = AuditEntry {
            timestamp: Self::now_rfc3339(),
            sequence: 0, // will be set by log()
            event_type: AuditEventType::ToolInvoke,
            severity: AuditSeverity::Info,
            tool_id: Some(tool_id.into()),
            transport: Some(transport.into()),
            caller_id: caller_id.map(|s| s.into()),
            input_hash,
            output_hash: None,
            duration_ms: None,
            retry_attempt,
            max_retries,
            error: None,
            context: None,
            chain_hash: String::new(),
        };

        drop(inner);
        self.log(entry)
    }

    /// Log a tool result.
    pub fn log_tool_result(
        &self,
        tool_id: &str,
        transport: &str,
        output: &[u8],
        duration_ms: u64,
        error: Option<&str>,
    ) -> Result<()> {
        let inner = self.inner.lock().unwrap();
        let output_hash = if inner.config.include_hashes {
            Some(Self::hash_payload(output))
        } else {
            None
        };

        let entry = AuditEntry {
            timestamp: Self::now_rfc3339(),
            sequence: 0,
            event_type: AuditEventType::ToolResult,
            severity: error.map_or(AuditSeverity::Info, |_| AuditSeverity::Error),
            tool_id: Some(tool_id.into()),
            transport: Some(transport.into()),
            caller_id: None,
            input_hash: None,
            output_hash,
            duration_ms: Some(duration_ms),
            retry_attempt: None,
            max_retries: None,
            error: error.map(|s| s.into()),
            context: None,
            chain_hash: String::new(),
        };

        drop(inner);
        self.log(entry)
    }

    /// Log a transport reconnection.
    pub fn log_transport_reconnect(
        &self,
        transport: &str,
        attempt: u32,
        error: Option<&str>,
    ) -> Result<()> {
        let entry = AuditEntry {
            timestamp: Self::now_rfc3339(),
            sequence: 0,
            event_type: AuditEventType::TransportReconnect,
            severity: error.map_or(AuditSeverity::Warning, |_| AuditSeverity::Error),
            tool_id: None,
            transport: Some(transport.into()),
            caller_id: None,
            input_hash: None,
            output_hash: None,
            duration_ms: None,
            retry_attempt: Some(attempt),
            max_retries: None,
            error: error.map(|s| s.into()),
            context: None,
            chain_hash: String::new(),
        };

        self.log(entry)
    }

    /// Log a retry attempt.
    pub fn log_retry(
        &self,
        tool_id: &str,
        attempt: u32,
        max_retries: u32,
        error: &str,
    ) -> Result<()> {
        let entry = AuditEntry {
            timestamp: Self::now_rfc3339(),
            sequence: 0,
            event_type: AuditEventType::RetryAttempt,
            severity: AuditSeverity::Warning,
            tool_id: Some(tool_id.into()),
            transport: None,
            caller_id: None,
            input_hash: None,
            output_hash: None,
            duration_ms: None,
            retry_attempt: Some(attempt),
            max_retries: Some(max_retries),
            error: Some(error.into()),
            context: None,
            chain_hash: String::new(),
        };

        self.log(entry)
    }

    /// Log circuit breaker trip.
    pub fn log_circuit_trip(&self, tool_id: &str, state: &str) -> Result<()> {
        let entry = AuditEntry {
            timestamp: Self::now_rfc3339(),
            sequence: 0,
            event_type: AuditEventType::CircuitTrip,
            severity: AuditSeverity::Critical,
            tool_id: Some(tool_id.into()),
            transport: None,
            caller_id: None,
            input_hash: None,
            output_hash: None,
            duration_ms: None,
            retry_attempt: None,
            max_retries: None,
            error: Some(format!("Circuit breaker tripped: {}", state)),
            context: Some(serde_json::json!({ "state": state })),
            chain_hash: String::new(),
        };

        self.log(entry)
    }

    /// Log config reload.
    pub fn log_config_reload(&self, tool_count: usize, error: Option<&str>) -> Result<()> {
        let entry = AuditEntry {
            timestamp: Self::now_rfc3339(),
            sequence: 0,
            event_type: AuditEventType::ConfigReload,
            severity: error.map_or(AuditSeverity::Info, |_| AuditSeverity::Error),
            tool_id: None,
            transport: None,
            caller_id: None,
            input_hash: None,
            output_hash: None,
            duration_ms: None,
            retry_attempt: None,
            max_retries: None,
            error: error.map(|s| s.into()),
            context: Some(serde_json::json!({ "tools_loaded": tool_count })),
            chain_hash: String::new(),
        };

        self.log(entry)
    }

    /// Log a verification-rung event from the permission gate.
    ///
    /// Records which rung of the verification ladder was evaluated, the agent
    /// and tool involved, and the decision outcome. The `context` field carries
    /// structured JSON for downstream telemetry aggregation.
    pub fn log_verification_rung(
        &self,
        rung: &str,
        agent_id: &str,
        tool_id: &str,
        decision: &str,
        granted: bool,
    ) -> Result<()> {
        let severity = if granted {
            AuditSeverity::Info
        } else {
            AuditSeverity::Warning
        };

        let entry = AuditEntry {
            timestamp: Self::now_rfc3339(),
            sequence: 0,
            event_type: AuditEventType::VerificationRung,
            severity,
            tool_id: Some(tool_id.into()),
            transport: None,
            caller_id: Some(agent_id.into()),
            input_hash: None,
            output_hash: None,
            duration_ms: None,
            retry_attempt: None,
            max_retries: None,
            error: None,
            context: Some(serde_json::json!({
                "rung": rung,
                "agent_id": agent_id,
                "tool_id": tool_id,
                "decision": decision,
                "granted": granted,
            })),
            chain_hash: String::new(),
        };

        self.log(entry)
    }

    /// Get current timestamp in RFC3339 format.
    fn now_rfc3339() -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let nanos = now.subsec_nanos();
        // Simple RFC3339 format
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}Z",
            1970 + (secs / 31536000),
            ((secs % 31536000) / 2592000) + 1,
            ((secs % 2592000) / 86400) + 1,
            (secs % 86400) / 3600,
            (secs % 3600) / 60,
            secs % 60,
            nanos
        )
    }

    /// Rotate log file.
    #[allow(dead_code)]
    fn rotate(&mut self) -> Result<()> {
        self.inner.lock().unwrap().rotate()
    }

    /// Verify the integrity of the audit log chain.
    pub fn verify_chain(&self, path: &Path) -> Result<Vec<usize>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut broken_indices = Vec::new();
        let mut previous_hash = String::new();
        let mut line_num = 0;

        for line in reader.lines() {
            line_num += 1;
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: AuditEntry = serde_json::from_str(&line)?;

            // Recompute chain hash
            let entry_json = serde_json::to_string(&AuditEntry {
                chain_hash: String::new(),
                ..entry.clone()
            })?;
            let expected_hash = AuditLoggerInner::compute_chain_hash_static(&previous_hash, &entry_json);

            if entry.chain_hash != expected_hash {
                broken_indices.push(line_num);
            }
            previous_hash = entry.chain_hash;
        }

        Ok(broken_indices)
    }

    /// Flush any buffered writes.
    pub fn flush(&self) -> Result<()> {
        self.inner.lock().unwrap().writer.flush()?;
        Ok(())
    }
}

impl AuditLoggerInner {
    /// Compute chain hash: SHA-256(previous_hash + entry_json).
    fn compute_chain_hash(&self, previous_hash: &str, entry_json: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(previous_hash.as_bytes());
        hasher.update(entry_json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute chain hash static: SHA-256(previous_hash + entry_json).
    fn compute_chain_hash_static(previous_hash: &str, entry_json: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(previous_hash.as_bytes());
        hasher.update(entry_json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Rotate log file.
    fn rotate(&mut self) -> Result<()> {
        let config = self.config.clone();
        let path = config.log_path.clone();

        // Close current writer
        self.writer.flush()?;

        // On Windows, we can't write to /dev/null, so use a temp file instead
        #[cfg(target_os = "windows")]
        {
            let temp_file = std::env::temp_dir().join(format!("audit_null_{}.tmp", std::process::id()));
            self.writer = BufWriter::new(File::create(&temp_file)?);
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.writer = BufWriter::new(File::create("/dev/null")?);
        }

        // Rotate existing files
        for i in (1..config.max_files).rev() {
            let old = path.with_extension(format!("log.{}", i));
            let new = path.with_extension(format!("log.{}", i + 1));
            if old.exists() {
                std::fs::rename(&old, &new)?;
            }
        }

        // Move current to .1
        let rotated = path.with_extension("log.1");
        std::fs::rename(&config.log_path, &rotated)?;

        // Reopen
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_path)?;
        self.writer = BufWriter::new(file);
        self.current_size = 0;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn audit_logger_basic_write() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = AuditConfig {
            log_path: temp_file.path().to_path_buf(),
            ..Default::default()
        };
        let logger = AuditLogger::new(config).unwrap();

        logger.log_tool_invoke("test-tool", "stdio", Some("caller-1"), b"input", None, None).unwrap();
        logger.log_tool_result("test-tool", "stdio", b"output", 100, None).unwrap();

        // Verify chain
        let broken = logger.verify_chain(temp_file.path()).unwrap();
        assert!(broken.is_empty());
    }

    #[test]
    fn audit_chain_tamper_detection() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = AuditConfig {
            log_path: temp_file.path().to_path_buf(),
            ..Default::default()
        };
        let logger = AuditLogger::new(config).unwrap();

        logger.log_tool_invoke("tool1", "stdio", None, b"in", None, None).unwrap();
        logger.log_tool_result("tool1", "stdio", b"out", 50, None).unwrap();

        // Manually corrupt the file
        let mut content = std::fs::read_to_string(temp_file.path()).unwrap();
        content = content.replace("tool1", "tampered");
        std::fs::write(temp_file.path(), content).unwrap();

        // Verify should detect tampering
        let broken = logger.verify_chain(temp_file.path()).unwrap();
        assert!(!broken.is_empty());
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn audit_log_rotation() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = AuditConfig {
            log_path: temp_file.path().to_path_buf(),
            max_file_size: 100, // Very small for testing
            ..Default::default()
        };
        let logger = AuditLogger::new(config).unwrap();

        // Write enough to trigger rotation
        for i in 0..50 {
            logger.log_tool_invoke(&format!("tool{}", i), "stdio", None, &b"x".repeat(100), None, None).unwrap();
        }

        // Should have rotated
        assert!(temp_file.path().with_extension("log.1").exists());
    }

    #[test]
    fn audit_hash_payload() {
        let hash = AuditLogger::hash_payload(b"test input");
        assert_eq!(hash.len(), 64); // SHA-256 hex = 64 chars
        assert_eq!(hash, "9dfe6f15d1ab73af898739394fd22fd72a03db01834582f24bb2e1c66c7aaeae");
    }

    #[test]
    fn audit_config_validation_error() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = AuditConfig {
            log_path: temp_file.path().to_path_buf(),
            ..Default::default()
        };
        let logger = AuditLogger::new(config).unwrap();

        logger.log_config_reload(5, Some("invalid YAML")).unwrap();

        let broken = logger.verify_chain(temp_file.path()).unwrap();
        assert!(broken.is_empty());
    }
}