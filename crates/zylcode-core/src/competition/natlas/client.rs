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

/// A bounded retry policy for transient N-ATLAS failures.
///
/// Only *transient* states are retried: a cold-starting shared GPU
/// (`Warming`, `Loading`) or a transport-level blip (`Unavailable`). A spent
/// GPU quota, a bad credential or a blocked integration is **not** retried —
/// the cause has to change first, and retrying would only burn time and make
/// the surface look stuck (NAT-A-001 / NAT-A-002).
///
/// The policy is bounded twice over: a maximum attempt count **and** a
/// wall-clock budget. A run therefore always terminates with a stated outcome,
/// which is what lets a UI show a real processing state instead of an
/// indefinite spinner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Total attempts, including the first. `1` disables retrying.
    pub max_attempts: u32,
    /// Delay before the second attempt, in milliseconds.
    pub base_delay_ms: u64,
    /// Upper bound on a single delay, in milliseconds.
    pub max_delay_ms: u64,
    /// Hard wall-clock budget for all attempts, in milliseconds.
    pub budget_ms: u64,
}

impl RetryPolicy {
    /// No retrying: exactly one attempt. This is the default, so existing
    /// callers keep their previous single-shot semantics.
    pub const NONE: Self = Self {
        max_attempts: 1,
        base_delay_ms: 0,
        max_delay_ms: 0,
        budget_ms: 0,
    };

    /// The policy for an interactive / demo run: three attempts with
    /// 1s → 2s exponential backoff, inside a 30s wall-clock budget.
    pub const fn demo() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1_000,
            max_delay_ms: 8_000,
            budget_ms: 30_000,
        }
    }

    /// The delay to apply before attempt `attempt` (1-based). Attempt 1 has no
    /// delay; later attempts back off exponentially, capped at `max_delay_ms`.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt <= 1 || self.base_delay_ms == 0 {
            return Duration::from_millis(0);
        }
        let shift = (attempt - 2).min(16);
        let delay = self.base_delay_ms.saturating_mul(1u64 << shift);
        Duration::from_millis(delay.min(self.max_delay_ms))
    }

    /// The wall-clock budget as a `Duration`.
    pub fn budget(&self) -> Duration {
        Duration::from_millis(self.budget_ms)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::NONE
    }
}

/// Everything one invocation produced.
#[derive(Debug, Clone)]
pub struct NatlasInvocation {
    pub status: NatlasStatus,
    pub response: Option<NatlasResponse>,
    pub error: Option<NatlasError>,
    pub evidence: NatlasEvidence,
    /// How many attempts were made, including the first. `1` means no retry
    /// occurred. Recorded so a reader can tell "it worked first time" from
    /// "it recovered after a cold start".
    pub attempts: u32,
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
    retry: RetryPolicy,
}

impl<T: NatlasTransport> NatlasClient<T> {
    pub fn new(config: NatlasConfig, transport: T, actor: impl Into<String>) -> Self {
        Self {
            config,
            transport,
            actor: actor.into(),
            retry: RetryPolicy::NONE,
        }
    }

    /// Set the retry policy. Without this the client makes exactly one attempt,
    /// which preserves the original single-shot behaviour for every existing
    /// caller.
    pub fn with_retry_policy(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// The retry policy in force.
    pub fn retry_policy(&self) -> RetryPolicy {
        self.retry
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

    /// Invoke N-ATLAS once, retrying only transient failures per the configured
    /// [`RetryPolicy`].
    ///
    /// Always returns an invocation; a failure is represented in the returned
    /// value, never by panicking and never by substituting a synthetic
    /// response. The loop is bounded by **both** an attempt count and a
    /// wall-clock budget, so it always terminates with a stated outcome.
    pub async fn invoke(&self, request: &NatlasRequest) -> NatlasInvocation {
        let started = Instant::now();
        let prompt_chars = request.intent.len()
            + request.system.len()
            + request.context.iter().map(|c| c.excerpt.len()).sum::<usize>();
        let classification = request.classification();
        let secrets = [self.config.api_key.as_str()];
        let language = request.language.clone();

        let mut attempts: u32 = 0;
        let outcome: Result<NatlasResponse, NatlasError> = loop {
            attempts += 1;
            let result = self.attempt(request).await;

            let retryable = match &result {
                Err(e) => {
                    e.is_retryable()
                        && attempts < self.retry.max_attempts
                        && started.elapsed() < self.retry.budget()
                }
                Ok(_) => false,
            };

            if retryable {
                let delay = self.retry.delay_for_attempt(attempts + 1);
                if started.elapsed() + delay <= self.retry.budget() {
                    tokio::time::sleep(delay).await;
                    continue;
                }
            }
            break result;
        };

        let latency_ms = started.elapsed().as_millis() as u64;

        match outcome {
            Ok(response) => {
                let mut evidence = NatlasEvidence::for_success(
                    &response,
                    classification,
                    latency_ms,
                    prompt_chars,
                );
                if let Some(lang) = &language {
                    evidence = evidence.with_language(lang.clone());
                }
                NatlasInvocation {
                    status: NatlasStatus::Succeeded,
                    evidence,
                    response: Some(response),
                    error: None,
                    attempts,
                }
            }
            Err(error) => {
                let mut evidence = NatlasEvidence::for_failure(
                    self.config.model.clone(),
                    classification,
                    &error,
                    latency_ms,
                    prompt_chars,
                    &secrets,
                );
                if let Some(lang) = &language {
                    evidence = evidence.with_language(lang.clone());
                }
                let status = evidence.status;
                NatlasInvocation {
                    status,
                    response: None,
                    error: Some(error),
                    evidence,
                    attempts,
                }
            }
        }
    }

    /// One attempt: enforce the timeout, send, interpret. Never retries.
    async fn attempt(&self, request: &NatlasRequest) -> Result<NatlasResponse, NatlasError> {
        match tokio::time::timeout(
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

    // --- NAT-A-001 / NAT-A-002 regression: bounded, selective retry ----------

    use std::sync::atomic::{AtomicU32, Ordering};

    use crate::competition::natlas::types::NatlasResilienceState;

    fn test_config() -> NatlasConfig {
        NatlasConfig::from_parts("http://127.0.0.1:1", "/v1/chat/completions", "m", "k").unwrap()
    }

    /// Fails with a chosen error for the first `failures` calls, then returns a
    /// valid canonical response. Counts every call.
    struct FlakyTransport {
        failures: u32,
        calls: AtomicU32,
        error: fn() -> NatlasError,
    }

    #[async_trait::async_trait]
    impl NatlasTransport for FlakyTransport {
        async fn send(
            &self,
            _config: &NatlasConfig,
            _request: &NatlasRequest,
        ) -> Result<NatlasRawResponse, NatlasError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n < self.failures {
                return Err((self.error)());
            }
            Ok(NatlasRawResponse {
                status: 200,
                body: r#"{"text":"ok","model":"natlas-test","request_id":"r1",
                          "usage":{"input_tokens":1,"output_tokens":1},"finish_reason":"stop"}"#
                    .to_string(),
            })
        }
    }

    fn fast_policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 1,
            max_delay_ms: 2,
            budget_ms: 5_000,
        }
    }

    #[tokio::test]
    async fn a_transient_failure_is_retried_and_recovers() {
        let client = NatlasClient::new(
            test_config(),
            FlakyTransport {
                failures: 2,
                calls: AtomicU32::new(0),
                error: || NatlasError::Transport {
                    message: "cold start".into(),
                },
            },
            "tester",
        )
        .with_retry_policy(fast_policy());

        let inv = client.invoke(&NatlasRequest::new("hi", "sys")).await;
        assert!(inv.succeeded(), "must recover: {:?}", inv.error);
        assert_eq!(inv.attempts, 3, "should have retried twice");
    }

    #[tokio::test]
    async fn a_spent_quota_is_not_retried() {
        let client = NatlasClient::new(
            test_config(),
            FlakyTransport {
                failures: 9,
                calls: AtomicU32::new(0),
                error: || NatlasError::HttpStatus {
                    status: 503,
                    body_excerpt: "You have exceeded your GPU quota".into(),
                },
            },
            "tester",
        )
        .with_retry_policy(fast_policy());

        let inv = client.invoke(&NatlasRequest::new("hi", "sys")).await;
        assert!(!inv.succeeded());
        assert_eq!(inv.attempts, 1, "a spent quota must not be retried");
        assert_eq!(
            inv.error.as_ref().unwrap().resilience_state(),
            NatlasResilienceState::Quota
        );
    }

    #[tokio::test]
    async fn retries_stop_at_the_attempt_cap() {
        let client = NatlasClient::new(
            test_config(),
            FlakyTransport {
                failures: 99,
                calls: AtomicU32::new(0),
                error: || NatlasError::Transport {
                    message: "still cold".into(),
                },
            },
            "tester",
        )
        .with_retry_policy(fast_policy());

        let inv = client.invoke(&NatlasRequest::new("hi", "sys")).await;
        assert!(!inv.succeeded());
        assert_eq!(inv.attempts, 3, "must stop at max_attempts, not spin");
    }

    #[tokio::test]
    async fn the_default_policy_makes_exactly_one_attempt() {
        let client = NatlasClient::new(
            test_config(),
            FlakyTransport {
                failures: 1,
                calls: AtomicU32::new(0),
                error: || NatlasError::Transport {
                    message: "x".into(),
                },
            },
            "tester",
        );
        assert_eq!(client.retry_policy(), RetryPolicy::NONE);
        let inv = client.invoke(&NatlasRequest::new("hi", "sys")).await;
        assert_eq!(inv.attempts, 1, "default must not retry");
    }

    #[test]
    fn backoff_delays_grow_then_cap() {
        let p = RetryPolicy {
            max_attempts: 5,
            base_delay_ms: 100,
            max_delay_ms: 400,
            budget_ms: 10_000,
        };
        assert_eq!(p.delay_for_attempt(1).as_millis(), 0);
        assert_eq!(p.delay_for_attempt(2).as_millis(), 100);
        assert_eq!(p.delay_for_attempt(3).as_millis(), 200);
        assert_eq!(p.delay_for_attempt(4).as_millis(), 400);
        assert_eq!(p.delay_for_attempt(9).as_millis(), 400, "must cap at max");
    }
}
