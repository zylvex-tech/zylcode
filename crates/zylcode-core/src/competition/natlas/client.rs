//! The N-ATLAS client: invoke → interpret → evidence.
//!
//! The client is the deterministic half of the integration. Given a transport
//! it will: enforce the timeout, classify the request, interpret the raw
//! answer, redact secrets, and produce an [`NatlasEvidence`] record. None of
//! that depends on N-ATLAS being reachable, which is why it can be tested today.

use std::time::{Duration, Instant};

use super::config::NatlasConfig;
use super::evidence::NatlasEvidence;
use super::transport::{BlockedNatlasTransport, NatlasRawResponse, NatlasTransport};
use super::types::{NatlasError, NatlasRequest, NatlasResponse, NatlasStatus, NatlasUsage};

/// Maximum number of response-body characters retained in an error record.
const BODY_EXCERPT_LIMIT: usize = 300;

/// Everything one invocation produced.
#[derive(Debug, Clone)]
pub struct NatlasInvocation {
    pub status: NatlasStatus,
    pub response: Option<NatlasResponse>,
    pub error: Option<NatlasError>,
    pub evidence: NatlasEvidence,
}

impl NatlasInvocation {
    pub fn succeeded(&self) -> bool {
        self.status == NatlasStatus::Succeeded
    }

    pub fn blocked(&self) -> bool {
        self.status == NatlasStatus::BlockedNatlasAccess
    }
}

/// A client bound to a configuration and a transport.
pub struct NatlasClient<T: NatlasTransport> {
    config: NatlasConfig,
    transport: T,
    actor: String,
}

impl<T: NatlasTransport> NatlasClient<T> {
    pub fn new(config: NatlasConfig, transport: T, actor: impl Into<String>) -> Self {
        Self {
            config,
            transport,
            actor: actor.into(),
        }
    }

    /// Build from the environment. Fails with [`NatlasError::NotConfigured`]
    /// when a required variable is absent — it never substitutes a default.
    pub fn from_env(transport: T, actor: impl Into<String>) -> Result<Self, NatlasError> {
        NatlasConfig::from_env()
            .map(|config| Self::new(config, transport, actor))
            .map_err(|e| NatlasError::NotConfigured { missing: e.missing })
    }

    pub fn config(&self) -> &NatlasConfig {
        &self.config
    }

    pub fn actor(&self) -> &str {
        &self.actor
    }

    /// Invoke N-ATLAS once.
    ///
    /// Always returns an invocation; a failure is represented in the returned
    /// value, never by panicking and never by substituting a synthetic
    /// response.
    pub async fn invoke(&self, request: &NatlasRequest) -> NatlasInvocation {
        let started = Instant::now();
        let prompt_chars = request.intent.len()
            + request.system.len()
            + request.context.iter().map(|c| c.excerpt.len()).sum::<usize>();
        let classification = request.classification();
        let secrets = [self.config.api_key.as_str()];

        let outcome: Result<NatlasResponse, NatlasError> = match tokio::time::timeout(
            Duration::from_millis(self.config.timeout_ms),
            self.transport.send(&self.config, request),
        )
        .await
        {
            Err(_elapsed) => Err(NatlasError::Timeout {
                timeout_ms: self.config.timeout_ms,
            }),
            Ok(Err(e)) => Err(e),
            Ok(Ok(raw)) => interpret_raw(raw),
        };

        let latency_ms = started.elapsed().as_millis() as u64;

        match outcome {
            Ok(response) => NatlasInvocation {
                status: NatlasStatus::Succeeded,
                evidence: NatlasEvidence::for_success(
                    &response,
                    classification,
                    latency_ms,
                    prompt_chars,
                ),
                response: Some(response),
                error: None,
            },
            Err(error) => {
                let evidence = NatlasEvidence::for_failure(
                    self.config.model.clone(),
                    classification,
                    &error,
                    latency_ms,
                    prompt_chars,
                    &secrets,
                );
                let status = evidence.status;
                NatlasInvocation {
                    status,
                    response: None,
                    error: Some(error),
                    evidence,
                }
            }
        }
    }
}

impl NatlasClient<BlockedNatlasTransport> {
    /// The Phase C0 client: fully configured, but with no verified transport,
    /// so every invocation is honestly blocked.
    pub fn blocked(config: NatlasConfig, actor: impl Into<String>) -> Self {
        Self::new(config, BlockedNatlasTransport::default_reason(), actor)
    }

    /// Same, reading the configuration from the environment.
    pub fn from_env_blocked(actor: impl Into<String>) -> Result<Self, NatlasError> {
        Self::from_env(BlockedNatlasTransport::default_reason(), actor)
    }
}

/// Turn a raw transport answer into a parsed response.
///
/// A non-2xx status is an [`NatlasError::HttpStatus`]; a 2xx body that does not
/// match the contract is an [`NatlasError::MalformedResponse`]. Neither is
/// silently coerced into a success.
pub fn interpret_raw(raw: NatlasRawResponse) -> Result<NatlasResponse, NatlasError> {
    if !(200..300).contains(&raw.status) {
        return Err(NatlasError::HttpStatus {
            status: raw.status,
            body_excerpt: excerpt(&raw.body),
        });
    }
    parse_canonical_response(&raw.body)
}

/// Parse the **ZylCode-side response contract**.
///
/// This schema is ours: it is what we ask the model to return, and it is
/// independent of N-ATLAS's own wire format. Translating N-ATLAS bytes into
/// this shape is the transport's job and is not implemented in Phase C0.
pub fn parse_canonical_response(body: &str) -> Result<NatlasResponse, NatlasError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|e| {
        NatlasError::MalformedResponse {
            detail: format!("not valid JSON: {e}"),
        }
    })?;
    let obj = value.as_object().ok_or_else(|| NatlasError::MalformedResponse {
        detail: "expected a JSON object at the top level".to_string(),
    })?;

    let text = obj
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| NatlasError::MalformedResponse {
            detail: "missing string field `text`".to_string(),
        })?;
    if text.trim().is_empty() {
        return Err(NatlasError::MalformedResponse {
            detail: "field `text` is empty".to_string(),
        });
    }

    // Model identity is required: an evidence record without it cannot prove
    // which model answered, so a response that omits it is rejected.
    let model = obj
        .get("model")
        .and_then(|v| v.as_str())
        .filter(|m| !m.trim().is_empty())
        .ok_or_else(|| NatlasError::MalformedResponse {
            detail: "missing non-empty string field `model`".to_string(),
        })?;

    let request_id = obj
        .get("request_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let finish_reason = obj
        .get("finish_reason")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let usage = obj
        .get("usage")
        .map(|u| NatlasUsage {
            input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()),
            output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()),
        })
        .unwrap_or_default();

    Ok(NatlasResponse {
        text: text.to_string(),
        model: model.to_string(),
        request_id,
        usage,
        finish_reason,
    })
}

fn excerpt(body: &str) -> String {
    let mut s: String = body.chars().take(BODY_EXCERPT_LIMIT).collect();
    if body.chars().count() > BODY_EXCERPT_LIMIT {
        s.push('…');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_well_formed_canonical_response() {
        let body = r#"{"model":"natlas-x","request_id":"req-1","text":"hello",
                       "usage":{"input_tokens":7,"output_tokens":3},"finish_reason":"stop"}"#;
        let r = parse_canonical_response(body).unwrap();
        assert_eq!(r.text, "hello");
        assert_eq!(r.model, "natlas-x");
        assert_eq!(r.request_id.as_deref(), Some("req-1"));
        assert_eq!(r.usage.input_tokens, Some(7));
        assert_eq!(r.finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn rejects_missing_model_identity() {
        let err = parse_canonical_response(r#"{"text":"hi"}"#).unwrap_err();
        assert_eq!(err.code(), "MALFORMED_RESPONSE");
        assert!(err.to_string().contains("model"));
    }

    #[test]
    fn rejects_non_json_and_empty_text() {
        assert_eq!(
            parse_canonical_response("not json").unwrap_err().code(),
            "MALFORMED_RESPONSE"
        );
        assert_eq!(
            parse_canonical_response(r#"{"model":"m","text":"   "}"#)
                .unwrap_err()
                .code(),
            "MALFORMED_RESPONSE"
        );
    }

    #[test]
    fn non_2xx_becomes_http_status_not_a_response() {
        let raw = NatlasRawResponse {
            status: 503,
            body: "upstream unavailable".to_string(),
        };
        let err = interpret_raw(raw).unwrap_err();
        assert_eq!(err.code(), "HTTP_STATUS");
    }
}
