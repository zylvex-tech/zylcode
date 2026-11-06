//! Neutral provider error classification.
//!
//! Provider modules map their own wire errors into [`ProviderError`] and the router
//! maps probe/dispatch failures into the same neutral envelope where possible. This
//! is the shared shape for "what went wrong with a provider call", not a second
//! error type that duplicates `anyhow` or the probe types.

use super::streaming::{FinishReason, ProviderError, ProviderErrorKind};
use crate::router::ProbeStatus;

impl ProviderErrorKind {
    /// Map a neutral error kind onto the probe status the settings surface already
    /// understands.
    pub fn probe_status(self) -> ProbeStatus {
        match self {
            Self::Authentication => ProbeStatus::AuthenticationFailed,
            Self::InvalidEndpoint => ProbeStatus::InvalidEndpoint,
            Self::ModelUnavailable => ProbeStatus::ModelUnavailable,
            Self::ProviderRejected => ProbeStatus::ProviderRejected,
            Self::Transport => ProbeStatus::NetworkError,
            // A truncated or malformed stream is a transport-level observation: the
            // connection worked well enough to deliver bytes, then stopped making
            // sense. The operator action is the same as for a network error.
            Self::Truncated | Self::Malformed => ProbeStatus::NetworkError,
            Self::Unknown => ProbeStatus::ProviderRejected,
        }
    }
}

impl ProviderError {
    /// Build a provider error with truncated message.
    pub fn new(kind: ProviderErrorKind, message: String) -> Self {
        let message = if message.len() > super::streaming::constants::ERROR_MESSAGE_MAX_CHARS {
            let mut s = message;
            s.truncate(super::streaming::constants::ERROR_MESSAGE_MAX_CHARS);
            s.push('…');
            s
        } else {
            message
        };
        Self {
            kind,
            message,
            code: None,
            raw: None,
        }
    }

    /// Map this error to the finish reason the stream consumer should use.
    pub fn finish_reason(self) -> FinishReason {
        match self.kind {
            ProviderErrorKind::Authentication
            | ProviderErrorKind::InvalidEndpoint
            | ProviderErrorKind::ModelUnavailable
            | ProviderErrorKind::ProviderRejected
            | ProviderErrorKind::Transport
            | ProviderErrorKind::Truncated
            | ProviderErrorKind::Malformed
            | ProviderErrorKind::Unknown => FinishReason::Error,
        }
    }
}
