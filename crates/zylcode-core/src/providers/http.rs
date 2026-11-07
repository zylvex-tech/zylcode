//! Shared HTTP utilities used by provider modules.
//!
//! This module does **not** parse provider responses. It only helps provider modules
//! build requests and common headers on top of the router's existing
//! [`reqwest::Client`]. Provider-specific request shapes, auth headers, and response
//! parsing stay in the provider submodules.

use std::time::Duration;

/// Build a POST request builder preloaded with a JSON body and a timeout.
pub fn post_json(
    client: &reqwest::Client,
    url: &str,
    body: serde_json::Value,
    timeout: Duration,
) -> reqwest::RequestBuilder {
    client
        .post(url)
        .timeout(timeout)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&body)
}

/// Convenience helper used by provider modules that reuse the router-wide default
/// timeout when a provider-specific one is not supplied.
pub fn router_default_timeout(config_timeout_ms: u64) -> Duration {
    Duration::from_millis(config_timeout_ms.max(1))
}

/// Base address helpers used by provider modules.
pub mod base {
    /// Strip a trailing slash from a URL the caller controls.
    pub fn strip_trailing_slash(raw: &str) -> String {
        let trimmed = raw.trim();
        trimmed.strip_suffix('/').unwrap_or(trimmed).to_string()
    }

    /// Append a path to a base URL, avoiding double slashes.
    pub fn join(base: &str, path: &str) -> String {
        let base = strip_trailing_slash(base);
        let path = path.strip_prefix('/').unwrap_or(path);
        format!("{base}/{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::base;

    #[test]
    fn join_avoids_double_slashes() {
        assert_eq!(
            base::join("https://example.com/v1", "/chat/completions"),
            "https://example.com/v1/chat/completions"
        );
        assert_eq!(
            base::join("https://example.com/v1/", "chat/completions"),
            "https://example.com/v1/chat/completions"
        );
        assert_eq!(
            base::join("https://example.com/v1/", "/chat/completions"),
            "https://example.com/v1/chat/completions"
        );
    }

    #[test]
    fn strip_trailing_slash_is_idempotent() {
        let a = "https://example.com/";
        let b = base::strip_trailing_slash(a);
        assert_eq!(b, "https://example.com");
        assert_eq!(base::strip_trailing_slash(&b), b);
    }
}
