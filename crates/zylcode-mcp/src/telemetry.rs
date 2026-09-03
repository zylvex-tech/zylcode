//! Simplified telemetry for zylcode-mcp — structured spans using tracing directly.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{span, Level, Span};
use uuid::Uuid;

/// Configuration for telemetry initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Service name for resource attribution.
    pub service_name: String,
    /// OTLP endpoint (e.g., "http://localhost:4317").
    pub otlp_endpoint: Option<String>,
    /// Sampling ratio (0.0 to 1.0).
    #[serde(default = "default_sample_ratio")]
    pub sample_ratio: f64,
    /// Export timeout.
    #[serde(default = "default_export_timeout")]
    pub export_timeout_secs: u64,
}

fn default_sample_ratio() -> f64 {
    1.0
}

fn default_export_timeout() -> u64 {
    30
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "zylcode-mcp".into(),
            otlp_endpoint: None,
            sample_ratio: 1.0,
            export_timeout_secs: 30,
        }
    }
}

/// Global telemetry handle using tracing spans.
#[derive(Clone)]
#[allow(dead_code)]
pub struct Telemetry {
    service_name: String,
}

impl Telemetry {
    /// Initialize the telemetry subsystem.
    pub fn init(config: TelemetryConfig) -> Result<Self> {
        Ok(Self {
            service_name: config.service_name,
        })
    }

    /// Shutdown the telemetry provider.
    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// Create a new tool invoke span.
    pub fn tool_invoke_span(&self, tool_id: &str, transport: &str, caller_id: Option<&str>) -> Span {
        let span = span!(
            Level::INFO,
            "mcp.tool.invoke",
            tool_id = tool_id,
            transport = transport,
            caller_id = caller_id.unwrap_or("unknown"),
            span_id = %Uuid::new_v4(),
        );
        span
    }

    /// Create a transport reconnect span.
    pub fn transport_reconnect_span(&self, transport: &str, attempt: u32) -> Span {
        span!(
            Level::INFO,
            "mcp.transport.reconnect",
            transport = transport,
            attempt = attempt,
        )
    }

    /// Create a retry span.
    pub fn retry_span(&self, tool_id: &str, attempt: u32, max_retries: u32) -> Span {
        span!(
            Level::INFO,
            "mcp.retry.attempt",
            tool_id = tool_id,
            attempt = attempt,
            max_retries = max_retries,
        )
    }
}

/// Span names for MCP operations.
pub mod span_names {
    pub const TOOL_INVOKE: &str = "mcp.tool.invoke";
    pub const TRANSPORT_RECONNECT: &str = "mcp.transport.reconnect";
    pub const RETRY_ATTEMPT: &str = "mcp.retry.attempt";
    pub const CIRCUIT_TRIP: &str = "mcp.circuit.trip";
    pub const CONFIG_RELOAD: &str = "mcp.config.reload";
}

/// Attribute keys for consistent telemetry.
pub mod attrs {
    pub const TOOL_ID: &str = "mcp.tool.id";
    pub const TOOL_TRANSPORT: &str = "mcp.tool.transport";
    pub const RETRY_ATTEMPT: &str = "mcp.retry.attempt";
    pub const RETRY_MAX: &str = "mcp.retry.max";
    pub const ERROR_TYPE: &str = "error.type";
    pub const ERROR_MESSAGE: &str = "error.message";
    pub const DURATION_MS: &str = "duration.ms";
    pub const PAYLOAD_HASH: &str = "mcp.payload.hash";
    pub const CALLER_ID: &str = "mcp.caller.id";
    pub const CIRCUIT_STATE: &str = "mcp.circuit.state";
}

/// Record error on a tracing span.
pub fn record_error_on_span(span: &tracing::Span, err: &anyhow::Error) {
    span.record("error.type", err.to_string());
    span.record("error.message", format!("{:?}", err));
}

/// Record payload hash on a tracing span.
pub fn record_payload_hash(span: &tracing::Span, payload: &[u8]) {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(payload);
    let hash = format!("{:x}", hasher.finalize());
    span.record("payload.hash", &hash);
}

/// Record duration on a tracing span.
pub fn record_duration_ms(span: &tracing::Span, duration: Duration) {
    span.record("duration.ms", duration.as_millis() as i64);
}

pub mod propagation {
    // Placeholder for future HTTP/SSE context propagation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn telemetry_init_and_shutdown() {
        let config = TelemetryConfig {
            service_name: "test-mcp".into(),
            otlp_endpoint: None,
            sample_ratio: 1.0,
            export_timeout_secs: 5,
        };
        let telemetry = Telemetry::init(config).unwrap();
        telemetry.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn span_creation() {
        let config = TelemetryConfig::default();
        let telemetry = Telemetry::init(config).unwrap();
        let _span = telemetry.tool_invoke_span("test-tool", "stdio", Some("caller-123"));
        telemetry.shutdown().await.unwrap();
    }
}