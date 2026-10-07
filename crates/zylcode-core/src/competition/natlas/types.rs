//! N-ATLAS request, response, error and status types.
//!
//! These are **our** types. They describe what ZylCode sends to the boundary
//! and what it needs back; they are not a claim about N-ATLAS's own wire
//! format, which is unknown and therefore left to the transport.

use serde::{Deserialize, Serialize};

/// A slice of repository context handed to the model.
///
/// Context comes from ZylCode's existing repository intelligence, not from the
/// model, so a request carries provenance with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatlasContextChunk {
    /// Workspace-relative path the excerpt came from.
    pub path: String,
    pub excerpt: String,
}

/// What ZylCode asks N-ATLAS to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatlasRequest {
    /// The developer's request, verbatim.
    pub intent: String,
    /// System preamble. Carries the response contract the model must honour.
    pub system: String,
    /// Repository context, ordered by the caller.
    pub context: Vec<NatlasContextChunk>,
    pub max_tokens: u32,
}

impl NatlasRequest {
    pub fn new(intent: impl Into<String>, system: impl Into<String>) -> Self {
        Self {
            intent: intent.into(),
            system: system.into(),
            context: Vec::new(),
            max_tokens: 4096,
        }
    }

    pub fn with_context(mut self, path: impl Into<String>, excerpt: impl Into<String>) -> Self {
        self.context.push(NatlasContextChunk {
            path: path.into(),
            excerpt: excerpt.into(),
        });
        self
    }

    /// A deterministic, coarse classification of the request.
    ///
    /// This is **our** label for evidence and reporting — a simple keyword
    /// heuristic, not an N-ATLAS concept and not a model output. It exists so
    /// an evidence record can say *what kind of request* it was without storing
    /// the whole prompt.
    pub fn classification(&self) -> &'static str {
        let t = self.intent.to_ascii_lowercase();
        let has = |words: &[&str]| words.iter().any(|w| t.contains(w));
        if has(&["implement", "add ", "fix", "refactor", "write ", "create ", "rename"]) {
            "implementation"
        } else if has(&["test", "assert", "coverage"]) {
            "testing"
        } else if has(&["explain", "why", "what is", "how does", "document", "describe"]) {
            "analysis"
        } else {
            "general"
        }
    }
}

/// Token usage, when the provider reports it. Absence is recorded as `None`,
/// never as a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NatlasUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

/// A parsed N-ATLAS response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatlasResponse {
    /// The model's text output.
    pub text: String,
    /// Model identity **as reported by the provider**. Recorded, not assumed.
    pub model: String,
    /// Provider-supplied request identifier, when present.
    pub request_id: Option<String>,
    pub usage: NatlasUsage,
    pub finish_reason: Option<String>,
}

/// Why an N-ATLAS invocation did not produce a response.
///
/// Every variant is a *stated reason*. There is no catch-all "something went
/// wrong", because an unstated failure cannot be acted on or reported honestly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NatlasError {
    /// Required configuration is absent.
    NotConfigured { missing: Vec<&'static str> },
    /// N-ATLAS cannot be reached: no verified transport, or access not granted.
    BlockedNatlasAccess { reason: String },
    /// The transport itself failed (connection, DNS, TLS).
    Transport { message: String },
    /// The request exceeded the configured timeout.
    Timeout { timeout_ms: u64 },
    /// The service answered with a non-success status.
    HttpStatus { status: u16, body_excerpt: String },
    /// The response was not valid against the expected contract.
    MalformedResponse { detail: String },
}

impl NatlasError {
    /// A stable, greppable code for evidence records.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotConfigured { .. } => "NOT_CONFIGURED",
            Self::BlockedNatlasAccess { .. } => "BLOCKED_NATLAS_ACCESS",
            Self::Transport { .. } => "TRANSPORT",
            Self::Timeout { .. } => "TIMEOUT",
            Self::HttpStatus { .. } => "HTTP_STATUS",
            Self::MalformedResponse { .. } => "MALFORMED_RESPONSE",
        }
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::BlockedNatlasAccess { .. })
    }
}

impl std::fmt::Display for NatlasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured { missing } => {
                write!(f, "N-ATLAS not configured; missing: {}", missing.join(", "))
            }
            Self::BlockedNatlasAccess { reason } => write!(f, "N-ATLAS blocked: {reason}"),
            Self::Transport { message } => write!(f, "N-ATLAS transport error: {message}"),
            Self::Timeout { timeout_ms } => {
                write!(f, "N-ATLAS request timed out after {timeout_ms} ms")
            }
            Self::HttpStatus { status, body_excerpt } => {
                write!(f, "N-ATLAS returned HTTP {status}: {body_excerpt}")
            }
            Self::MalformedResponse { detail } => {
                write!(f, "N-ATLAS response did not match the expected contract: {detail}")
            }
        }
    }
}

impl std::error::Error for NatlasError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_status_classifies_auth_quota_and_cold_starts() {
        assert_eq!(
            NatlasResilienceState::from_http(401, ""),
            NatlasResilienceState::AuthFailure
        );
        assert_eq!(
            NatlasResilienceState::from_http(429, "quota exceeded"),
            NatlasResilienceState::Quota
        );
        // A 503 that names a load is "loading"; a bare one is "warming".
        assert_eq!(
            NatlasResilienceState::from_http(503, "model is loading"),
            NatlasResilienceState::Loading
        );
        assert_eq!(
            NatlasResilienceState::from_http(502, "bad gateway"),
            NatlasResilienceState::Warming
        );
        assert_eq!(
            NatlasResilienceState::from_http(500, "boom"),
            NatlasResilienceState::Failed
        );
    }

    #[test]
    fn error_classification_matches_transport_and_blocked() {
        assert_eq!(
            NatlasResilienceState::from_error(&NatlasError::Transport {
                message: "conn refused".into()
            }),
            NatlasResilienceState::Unavailable
        );
        assert_eq!(
            NatlasResilienceState::from_error(&NatlasError::Timeout { timeout_ms: 5 }),
            NatlasResilienceState::Timeout
        );
        assert_eq!(
            NatlasResilienceState::from_error(&NatlasError::BlockedNatlasAccess {
                reason: "no transport".into()
            }),
            NatlasResilienceState::Blocked
        );
        // An HTTP error routes through the same mapper.
        assert_eq!(
            NatlasResilienceState::from_error(&NatlasError::HttpStatus {
                status: 401,
                body_excerpt: "".into()
            }),
            NatlasResilienceState::AuthFailure
        );
    }

    #[test]
    fn only_ok_is_operational() {
        assert!(NatlasResilienceState::Ok.is_operational());
        assert!(!NatlasResilienceState::Warming.is_operational());
        assert!(!NatlasResilienceState::AuthFailure.is_operational());
    }
}

/// The outcome state of an N-ATLAS invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatlasStatus {
    /// Required configuration is absent.
    NotConfigured,
    /// Configured, but N-ATLAS access is not available.
    BlockedNatlasAccess,
    /// A response was received and parsed.
    Succeeded,
    /// A call was attempted and failed for a stated reason.
    Failed,
}

impl NatlasStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotConfigured => "NOT_CONFIGURED",
            Self::BlockedNatlasAccess => "BLOCKED_NATLAS_ACCESS",
            Self::Succeeded => "SUCCEEDED",
            Self::Failed => "FAILED",
        }
    }

    /// Whether this state proves a real N-ATLAS response.
    ///
    /// Only [`NatlasStatus::Succeeded`] does — and only when it came from a
    /// real transport. A mock-driven success is still `Succeeded` at the type
    /// level, so **callers must not treat this as competition evidence** unless
    /// the transport was the real one.
    pub fn is_verified_success(self) -> bool {
        matches!(self, Self::Succeeded)
    }
}

/// The honest state of the N-ATLAS endpoint, as observed by ZylCode.
///
/// Every variant is a *stated* condition. There is deliberately **no** "falling
/// back to another model" state, because substituting a different model would be
/// fabrication. When the endpoint is not `Ok`, ZylCode reports the true state
/// and does nothing with a synthetic answer.
///
/// This is what the UI and evidence surface should show, so a reviewer sees
/// "warming" or "auth_failure" rather than a silent degraded experience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatlasResilienceState {
    /// A response was received and parsed successfully.
    Ok,
    /// Endpoint is cold; shared GPU is spinning up (typically HTTP 502/503 with
    /// no load message). No model is substituted.
    Warming,
    /// Model is loading into memory (HTTP 503 that names a load). No model is
    /// substituted.
    Loading,
    /// The request exceeded our configured timeout.
    Timeout,
    /// Rate-limited or out of quota (HTTP 429), or a known ZeroGPU quota message.
    Quota,
    /// Authentication/authorisation failed (HTTP 401/403).
    AuthFailure,
    /// Transport-level failure: DNS, connection refused, proxy error.
    Unavailable,
    /// Configured values are absent; the integration cannot be attempted.
    Blocked,
    /// Any other failure (HTTP 4xx/5xx not otherwise classified, bad body, …).
    Failed,
}

impl NatlasResilienceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warming => "WARMING",
            Self::Loading => "LOADING",
            Self::Timeout => "TIMEOUT",
            Self::Quota => "QUOTA",
            Self::AuthFailure => "AUTH_FAILURE",
            Self::Unavailable => "UNAVAILABLE",
            Self::Blocked => "BLOCKED",
            Self::Failed => "FAILED",
        }
    }

    /// True only for [`NatlasResilienceState::Ok`]. Every other state means the
    /// request did not yield a usable N-ATLAS answer.
    pub fn is_operational(self) -> bool {
        matches!(self, Self::Ok)
    }

    /// Classify an HTTP status (with an optional body excerpt that may name a
    /// load) into a resilience state. Never returns `Ok` — use
    /// [`NatlasResilienceState::Ok`] directly for a 2xx.
    pub fn from_http(status: u16, body_excerpt: &str) -> Self {
        let b = body_excerpt.to_ascii_lowercase();
        match status {
            401 | 403 => Self::AuthFailure,
            429 => Self::Quota,
            502 | 503 => {
                if b.contains("load") || b.contains("starting") || b.contains("boot") {
                    Self::Loading
                } else {
                    Self::Warming
                }
            }
            400 | 404 | 405 | 500 | 501 => Self::Failed,
            _ => Self::Unavailable,
        }
    }

    /// Classify a [`NatlasError`] into a resilience state.
    pub fn from_error(error: &NatlasError) -> Self {
        match error {
            NatlasError::NotConfigured { .. } => Self::Blocked,
            NatlasError::BlockedNatlasAccess { .. } => Self::Blocked,
            NatlasError::Transport { .. } => Self::Unavailable,
            NatlasError::Timeout { .. } => Self::Timeout,
            NatlasError::HttpStatus { status, body_excerpt } => {
                Self::from_http(*status, body_excerpt)
            }
            NatlasError::MalformedResponse { .. } => Self::Failed,
        }
    }
}
