//! N-ATLAS configuration.
//!
//! Every value comes from the environment. There are **no invented defaults**:
//! the base URL, request path, API key and model are all required, because
//! guessing any of them would be inventing N-ATLAS interface details.
//!
//! Secrets are never logged. [`NatlasConfig::redacted`] produces a
//! serialisable view that carries the API key's *presence and length* only —
//! never the value.

use std::fmt;

/// Required environment variables, in the order a human should set them.
pub const REQUIRED_ENV: &[&str] = &[ENV_BASE_URL, ENV_REQUEST_PATH, ENV_MODEL, ENV_API_KEY];

/// Base URL of the N-ATLAS service, e.g. `https://...`. **Required.**
pub const ENV_BASE_URL: &str = "NATLAS_BASE_URL";
/// Request path, relative to the base URL. **Required** — deliberately not
/// defaulted, because the path is an N-ATLAS interface detail we do not know.
pub const ENV_REQUEST_PATH: &str = "NATLAS_REQUEST_PATH";
/// Model identifier to request. **Required** — never defaulted to a guess.
pub const ENV_MODEL: &str = "NATLAS_MODEL";
/// API key. **Required.** Never logged, never committed.
pub const ENV_API_KEY: &str = "NATLAS_API_KEY";
/// Optional per-request timeout in milliseconds.
pub const ENV_TIMEOUT_MS: &str = "NATLAS_TIMEOUT_MS";

/// Timeout applied when [`ENV_TIMEOUT_MS`] is unset. This is our own bound on
/// how long *we* wait — not a claim about N-ATLAS.
pub const DEFAULT_TIMEOUT_MS: u64 = 60_000;

/// A complete, validated N-ATLAS configuration.
#[derive(Clone, PartialEq, Eq)]
pub struct NatlasConfig {
    pub base_url: String,
    /// **Secret.** Never log, never serialise this struct directly.
    pub api_key: String,
    pub model: String,
    pub request_path: String,
    pub timeout_ms: u64,
}

impl fmt::Debug for NatlasConfig {
    /// Debug output is redacted by construction so an accidental `{:?}` in a
    /// log line cannot leak the key.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NatlasConfig")
            .field("base_url", &self.base_url)
            .field("request_path", &self.request_path)
            .field("model", &self.model)
            .field("timeout_ms", &self.timeout_ms)
            .field("api_key", &format!("<redacted; {} chars>", self.api_key.len()))
            .finish()
    }
}

/// Which required variables were absent or empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatlasConfigError {
    pub missing: Vec<&'static str>,
}

impl fmt::Display for NatlasConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "N-ATLAS is not configured; missing environment variable(s): {}",
            self.missing.join(", ")
        )
    }
}

impl std::error::Error for NatlasConfigError {}

/// A log-safe, serialisable view of a configuration. Contains no secret value.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RedactedNatlasConfig {
    pub base_url: String,
    pub request_path: String,
    pub model: String,
    pub timeout_ms: u64,
    pub api_key_present: bool,
    /// Length only — enough to tell "set" from "set to the placeholder".
    pub api_key_len: usize,
}

impl NatlasConfig {
    /// Read the configuration from the process environment.
    ///
    /// Returns [`NatlasConfigError`] listing **every** missing variable at once
    /// rather than failing on the first, so an operator can fix them in one
    /// pass.
    pub fn from_env() -> Result<Self, NatlasConfigError> {
        let base_url = read_env(ENV_BASE_URL);
        let request_path = read_env(ENV_REQUEST_PATH);
        let model = read_env(ENV_MODEL);
        let api_key = read_env(ENV_API_KEY);

        let mut missing = Vec::new();
        if base_url.is_none() {
            missing.push(ENV_BASE_URL);
        }
        if request_path.is_none() {
            missing.push(ENV_REQUEST_PATH);
        }
        if model.is_none() {
            missing.push(ENV_MODEL);
        }
        if api_key.is_none() {
            missing.push(ENV_API_KEY);
        }
        if !missing.is_empty() {
            return Err(NatlasConfigError { missing });
        }

        let timeout_ms = read_env(ENV_TIMEOUT_MS)
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(DEFAULT_TIMEOUT_MS);

        Ok(Self {
            base_url: base_url.expect("checked above"),
            request_path: request_path.expect("checked above"),
            model: model.expect("checked above"),
            api_key: api_key.expect("checked above"),
            timeout_ms,
        })
    }

    /// Build a configuration from explicit parts. Intended for tests and for
    /// callers that read configuration from somewhere other than the
    /// environment. Applies the same "all four required" rule.
    pub fn from_parts(
        base_url: impl Into<String>,
        request_path: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<Self, NatlasConfigError> {
        let mut missing = Vec::new();
        let base_url = trim_or_flag(base_url.into(), ENV_BASE_URL, &mut missing);
        let request_path = trim_or_flag(request_path.into(), ENV_REQUEST_PATH, &mut missing);
        let model = trim_or_flag(model.into(), ENV_MODEL, &mut missing);
        let api_key = trim_or_flag(api_key.into(), ENV_API_KEY, &mut missing);
        if !missing.is_empty() {
            return Err(NatlasConfigError { missing });
        }
        Ok(Self {
            base_url: base_url.expect("checked above"),
            request_path: request_path.expect("checked above"),
            model: model.expect("checked above"),
            api_key: api_key.expect("checked above"),
            timeout_ms: DEFAULT_TIMEOUT_MS,
        })
    }

    /// The absolute endpoint, assembled from the two configured parts. No
    /// component is defaulted.
    pub fn endpoint(&self) -> String {
        format!(
            "{}{}",
            self.base_url.trim_end_matches('/'),
            if self.request_path.starts_with('/') {
                self.request_path.clone()
            } else {
                format!("/{}", self.request_path)
            }
        )
    }

    /// A view safe to log or serialise.
    pub fn redacted(&self) -> RedactedNatlasConfig {
        RedactedNatlasConfig {
            base_url: self.base_url.clone(),
            request_path: self.request_path.clone(),
            model: self.model.clone(),
            timeout_ms: self.timeout_ms,
            api_key_present: !self.api_key.is_empty(),
            api_key_len: self.api_key.len(),
        }
    }
}

fn read_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn trim_or_flag(
    value: String,
    key: &'static str,
    missing: &mut Vec<&'static str>,
) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        missing.push(key);
        None
    } else {
        Some(trimmed)
    }
}
