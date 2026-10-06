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
