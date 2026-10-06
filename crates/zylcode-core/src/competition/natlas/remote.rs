//! Remote N-ATLAS HTTP transport — a **configurable adapter with an UNVERIFIED
//! contract**.
//!
//! # Read this before using it
//!
//! Phase C1 found **no documented public N-ATLAS inference API**
//! (`BLOCKED_NATLAS_API_ACCESS`; see
//! `docs/competition/natlas/NATLAS_RUNTIME_FEASIBILITY_2026-10-06.md` §2).
//! Consequently this transport does **not** claim to speak N-ATLAS's protocol.
//!
//! What it does is deliberately narrow and non-inventive:
//!
//! * it POSTs **ZylCode's own canonical request** as JSON to the configured
//!   `NATLAS_BASE_URL` + `NATLAS_REQUEST_PATH`;
//! * it attaches the API key using an authentication scheme the operator must
//!   state explicitly (`NATLAS_AUTH_HEADER`, `NATLAS_AUTH_PREFIX`). **There is
//!   no default auth scheme**, because choosing one would be inventing an
//!   N-ATLAS interface detail;
//! * it expects **ZylCode's own canonical response** back.
//!
//! Until official documentation arrives, this transport is
//! `IMPLEMENTED · CONTRACT_UNVERIFIED` and must never be presented as proof of
//! N-ATLAS integration. If `NATLAS_AUTH_HEADER` is not set, the transport
//! refuses with [`NatlasError::BlockedNatlasAccess`] rather than guessing.

use super::config::NatlasConfig;
use super::transport::{NatlasRawResponse, NatlasTransport};
use super::types::{NatlasError, NatlasRequest};

/// Environment variable naming the auth header, e.g. `Authorization`.
/// **Required** for this transport; deliberately has no default.
pub const ENV_AUTH_HEADER: &str = "NATLAS_AUTH_HEADER";
/// Optional prefix placed before the key, e.g. `Bearer `. Defaults to empty.
pub const ENV_AUTH_PREFIX: &str = "NATLAS_AUTH_PREFIX";

/// A generic HTTP adapter for a remote N-ATLAS endpoint.
///
/// Contract **UNVERIFIED** — see the module documentation.
pub struct HttpNatlasTransport {
    http: reqwest::Client,
    auth_header: String,
    auth_prefix: String,
}

impl HttpNatlasTransport {
    /// Build with an explicit, operator-stated auth scheme.
    ///
    /// `auth_header` is the HTTP header that carries the key (e.g.
    /// `Authorization`); `auth_prefix` is prepended to the key (e.g. `Bearer `,
    /// or empty).
    pub fn new(auth_header: impl Into<String>, auth_prefix: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .build()
                .expect("a default reqwest client always builds"),
            auth_header: auth_header.into(),
            auth_prefix: auth_prefix.into(),
        }
    }

    /// Build from `NATLAS_AUTH_HEADER` / `NATLAS_AUTH_PREFIX`.
    ///
    /// Returns `None` when `NATLAS_AUTH_HEADER` is unset — in which case the
    /// caller should use a blocked transport instead of inventing a scheme.
    pub fn from_env() -> Option<Self> {
        let header = std::env::var(ENV_AUTH_HEADER).ok()?;
        let header = header.trim();
        if header.is_empty() {
            return None;
        }
        let prefix = std::env::var(ENV_AUTH_PREFIX).unwrap_or_default();
        Some(Self::new(header.to_string(), prefix))
    }

    /// The auth header name in use. Never includes the key.
    pub fn auth_header(&self) -> &str {
        &self.auth_header
    }
}

#[async_trait::async_trait]
impl NatlasTransport for HttpNatlasTransport {
    async fn send(
        &self,
        config: &NatlasConfig,
        request: &NatlasRequest,
    ) -> Result<NatlasRawResponse, NatlasError> {
        if self.auth_header.trim().is_empty() {
            return Err(NatlasError::BlockedNatlasAccess {
                reason: format!(
                    "{ENV_AUTH_HEADER} is not set: the remote N-ATLAS authentication \
                     scheme is unknown and will not be guessed"
                ),
            });
        }

        let url = config.endpoint();
        let body = serde_json::json!({
            "model": config.model,
            "system": request.system,
            "intent": request.intent,
            "context": request.context,
            "max_tokens": request.max_tokens,
        });

        let mut builder = self.http.post(&url).json(&body);
        let value = format!("{}{}", self.auth_prefix, config.api_key);
        builder = builder.header(self.auth_header.as_str(), value);

        let response = builder.send().await.map_err(|e| NatlasError::Transport {
            message: format!("could not reach the remote N-ATLAS endpoint at {url}: {e}"),
        })?;

        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|e| NatlasError::Transport {
                message: format!("remote N-ATLAS reply could not be read: {e}"),
            })?;

        // No translation, no fallback: the canonical parser above decides
        // whether this is a valid response.
        Ok(NatlasRawResponse {
            status,
            body: text,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> NatlasConfig {
        NatlasConfig::from_parts("http://127.0.0.1:1", "/v1/infer", "m", "k").unwrap()
    }

    #[tokio::test]
    async fn without_an_explicit_auth_scheme_the_transport_refuses_rather_than_guessing() {
        let t = HttpNatlasTransport::new("", "");
        let err = t
            .send(&config(), &NatlasRequest::new("hi", "sys"))
            .await
            .unwrap_err();
        assert!(err.is_blocked(), "must refuse, got {err:?}");
        assert_eq!(err.code(), "BLOCKED_NATLAS_ACCESS");
    }

    #[tokio::test]
    async fn with_a_scheme_it_attempts_a_real_call_and_never_fabricates_a_success() {
        // Port 1 is not a listening service. Depending on the environment the
        // failure surfaces either as a transport error or as a non-2xx raw
        // response (this machine has an HTTP proxy that answers 502 for the
        // loopback). What must NEVER happen is a fabricated 2xx success.
        let t = HttpNatlasTransport::new("Authorization", "Bearer ");
        match t.send(&config(), &NatlasRequest::new("hi", "sys")).await {
            Err(e) => assert_eq!(e.code(), "TRANSPORT"),
            Ok(raw) => assert!(
                !(200..300).contains(&raw.status),
                "a dead endpoint must not produce a 2xx, got {}",
                raw.status
            ),
        }
    }
}
