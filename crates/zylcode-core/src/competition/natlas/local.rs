//! Local N-ATLAS transport — the genuine runtime path (Phase C1).
//!
//! # What this is
//!
//! This transport performs a **real network call** to a locally-served N-ATLAS
//! model and returns the model's actual output. It is not a mock, it contains
//! no fallback, and it cannot fabricate a response: if the server is not
//! running, [`send`](NatlasTransport::send) returns
//! [`NatlasError::Transport`]; if the server answers with something unexpected,
//! it returns [`NatlasError::MalformedResponse`].
//!
//! # Why the OpenAI-compatible protocol is not an invented interface
//!
//! The protocol spoken here is the **OpenAI-compatible chat-completions**
//! protocol, served by Ollama (`/v1/chat/completions`), llama.cpp
//! (`llama-server`) and others. It is a public, widely implemented standard —
//! **not** an N-ATLAS-specific detail. We are not guessing N-ATLAS's endpoint,
//! auth scheme or schema; we are talking to a local server over a documented
//! protocol and translating its documented reply into ZylCode's own canonical
//! response shape.
//!
//! The model that answers is N-ATLAS (see
//! `docs/competition/natlas/NATLAS_RUNTIME_FEASIBILITY_2026-10-06.md`), but the
//! *identity recorded in evidence* is whatever the server reports — it is never
//! assumed.

use std::time::Duration;

use serde_json::{json, Value};

use super::config::NatlasConfig;
use super::transport::{NatlasRawResponse, NatlasTransport};
use super::types::{NatlasError, NatlasRequest};

/// The OpenAI-compatible chat-completions path. Documented by Ollama and
/// llama.cpp; **not** an N-ATLAS-specific path.
pub const OPENAI_CHAT_PATH: &str = "/v1/chat/completions";

/// Ollama's model-listing path, used by [`probe`].
pub const OLLAMA_TAGS_PATH: &str = "/api/tags";

/// Timeout for the lightweight health probe. Deliberately short: a probe that
/// hangs is a probe that lies about availability.
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// A transport that calls a locally-served, OpenAI-compatible N-ATLAS runtime.
pub struct LocalNatlasTransport {
    http: reqwest::Client,
}

impl Default for LocalNatlasTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalNatlasTransport {
    /// Build with a default client. No connection pooling across transports is
    /// shared, so a transport owns exactly one client.
    pub fn new() -> Self {
        Self {
            // `no_proxy` is deliberate and load-bearing: a local runtime lives
            // on the loopback interface and must NOT be routed through an
            // ambient HTTP_PROXY. On a machine with a proxy set (this one has
            // one), the default client would send `127.0.0.1` requests to the
            // proxy and fail — or worse, succeed against the wrong service.
            http: reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("a default reqwest client always builds"),
        }
    }

    /// Build with a caller-supplied client (tests inject one with a short
    /// timeout).
    pub fn with_client(http: reqwest::Client) -> Self {
        Self { http }
    }

    /// Build the OpenAI-compatible request body from our request type.
    ///
    /// Context is folded into the user turn with explicit path labels so the
    /// model can see provenance and so nothing about the repository is
    /// presented as if it were the developer's own words.
    pub fn build_body(config: &NatlasConfig, request: &NatlasRequest) -> Value {
        let mut user = String::new();
        if !request.context.is_empty() {
            user.push_str("Repository context (read-only excerpts):\n");
            for chunk in &request.context {
                user.push_str(&format!("\n--- {} ---\n{}\n", chunk.path, chunk.excerpt));
            }
            user.push_str("\nDeveloper request:\n");
        }
        user.push_str(&request.intent);

        json!({
            "model": config.model,
            "messages": [
                { "role": "system", "content": request.system },
                { "role": "user", "content": user },
            ],
            "stream": false,
            "max_tokens": request.max_tokens,
            // Deterministic by default: this is an engineering tool, not a
            // creative one, and reproducibility is worth more than variety.
            "temperature": 0,
        })
    }

    /// Translate a documented OpenAI-compatible reply into ZylCode's canonical
    /// response body.
    ///
    /// This is a **pure function** and is unit-tested against recorded shapes.
    /// Model identity is taken from the reply; if the reply does not state it,
    /// the call fails rather than borrowing the configured name, because
    /// evidence must record what answered, not what we hoped would answer.
    pub fn translate_openai_reply(body: &str) -> Result<String, NatlasError> {
        let value: Value =
            serde_json::from_str(body).map_err(|e| NatlasError::MalformedResponse {
                detail: format!("local runtime reply was not valid JSON: {e}"),
            })?;

        let choices = value
            .get("choices")
            .and_then(|c| c.as_array())
            .ok_or_else(|| NatlasError::MalformedResponse {
                detail: "reply has no `choices` array".to_string(),
            })?;
        let first = choices.first().ok_or_else(|| NatlasError::MalformedResponse {
            detail: "reply has an empty `choices` array".to_string(),
        })?;

        let text = first
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| NatlasError::MalformedResponse {
                detail: "reply has no `choices[0].message.content` string".to_string(),
            })?;
        if text.trim().is_empty() {
            return Err(NatlasError::MalformedResponse {
                detail: "reply content is empty".to_string(),
            });
        }

        // Identity must come from the server. No substitution.
        let model = value
            .get("model")
            .and_then(|m| m.as_str())
            .filter(|m| !m.trim().is_empty())
            .ok_or_else(|| NatlasError::MalformedResponse {
                detail: "local runtime reply did not report a `model` identity".to_string(),
            })?;

        let request_id = value.get("id").and_then(|v| v.as_str());
        let finish_reason = first.get("finish_reason").and_then(|v| v.as_str());
        let usage = value.get("usage");

        let canonical = json!({
            "text": text,
            "model": model,
            "request_id": request_id,
            "finish_reason": finish_reason,
            "usage": {
                "input_tokens": usage.and_then(|u| u.get("prompt_tokens")).and_then(|v| v.as_u64()),
                "output_tokens": usage.and_then(|u| u.get("completion_tokens")).and_then(|v| v.as_u64()),
            },
        });

        serde_json::to_string(&canonical).map_err(|e| NatlasError::MalformedResponse {
            detail: format!("could not re-serialise the translated reply: {e}"),
        })
    }
}

#[async_trait::async_trait]
impl NatlasTransport for LocalNatlasTransport {
    async fn send(
        &self,
        config: &NatlasConfig,
        request: &NatlasRequest,
    ) -> Result<NatlasRawResponse, NatlasError> {
        let url = config.endpoint();
        let body = Self::build_body(config, request);

        let mut builder = self.http.post(&url).json(&body);
        // A local server usually ignores auth. If a key is configured we still
        // send it, because a reverse-proxied deployment may require it.
        if !config.api_key.is_empty() {
            builder = builder.bearer_auth(&config.api_key);
        }

        let response = builder.send().await.map_err(|e| NatlasError::Transport {
            // The URL is safe to include; the key never is.
            message: format!("could not reach the local N-ATLAS runtime at {url}: {e}"),
        })?;

        let status = response.status().as_u16();
        let raw_body = response
            .text()
            .await
            .map_err(|e| NatlasError::Transport {
                message: format!("local runtime reply could not be read: {e}"),
            })?;

        if !(200..300).contains(&status) {
            // Hand the raw body up; the client turns this into an HttpStatus
            // error and records an excerpt. Never coerced into a success.
            return Ok(NatlasRawResponse {
                status,
                body: raw_body,
            });
        }

        let canonical = Self::translate_openai_reply(&raw_body)?;
        Ok(NatlasRawResponse {
            status,
            body: canonical,
        })
    }
}

/// The result of probing a local runtime for availability.
///
/// This is **observed**, not inferred: `reachable` is true only when an HTTP
/// request actually succeeded.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LocalRuntimeHealth {
    pub base_url: String,
    pub reachable: bool,
    /// Model names the server reports, when it reports them.
    pub models: Vec<String>,
    /// Whether the requested model appears in `models`; `None` when unknown.
    pub model_present: Option<bool>,
    /// A human-readable, secret-free explanation.
    pub detail: String,
}

impl LocalRuntimeHealth {
    /// True only when the runtime is reachable **and** the requested model is
    /// present. Never optimistic.
    pub fn is_ready(&self) -> bool {
        self.reachable && self.model_present == Some(true)
    }
}

/// Probe a local runtime without invoking the model.
///
/// Tries Ollama's `/api/tags` first, then the OpenAI-compatible `/v1/models`.
/// Returns a truthful record either way; an unreachable runtime is a normal
/// result, not an error.
pub async fn probe(base_url: &str, model: Option<&str>) -> LocalRuntimeHealth {
    // Loopback must bypass any ambient proxy — see `LocalNatlasTransport::new`.
    let client = match reqwest::Client::builder()
        .timeout(PROBE_TIMEOUT)
        .no_proxy()
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return LocalRuntimeHealth {
                base_url: base_url.to_string(),
                reachable: false,
                models: Vec::new(),
                model_present: None,
                detail: format!("could not build an HTTP client: {e}"),
            }
        }
    };

    let base = base_url.trim_end_matches('/');

    // Ollama first.
    if let Some(models) = fetch_ollama_models(&client, base).await {
        let model_present = model.map(|m| models.iter().any(|x| x == m || x.starts_with(&format!("{m}:"))));
        return LocalRuntimeHealth {
            base_url: base.to_string(),
            reachable: true,
            models,
            model_present,
            detail: "local runtime reachable (Ollama /api/tags)".to_string(),
        };
    }

    // Then the generic OpenAI-compatible listing.
    if let Some(models) = fetch_openai_models(&client, base).await {
        let model_present = model.map(|m| models.iter().any(|x| x == m));
        return LocalRuntimeHealth {
            base_url: base.to_string(),
            reachable: true,
            models,
            model_present,
            detail: "local runtime reachable (OpenAI-compatible /v1/models)".to_string(),
        };
    }

    LocalRuntimeHealth {
        base_url: base.to_string(),
        reachable: false,
        models: Vec::new(),
        model_present: None,
        detail: format!(
            "no local runtime answered at {base} (tried {OLLAMA_TAGS_PATH} and /v1/models)"
        ),
    }
}

async fn fetch_ollama_models(client: &reqwest::Client, base: &str) -> Option<Vec<String>> {
    let url = format!("{base}{OLLAMA_TAGS_PATH}");
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let value: Value = resp.json().await.ok()?;
    let arr = value.get("models")?.as_array()?;
    Some(
        arr.iter()
            .filter_map(|m| {
                m.get("name")
                    .or_else(|| m.get("model"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .collect(),
    )
}

async fn fetch_openai_models(client: &reqwest::Client, base: &str) -> Option<Vec<String>> {
    let url = format!("{base}/v1/models");
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let value: Value = resp.json().await.ok()?;
    let arr = value.get("data")?.as_array()?;
    Some(
        arr.iter()
            .filter_map(|m| m.get("id").and_then(|n| n.as_str()).map(|s| s.to_string()))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> NatlasConfig {
        NatlasConfig::from_parts(
            "http://127.0.0.1:11434",
            OPENAI_CHAT_PATH,
            "n-atlas",
            "local-test-key",
        )
        .unwrap()
    }

    #[test]
    fn build_body_is_openai_compatible_and_places_context_in_the_user_turn() {
        let req = NatlasRequest::new("add a health check", "you are a coding assistant")
            .with_context("src/main.rs", "fn main() {}");
        let body = LocalNatlasTransport::build_body(&config(), &req);

        assert_eq!(body["model"], "n-atlas");
        assert_eq!(body["stream"], false);
        assert_eq!(body["temperature"], 0);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "you are a coding assistant");
        let user = body["messages"][1]["content"].as_str().unwrap();
        assert!(user.contains("src/main.rs"), "context path must be labelled");
        assert!(user.contains("add a health check"), "intent must be present");
    }

    #[test]
    fn translates_a_documented_openai_reply_into_the_canonical_shape() {
        let reply = r#"{
            "id": "chatcmpl-abc",
            "model": "n-atlas:latest",
            "choices": [
                { "index": 0,
                  "message": { "role": "assistant", "content": "{\"summary\":\"s\",\"steps\":[]}" },
                  "finish_reason": "stop" }
            ],
            "usage": { "prompt_tokens": 42, "completion_tokens": 7 }
        }"#;

        let canonical = LocalNatlasTransport::translate_openai_reply(reply).unwrap();
        let v: Value = serde_json::from_str(&canonical).unwrap();
        assert_eq!(v["model"], "n-atlas:latest", "identity must come from the server");
        assert_eq!(v["request_id"], "chatcmpl-abc");
        assert_eq!(v["finish_reason"], "stop");
        assert_eq!(v["usage"]["input_tokens"], 42);
        assert_eq!(v["usage"]["output_tokens"], 7);
        assert!(v["text"].as_str().unwrap().contains("summary"));
    }

    #[test]
    fn translation_refuses_a_reply_that_does_not_state_its_model() {
        // Identity is not borrowable from configuration: it must be reported.
        let reply = r#"{"id":"x","choices":[{"message":{"content":"hi"},"finish_reason":"stop"}]}"#;
        let err = LocalNatlasTransport::translate_openai_reply(reply).unwrap_err();
        assert_eq!(err.code(), "MALFORMED_RESPONSE");
        assert!(err.to_string().contains("model"));
    }

    #[test]
    fn translation_refuses_empty_content_and_missing_choices() {
        assert_eq!(
            LocalNatlasTransport::translate_openai_reply(
                r#"{"model":"m","choices":[{"message":{"content":"  "}}]}"#
            )
            .unwrap_err()
            .code(),
            "MALFORMED_RESPONSE"
        );
        assert_eq!(
            LocalNatlasTransport::translate_openai_reply(r#"{"model":"m","choices":[]}"#)
                .unwrap_err()
                .code(),
            "MALFORMED_RESPONSE"
        );
    }

    #[test]
    fn translation_never_invents_a_response_from_non_json() {
        assert_eq!(
            LocalNatlasTransport::translate_openai_reply("connection reset")
                .unwrap_err()
                .code(),
            "MALFORMED_RESPONSE"
        );
    }

    #[tokio::test]
    async fn an_unreachable_runtime_is_a_transport_error_not_a_synthetic_answer() {
        // Port 1 is not a listening service. The transport must say so and must
        // NOT return any body.
        //
        // This test is also the regression guard for the loopback proxy bypass:
        // this machine has HTTP_PROXY set, so if the client honoured the proxy
        // the request would come back as an HTTP 502 *response* instead of a
        // transport error, and this assertion would fail.
        let cfg = NatlasConfig::from_parts(
            "http://127.0.0.1:1",
            OPENAI_CHAT_PATH,
            "n-atlas",
            "local-test-key",
        )
        .unwrap();
        let t = LocalNatlasTransport::new();
        let err = t
            .send(&cfg, &NatlasRequest::new("hello", "sys"))
            .await
            .unwrap_err();
        assert_eq!(err.code(), "TRANSPORT");
    }

    #[tokio::test]
    async fn probe_reports_an_unreachable_runtime_truthfully() {
        let health = probe("http://127.0.0.1:1", Some("n-atlas")).await;
        assert!(!health.reachable);
        assert!(!health.is_ready());
        assert!(health.models.is_empty());
    }
}
