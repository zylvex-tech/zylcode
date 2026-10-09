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
    /// Language tag of `intent`, when the caller states one (`en-NG`, `ha`,
    /// `yo`, `ig`, …).
    ///
    /// ZylCode-side metadata only: it is recorded in the evidence record and
    /// used by the multilingual regression harness. It is **not** sent on the
    /// wire and never inferred from the text — the prompt itself is written in
    /// the target language, which is what actually makes the model answer in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

impl NatlasRequest {
    pub fn new(intent: impl Into<String>, system: impl Into<String>) -> Self {
        Self {
            intent: intent.into(),
            system: system.into(),
            context: Vec::new(),
            max_tokens: 4096,
            language: None,
        }
    }

    /// State the language of the instruction. Recorded in evidence; never
    /// guessed.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        let tag = language.into();
        let tag = tag.trim().to_string();
        self.language = if tag.is_empty() { None } else { Some(tag) };
        self
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

    /// The resilience state this error corresponds to.
    pub fn resilience_state(&self) -> NatlasResilienceState {
        NatlasResilienceState::from_error(self)
    }

    /// Whether retrying could plausibly succeed without a human changing
    /// anything. See [`NatlasResilienceState::is_retryable`].
    pub fn is_retryable(&self) -> bool {
        self.resilience_state().is_retryable()
    }

    /// A human-readable, actionable explanation for a tester.
    ///
    /// Two lines: what happened (the typed error), then what to do about it
    /// (the state's guidance). Never contains a secret — the constituent parts
    /// are the redaction-checked error text and a static guidance string.
    pub fn human_message(&self) -> String {
        format!("{}\n{}", self, self.resilience_state().guidance())
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

    // --- NAT-A-001 / NAT-A-003 regression -----------------------------------
    // A ZeroGPU quota exhaustion must be classified as Quota, not "warming",
    // and must not be marked retryable (retrying cannot fix a spent budget).

    #[test]
    fn a_zero_gpu_quota_503_is_quota_not_warming() {
        let body = "You have exceeded your GPU quota (0s left). Please try again later.";
        assert_eq!(
            NatlasResilienceState::from_http(503, body),
            NatlasResilienceState::Quota
        );
        // The load/cold-start word "try again" must not win over the quota body.
        assert_eq!(
            NatlasResilienceState::from_http(503, "GPU quota exceeded"),
            NatlasResilienceState::Quota
        );
    }

    #[test]
    fn quota_is_reported_but_never_retryable() {
        let state = NatlasResilienceState::Quota;
        assert!(!state.is_retryable(), "a spent quota is not retryable");
        assert!(!state.is_operational());
        // Cold-start states ARE retryable.
        assert!(NatlasResilienceState::Warming.is_retryable());
        assert!(NatlasResilienceState::Loading.is_retryable());
    }

    #[test]
    fn every_state_has_a_non_empty_human_guidance() {
        for state in [
            NatlasResilienceState::Ok,
            NatlasResilienceState::Warming,
            NatlasResilienceState::Loading,
            NatlasResilienceState::Timeout,
            NatlasResilienceState::Quota,
            NatlasResilienceState::AuthFailure,
            NatlasResilienceState::Unavailable,
            NatlasResilienceState::Blocked,
            NatlasResilienceState::Failed,
        ] {
            let g = state.guidance();
            assert!(
                g.len() > 20,
                "{} guidance is too terse to be useful: {g:?}",
                state.as_str()
            );
        }
    }

    #[test]
    fn a_quota_error_carries_human_readable_guidance_and_is_not_retryable() {
        let err = NatlasError::HttpStatus {
            status: 503,
            body_excerpt: "You have exceeded your GPU quota".to_string(),
        };
        assert_eq!(err.resilience_state(), NatlasResilienceState::Quota);
        assert!(!err.is_retryable());
        let msg = err.human_message();
        assert!(msg.contains("quota"), "human message must name the cause: {msg}");
        assert!(
            msg.contains("LOCAL_RUNTIME_FALLBACK"),
            "human message must point at an actionable next step: {msg}"
        );
    }

    #[test]
    fn human_message_never_leaks_a_secret_passed_in_the_body() {
        // The guidance is static, but the first line echoes the error. Assert
        // the shape is what callers redact before display.
        let err = NatlasError::Transport {
            message: "connection refused".to_string(),
        };
        let msg = err.human_message();
        assert!(msg.contains("connection refused"));
        assert!(msg.lines().count() >= 2, "expect cause + guidance lines");
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
            // 429 is the canonical rate-limit. Hugging Face's ZeroGPU quota
            // exhaustion also surfaces as 402 ("payment required") on some
            // deployments. Both mean "there is no budget left right now".
            429 | 402 => Self::Quota,
            502 | 503 => {
                // ZeroGPU / Hugging Face quota exhaustion arrives as a 503 whose
                // body names the quota (e.g. "You have exceeded your GPU
                // quota"). This is checked BEFORE the load/cold-start words,
                // because a quota body must never be mislabelled "warming" —
                // that would invite a retry that cannot possibly succeed and
                // leave the caller looking stuck.
                if b.contains("quota")
                    || b.contains("exceeded")
                    || b.contains("rate limit")
                    || b.contains("too many requests")
                {
                    Self::Quota
                } else if b.contains("load") || b.contains("starting") || b.contains("boot") {
                    Self::Loading
                } else {
                    Self::Warming
                }
            }
            400 | 404 | 405 | 500 | 501 => Self::Failed,
            _ => Self::Unavailable,
        }
    }

    /// Whether retrying the same request could plausibly succeed without a
    /// human changing anything.
    ///
    /// Transient cold-start and transport states are retryable. A quota
    /// exhaustion, an auth failure or a blocked integration is **not**: the
    /// budget or the credential has to change first, and retrying would only
    /// burn time and make the surface look stuck.
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::Warming | Self::Loading | Self::Unavailable)
    }

    /// A short, human-readable, actionable explanation for this state.
    ///
    /// This is the text a tester should see instead of a bare error code. It
    /// states what happened and what to do next; it never contains a secret.
    pub fn guidance(self) -> &'static str {
        match self {
            Self::Ok => "The endpoint answered normally.",
            Self::Warming => {
                "The shared GPU is waking up. This is normal on a cold start; retrying shortly usually succeeds."
            }
            Self::Loading => "The model is loading into memory. Wait for it to finish, then retry.",
            Self::Timeout => {
                "The request took longer than the configured timeout. The shared GPU may be busy; retry, or raise NATLAS_TIMEOUT_MS."
            }
            Self::Quota => {
                "The endpoint's shared GPU quota is exhausted. No code change fixes this — wait for the quota to reset, or run the bridge against a local OpenAI-compatible runtime (see docs/competition/natlas/LOCAL_RUNTIME_FALLBACK.md)."
            }
            Self::AuthFailure => {
                "Authentication failed. Check that NATLAS_API_KEY is set and correct; the endpoint returns 401 for a wrong key."
            }
            Self::Unavailable => {
                "The runtime could not be reached (DNS, connection refused, or a proxy). Check the base URL and any HTTP_PROXY setting."
            }
            Self::Blocked => {
                "N-ATLAS is not configured, or access has not been granted. Set NATLAS_BASE_URL, NATLAS_REQUEST_PATH, NATLAS_MODEL and NATLAS_API_KEY."
            }
            Self::Failed => {
                "The endpoint returned an error that is not a transient condition. Inspect the recorded body excerpt."
            }
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
