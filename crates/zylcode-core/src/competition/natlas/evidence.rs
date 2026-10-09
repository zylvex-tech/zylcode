//! Evidence for N-ATLAS invocations.
//!
//! The competition will eventually have to *prove* genuine N-ATLAS
//! integration. That proof is assembled from records like these, so the shape
//! is defined now, before the first real call, rather than retro-fitted.
//!
//! # What a record may and may not contain
//!
//! It may contain: when, which provider and model, the provider's request id,
//! a coarse request classification, the outcome, the latency, what ZylCode did
//! as a result, and how that was verified.
//!
//! It may **not** contain an API key or any other secret. Every free-text field
//! that could echo a secret is passed through [`redact_secrets`] before it is
//! stored, and the count of redactions is recorded so a reader can see that
//! redaction happened rather than trusting it silently.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

use crate::claim::EvidenceKind;
use crate::evidence_graph::{EvidenceNode, NodeKind};

use super::types::{NatlasError, NatlasResponse, NatlasStatus};

/// The provider name recorded on every N-ATLAS evidence record.
pub const NATLAS_PROVIDER: &str = "natlas";

/// Replace every occurrence of each secret with `[REDACTED]`.
///
/// Secrets shorter than 4 characters are ignored: redacting a 1–3 character
/// string would corrupt unrelated text while protecting nothing. Returns the
/// redacted text and the number of replacements, so the caller can record that
/// redaction occurred.
pub fn redact_secrets(text: &str, secrets: &[&str]) -> (String, usize) {
    let mut out = text.to_string();
    let mut count = 0usize;
    for secret in secrets {
        if secret.len() < 4 {
            continue;
        }
        let hits = out.matches(*secret).count();
        if hits > 0 {
            out = out.replace(*secret, "[REDACTED]");
            count += hits;
        }
    }
    (out, count)
}

/// A non-secret record of one N-ATLAS invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatlasEvidence {
    pub timestamp: DateTime<Utc>,
    /// Always [`NATLAS_PROVIDER`]. Present so a mixed-provider log is unambiguous.
    pub provider: String,
    /// Model identity as configured / as reported. Never a guess.
    pub model: String,
    pub request_id: Option<String>,
    /// Coarse classification of the request (see `NatlasRequest::classification`).
    pub request_classification: String,
    /// BCP-47-ish language tag of the developer's instruction, when the caller
    /// states one (`en-NG`, `ha`, `yo`, `ig`, …).
    ///
    /// This mirrors the `language` field the TypeScript SDK already records. It
    /// is `None` when the caller does not supply one — never guessed from the
    /// text, because guessing a language is exactly the kind of unmeasured
    /// claim this record exists to prevent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub status: NatlasStatus,
    pub http_status: Option<u16>,
    pub latency_ms: u64,
    pub success: bool,
    pub error_code: Option<String>,
    /// Redacted before storage.
    pub error_detail: Option<String>,
    /// What ZylCode did as a result of the response. Filled by the caller.
    pub resulting_operation: Option<String>,
    /// How the resulting operation was verified. Filled by the caller.
    pub verification: Option<String>,
    pub prompt_chars: usize,
    pub response_chars: usize,
    /// How many secret occurrences were scrubbed from this record.
    pub secret_redactions: usize,
    /// Explicit record of what is *not* known. Mirrors the evidence graph's
    /// first-class `unknowns` field: a record that can only state what is known
    /// cannot be trusted about what is not.
    pub unknowns: Vec<String>,
}

impl NatlasEvidence {
    /// Build a record for a failed or blocked invocation.
    #[allow(clippy::too_many_arguments)]
    pub fn for_failure(
        model: impl Into<String>,
        classification: impl Into<String>,
        error: &NatlasError,
        latency_ms: u64,
        prompt_chars: usize,
        secrets: &[&str],
    ) -> Self {
        let (detail, redactions) = redact_secrets(&error.to_string(), secrets);
        let status = match error {
            NatlasError::NotConfigured { .. } => NatlasStatus::NotConfigured,
            NatlasError::BlockedNatlasAccess { .. } => NatlasStatus::BlockedNatlasAccess,
            _ => NatlasStatus::Failed,
        };
        Self {
            timestamp: Utc::now(),
            provider: NATLAS_PROVIDER.to_string(),
            model: model.into(),
            request_id: None,
            request_classification: classification.into(),
            language: None,
            status,
            http_status: None,
            latency_ms,
            success: false,
            error_code: Some(error.code().to_string()),
            error_detail: Some(detail),
            resulting_operation: None,
            verification: None,
            prompt_chars,
            response_chars: 0,
            secret_redactions: redactions,
            unknowns: vec!["no N-ATLAS response was received".to_string()],
        }
    }

    /// Build a record for a successful invocation.
    pub fn for_success(
        response: &NatlasResponse,
        classification: impl Into<String>,
        latency_ms: u64,
        prompt_chars: usize,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            provider: NATLAS_PROVIDER.to_string(),
            model: response.model.clone(),
            request_id: response.request_id.clone(),
            request_classification: classification.into(),
            language: None,
            status: NatlasStatus::Succeeded,
            http_status: Some(200),
            latency_ms,
            success: true,
            error_code: None,
            error_detail: None,
            resulting_operation: None,
            verification: None,
            prompt_chars,
            response_chars: response.text.len(),
            secret_redactions: 0,
            unknowns: Vec::new(),
        }
    }

    /// Attach what ZylCode did as a result, and how it was checked.
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.resulting_operation = Some(operation.into());
        self
    }

    /// Record the language tag of the developer's instruction.
    ///
    /// Set by the caller that actually knows the language (e.g. the CLI or the
    /// multilingual harness). Never inferred from the text.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        let tag = language.into();
        let tag = tag.trim().to_string();
        self.language = if tag.is_empty() { None } else { Some(tag) };
        self
    }

    pub fn with_verification(mut self, verification: impl Into<String>) -> Self {
        self.verification = Some(verification.into());
        self
    }

    /// Record something this invocation does not establish.
    pub fn with_unknown(mut self, unknown: impl Into<String>) -> Self {
        self.unknowns.push(unknown.into());
        self
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Append this record as one JSON line to a JSONL file, creating it if
    /// necessary. The parent directory is created too.
    pub fn append_jsonl(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(file, "{}", self.to_json())?;
        Ok(())
    }

    /// Project this record into the existing evidence graph.
    ///
    /// An N-ATLAS call is a *decision* the system made, so it becomes a
    /// [`NodeKind::Decision`] node whose `model` field carries the provider and
    /// model identity. It is **not** a `ToolCall`: no ZylCode tool ran.
    pub fn as_graph_node(&self, actor: &str) -> EvidenceNode {
        let summary = format!(
            "N-ATLAS {} ({}): {}",
            self.status.as_str(),
            self.request_classification,
            self.error_code
                .clone()
                .unwrap_or_else(|| format!("{} chars returned", self.response_chars))
        );
        EvidenceNode::new(NodeKind::Decision, summary, actor)
            .with_model(format!("{}/{}", self.provider, self.model))
            .with_evidence(
                EvidenceKind::Other,
                format!("{NATLAS_PROVIDER}:{}", self.timestamp.to_rfc3339()),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_removes_the_secret_and_counts_it() {
        let (out, n) = redact_secrets("token=abc123SECRET and abc123SECRET again", &["abc123SECRET"]);
        assert_eq!(n, 2);
        assert!(!out.contains("abc123SECRET"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn short_secrets_are_left_alone() {
        // A 3-char "secret" would corrupt unrelated text; it is skipped.
        let (out, n) = redact_secrets("the cat sat", &["cat"]);
        assert_eq!(n, 0);
        assert_eq!(out, "the cat sat");
    }

    #[test]
    fn failure_evidence_redacts_the_key_from_the_error_text() {
        let err = NatlasError::Transport {
            message: "connect failed with key sk-super-secret-value".to_string(),
        };
        let ev = NatlasEvidence::for_failure(
            "m",
            "general",
            &err,
            12,
            100,
            &["sk-super-secret-value"],
        );
        let detail = ev.error_detail.unwrap();
        assert!(!detail.contains("sk-super-secret-value"));
        assert_eq!(ev.secret_redactions, 1);
        assert!(!ev.success);
        assert_eq!(ev.status, NatlasStatus::Failed);
    }

    #[test]
    fn blocked_evidence_is_labelled_blocked_not_failed() {
        let err = NatlasError::BlockedNatlasAccess {
            reason: "no access".to_string(),
        };
        let ev = NatlasEvidence::for_failure("m", "general", &err, 0, 10, &[]);
        assert_eq!(ev.status, NatlasStatus::BlockedNatlasAccess);
        assert_eq!(ev.error_code.as_deref(), Some("BLOCKED_NATLAS_ACCESS"));
    }

    #[test]
    fn evidence_round_trips_through_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("natlas.jsonl");
        let err = NatlasError::Timeout { timeout_ms: 5 };
        let ev = NatlasEvidence::for_failure("m", "general", &err, 5, 10, &[]);
        ev.append_jsonl(&path).unwrap();
        ev.append_jsonl(&path).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(body.lines().count(), 2);
        assert!(body.contains("TIMEOUT"));
    }

    #[test]
    fn graph_node_carries_provider_and_model_identity() {
        let err = NatlasError::Timeout { timeout_ms: 5 };
        let ev = NatlasEvidence::for_failure("natlas-model-x", "general", &err, 5, 10, &[]);
        let node = ev.as_graph_node("tester");
        assert_eq!(node.kind, NodeKind::Decision);
        assert_eq!(node.model.as_deref(), Some("natlas/natlas-model-x"));
        assert_eq!(node.actor, "tester");
    }

    // --- NAT-A-004 regression: language is recorded, never guessed -----------

    #[test]
    fn language_is_absent_until_the_caller_states_it() {
        let err = NatlasError::Timeout { timeout_ms: 5 };
        let ev = NatlasEvidence::for_failure("m", "general", &err, 5, 10, &[]);
        assert_eq!(ev.language, None, "language must never be inferred");
    }

    #[test]
    fn language_is_recorded_when_stated_and_blank_is_ignored() {
        let err = NatlasError::Timeout { timeout_ms: 5 };
        let ev = NatlasEvidence::for_failure("m", "general", &err, 5, 10, &[])
            .with_language("ig");
        assert_eq!(ev.language.as_deref(), Some("ig"));

        let blank = NatlasEvidence::for_failure("m", "general", &err, 5, 10, &[])
            .with_language("   ");
        assert_eq!(blank.language, None, "a blank tag is not a language");
    }

    #[test]
    fn language_survives_the_jsonl_round_trip() {
        let err = NatlasError::Timeout { timeout_ms: 5 };
        let ev =
            NatlasEvidence::for_failure("m", "general", &err, 5, 10, &[]).with_language("yo");
        let value = ev.to_json();
        assert_eq!(value["language"], "yo");
    }
}
