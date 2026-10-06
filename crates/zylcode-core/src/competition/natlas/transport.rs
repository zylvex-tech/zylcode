//! The N-ATLAS transport boundary.
//!
//! A [`NatlasTransport`] is the single place where bytes cross to the external
//! service. Everything above it — parsing, evidence, redaction, the factory
//! handoff — is deterministic and testable without the service.
//!
//! # Why the only shipped transport blocks
//!
//! Implementing a real transport means committing to N-ATLAS's endpoint path,
//! authentication scheme and response shape. Those are not documented to us.
//! Writing them anyway would be inventing interface details — the exact thing
//! the directive forbids — so Phase C0 ships [`BlockedNatlasTransport`] and
//! leaves the real one to Phase C1, once the official contract is known.
//!
//! # Test doubles
//!
//! A test double implements this same trait and lives **only in test code**. It
//! is named `Mock*` and its success is never competition evidence. The trait
//! being public is what makes that possible without shipping a mock in the
//! library.

use super::config::NatlasConfig;
use super::types::{NatlasError, NatlasRequest};

/// A raw answer from the transport, before interpretation.
///
/// The body is kept as text so the client owns parsing and can report a
/// [`NatlasError::MalformedResponse`] precisely rather than losing the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatlasRawResponse {
    pub status: u16,
    pub body: String,
}

/// The external boundary. One method; no default implementation.
#[async_trait::async_trait]
pub trait NatlasTransport: Send + Sync {
    /// Send one request. Must not swallow the reason for a failure.
    async fn send(
        &self,
        config: &NatlasConfig,
        request: &NatlasRequest,
    ) -> Result<NatlasRawResponse, NatlasError>;
}

/// The Phase C0 transport: it always refuses, with a stated reason.
///
/// This is **not** a mock and **not** a failure mode — it is the truthful
/// representation of an integration whose access has not been granted. A
/// blocked result carries no claim about what N-ATLAS would have returned.
pub struct BlockedNatlasTransport {
    reason: String,
}

impl BlockedNatlasTransport {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    /// The default reason, naming the actual blocker.
    pub fn default_reason() -> Self {
        Self::new(
            "no verified N-ATLAS transport exists: endpoint path, authentication \
             scheme and response shape are undocumented to this project",
        )
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[async_trait::async_trait]
impl NatlasTransport for BlockedNatlasTransport {
    async fn send(
        &self,
        _config: &NatlasConfig,
        _request: &NatlasRequest,
    ) -> Result<NatlasRawResponse, NatlasError> {
        Err(NatlasError::BlockedNatlasAccess {
            reason: self.reason.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> NatlasConfig {
        NatlasConfig::from_parts("https://example.invalid", "/v1/infer", "test-model", "secret")
            .unwrap()
    }

    #[tokio::test]
    async fn blocked_transport_always_reports_blocked_natlas_access() {
        let t = BlockedNatlasTransport::default_reason();
        let err = t
            .send(&config(), &NatlasRequest::new("do a thing", "sys"))
            .await
            .unwrap_err();
        assert!(err.is_blocked(), "must be a blocked error, got {err:?}");
        assert_eq!(err.code(), "BLOCKED_NATLAS_ACCESS");
    }

    #[tokio::test]
    async fn blocked_transport_never_returns_a_body() {
        let t = BlockedNatlasTransport::default_reason();
        assert!(
            t.send(&config(), &NatlasRequest::new("x", "y"))
                .await
                .is_err(),
            "a blocked transport must not fabricate a response"
        );
    }
}
