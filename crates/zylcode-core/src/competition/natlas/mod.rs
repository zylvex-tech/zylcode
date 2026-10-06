//! N-ATLAS integration boundary — **Phase C0: scaffold and block, never fake**.
//!
//! # The one rule
//!
//! > N-ATLAS is never simulated and described as real.
//!
//! No endpoint is invented. No authentication format is invented. No model id
//! is invented. No response is fabricated. No other provider is relabelled as
//! N-ATLAS. A mock transport exists only inside tests and is named as such.
//!
//! If official N-ATLAS documentation, credentials, endpoint information or
//! access are unavailable, this module produces
//! [`NatlasStatus::BlockedNatlasAccess`] and says so. That is the preferred
//! outcome, because it is true.
//!
//! # What is actually here (Phase C0)
//!
//! | Item | State |
//! |---|---|
//! | Configuration from the environment, no invented defaults | `IMPLEMENTED` · `TESTED` |
//! | Request / response / error / status types | `IMPLEMENTED` · `TESTED` |
//! | `NatlasTransport` trait — the provider boundary | `IMPLEMENTED` |
//! | `BlockedNatlasTransport` — the C0 runtime state | `IMPLEMENTED` · `TESTED` |
//! | Client: invoke → parse → evidence | `IMPLEMENTED` · `TESTED` (mock transport) |
//! | Evidence record + secret redaction | `IMPLEMENTED` · `TESTED` |
//! | Structured-intent parse → factory task graph handoff | `IMPLEMENTED` · `TESTED` |
//! | **A real N-ATLAS HTTP call** | **`BLOCKED_NATLAS_ACCESS`** |
//!
//! # Why there is no HTTP transport yet
//!
//! A real transport needs to know N-ATLAS's endpoint path, authentication
//! scheme and response shape. None of those are documented to us. Writing a
//! speculative one would mean inventing the very things the directive forbids,
//! so the boundary is defined and the transport is left [`BlockedNatlasTransport`]
//! until the real contract is known. Phase C1 implements it against the
//! official documentation.
//!
//! # The honest separation that makes this testable anyway
//!
//! The **wire translation** (N-ATLAS bytes ⇄ our types) is the part that is
//! blocked. The **response contract** we parse is *ours* — it is the schema we
//! ask the model to return — so it can be parsed, validated and tested
//! deterministically today without claiming anything about N-ATLAS.

pub mod client;
pub mod config;
pub mod evidence;
pub mod intent;
pub mod transport;
pub mod types;

pub use client::{NatlasClient, NatlasInvocation};
pub use config::{NatlasConfig, NatlasConfigError, RedactedNatlasConfig};
pub use evidence::{redact_secrets, NatlasEvidence, NATLAS_PROVIDER};
pub use intent::{IntentStepKind, NatlasEngineeringIntent, NatlasIntentStep};
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

/// The runtime state of N-ATLAS integration, resolvable without constructing a
/// client. Useful for a CLI banner or a UI badge.
///
/// * [`NatlasStatus::NotConfigured`] — required environment variables are absent.
/// * [`NatlasStatus::BlockedNatlasAccess`] — configured, but no verified
///   transport exists yet.
///
/// It never returns [`NatlasStatus::Succeeded`]: success requires a real call.
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
