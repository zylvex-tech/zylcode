//! N-ATLAS integration boundary — **never fake**.
//!
//! # The one rule
//!
//! > N-ATLAS is never simulated and described as real.
//!
//! No endpoint is invented. No authentication format is invented. No model id
//! is invented. No response is fabricated. No other provider is relabelled as
//! N-ATLAS. A test double exists only inside tests and is named as such.
//!
//! # What is actually here
//!
//! | Item | State |
//! |---|---|
//! | Configuration from the environment, no invented defaults | `IMPLEMENTED` · `TESTED` |
//! | Request / response / error / status types | `IMPLEMENTED` · `TESTED` |
//! | `NatlasTransport` trait — the provider boundary | `IMPLEMENTED` |
//! | `BlockedNatlasTransport` — the truthful blocked state | `IMPLEMENTED` · `TESTED` |
//! | **`LocalNatlasTransport`** — real call to a local N-ATLAS runtime | `IMPLEMENTED` · `TESTED` |
//! | **Runtime health probe** (`local::probe`) | `IMPLEMENTED` · `TESTED` |
//! | `HttpNatlasTransport` — remote adapter, contract **unverified** | `IMPLEMENTED` · `CONTRACT_UNVERIFIED` |
//! | Client: invoke → parse → evidence | `IMPLEMENTED` · `TESTED` |
//! | Evidence record + secret redaction | `IMPLEMENTED` · `TESTED` |
//! | Structured-intent parse → factory task graph handoff | `IMPLEMENTED` · `TESTED` |
//! | **A captured genuine N-ATLAS invocation** | **`BLOCKED_NATLAS_ACCESS`** |
//!
//! # The two blocked reasons, kept distinct
//!
//! * [`BLOCKED_NATLAS_API_ACCESS`] — no documented remote API contract exists.
//! * [`BLOCKED_NATLAS_ACCESS`] — the runtime could not be reached or its access
//!   was not granted (e.g. the official model weights are gated).
//!
//! Neither is a synonym for "not done". Both are statements about the world.
//!
//! # Why a mock cannot be mistaken for evidence
//!
//! The `NatlasTransport` trait is public so that tests can implement a double.
//! Any such double lives **only in test code**, is named `Mock*`, and its
//! success is never competition evidence. The shipped transports perform real
//! network calls and have **no fallback path**: an unreachable runtime yields
//! [`NatlasError::Transport`], never a synthesised answer.

pub mod client;
pub mod config;
pub mod evidence;
pub mod intent;
pub mod local;
pub mod remote;
pub mod transport;
pub mod types;

pub use client::{NatlasClient, NatlasInvocation};
pub use config::{NatlasConfig, NatlasConfigError, RedactedNatlasConfig};
pub use evidence::{redact_secrets, NatlasEvidence, NATLAS_PROVIDER};
pub use intent::{IntentStepKind, NatlasEngineeringIntent, NatlasIntentStep};
pub use local::{probe, LocalNatlasTransport, LocalRuntimeHealth};
pub use remote::HttpNatlasTransport;
pub use transport::{BlockedNatlasTransport, NatlasRawResponse, NatlasTransport};
pub use types::{
    NatlasContextChunk, NatlasError, NatlasRequest, NatlasResponse, NatlasStatus, NatlasUsage,
};

/// The greppable reason string used wherever N-ATLAS access is unavailable.
///
/// A constant, not a formatted string, so a report can search for it and a test
/// can assert on it — the same discipline the factory runner applies to
/// `BLOCKED_PROVIDER`.
pub const BLOCKED_NATLAS_ACCESS: &str = "BLOCKED_NATLAS_ACCESS";

/// The greppable reason string used when no documented remote N-ATLAS API
/// contract exists (Route A). Kept distinct from [`BLOCKED_NATLAS_ACCESS`] so a
/// report can say *which* access is missing.
pub const BLOCKED_NATLAS_API_ACCESS: &str = "BLOCKED_NATLAS_API_ACCESS";

/// The runtime state of N-ATLAS integration, resolvable **without any I/O**.
///
/// * [`NatlasStatus::NotConfigured`] — required environment variables are absent.
/// * [`NatlasStatus::BlockedNatlasAccess`] — configured, but no verified call
///   has been made.
///
/// It never returns [`NatlasStatus::Succeeded`]: success requires a real call.
/// For a live availability check use [`local::probe`], which performs real I/O
/// and reports what it actually observed.
pub fn runtime_status() -> NatlasStatus {
    match NatlasConfig::from_env() {
        Err(_) => NatlasStatus::NotConfigured,
        Ok(_) => NatlasStatus::BlockedNatlasAccess,
    }
}

/// The environment variables a complete N-ATLAS configuration requires, for
/// display in an operator-facing message. Never contains a value.
pub fn required_env_vars() -> &'static [&'static str] {
    config::REQUIRED_ENV
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_status_never_reports_success_without_a_call() {
        // Whatever the environment, this synchronous status must not claim a
        // verified success — that is the whole point of the honest boundary.
        assert!(!runtime_status().is_verified_success());
    }

    #[test]
    fn the_two_blocked_reasons_are_distinct_strings() {
        assert_ne!(BLOCKED_NATLAS_ACCESS, BLOCKED_NATLAS_API_ACCESS);
    }
}
