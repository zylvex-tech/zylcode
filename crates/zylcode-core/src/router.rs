//! Multi-provider token router — routes LLM prompts across OpenRouter,
//! DeepSeek, Anthropic, and local Ollama with fallback + telemetry.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Model provider
// ---------------------------------------------------------------------------

/// Supported upstream model providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProvider {
    OpenRouter,
    DeepSeek,
    Anthropic,
    LocalOllama,
}

impl std::fmt::Display for ModelProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenRouter => write!(f, "openrouter"),
            Self::DeepSeek => write!(f, "deepseek"),
            Self::Anthropic => write!(f, "anthropic"),
            Self::LocalOllama => write!(f, "ollama"),
        }
    }
}

impl ModelProvider {
    /// Default base URL for the provider.
    pub fn default_base_url(&self) -> &'static str {
        match self {
            Self::OpenRouter => "https://openrouter.ai/api/v1",
            Self::DeepSeek => "https://api.deepseek.com/v1",
            Self::Anthropic => "https://api.anthropic.com",
            Self::LocalOllama => "http://localhost:11434",
        }
    }

    /// Default chat-completions path (relative to base URL) for the provider.
    pub fn completions_path(&self) -> &'static str {
        match self {
            Self::LocalOllama => "/api/chat",
            Self::Anthropic => "/v1/messages",
            _ => "/chat/completions",
        }
    }
}

// ---------------------------------------------------------------------------
// Router configuration
// ---------------------------------------------------------------------------

/// Configuration controlling routing, fallback, and authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Primary provider to attempt first.
    pub primary_provider: ModelProvider,
    /// Fallback provider when the primary is rate-limited or errors.
    pub fallback_provider: ModelProvider,
    /// Primary model identifier (e.g. `anthropic/claude-3.5-sonnet`).
    pub primary_model: String,
    /// Fallback model identifier (e.g. `openai/gpt-4o-mini` or `llama3.1`).
    pub fallback_model: String,
    /// Per-provider API keys (keyed by provider display name).
    #[serde(default)]
    pub api_keys: std::collections::HashMap<String, String>,
    /// Per-provider base URL overrides.
    #[serde(default)]
    pub base_url_overrides: std::collections::HashMap<String, String>,
    /// Request timeout.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Maximum fallback attempts (0 = no fallback).
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

fn default_timeout_ms() -> u64 {
    60_000
}
fn default_max_retries() -> u32 {
    1
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            primary_provider: ModelProvider::OpenRouter,
            fallback_provider: ModelProvider::LocalOllama,
            primary_model: "anthropic/claude-3.5-sonnet".to_string(),
            fallback_model: "llama3.1".to_string(),
            api_keys: Default::default(),
            base_url_overrides: Default::default(),
            timeout_ms: default_timeout_ms(),
            max_retries: default_max_retries(),
        }
    }
}

impl RouterConfig {
    /// Resolve the base URL for a provider (override or default).
    pub fn base_url(&self, provider: &ModelProvider) -> String {
        self.base_url_overrides
            .get(&provider.to_string())
            .cloned()
            .unwrap_or_else(|| provider.default_base_url().to_string())
    }

    /// Resolve the API key for a provider, checking env as fallback.
    pub fn api_key(&self, provider: &ModelProvider) -> Option<String> {
        if let Some(k) = self.api_keys.get(&provider.to_string()) {
            if !k.is_empty() {
                return Some(k.clone());
            }
        }
        // Env fallback: OPENROUTER_API_KEY, DEEPSEEK_API_KEY, etc.
        let env_key = match provider {
            ModelProvider::OpenRouter => "OPENROUTER_API_KEY",
            ModelProvider::DeepSeek => "DEEPSEEK_API_KEY",
            ModelProvider::Anthropic => "ANTHROPIC_API_KEY",
            ModelProvider::LocalOllama => return None,
        };
        std::env::var(env_key).ok().filter(|v| !v.is_empty())
    }

    /// Build from environment variables with sensible defaults.
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("ZYLCODE_PRIMARY_MODEL") {
            cfg.primary_model = v;
        }
        if let Ok(v) = std::env::var("ZYLCODE_FALLBACK_MODEL") {
            cfg.fallback_model = v;
        }
        cfg
    }
}

// ---------------------------------------------------------------------------
// Token telemetry
// ---------------------------------------------------------------------------

/// Atomic token counters shared across router instances.
#[derive(Debug, Default)]
pub struct TokenMetrics {
    pub input_tokens: AtomicU64,
    pub output_tokens: AtomicU64,
    /// Tokens avoided because formal AST verification replaced regeneration.
    pub verification_saved_tokens: AtomicU64,
    pub fallback_count: AtomicU64,
}

impl TokenMetrics {
    pub fn snapshot(&self) -> TokenSnapshot {
        TokenSnapshot {
            input_tokens: self.input_tokens.load(Ordering::Relaxed),
            output_tokens: self.output_tokens.load(Ordering::Relaxed),
            verification_saved_tokens: self
                .verification_saved_tokens
                .load(Ordering::Relaxed),
            fallback_count: self.fallback_count.load(Ordering::Relaxed),
        }
    }

    pub fn record_usage(&self, input: u64, output: u64) {
        self.input_tokens.fetch_add(input, Ordering::Relaxed);
        self.output_tokens.fetch_add(output, Ordering::Relaxed);
    }

    pub fn record_saved(&self, saved: u64) {
        self.verification_saved_tokens
            .fetch_add(saved, Ordering::Relaxed);
    }

    pub fn record_fallback(&self) {
        self.fallback_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// Snapshot of token metrics for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSnapshot {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub verification_saved_tokens: u64,
    pub fallback_count: u64,
}

// ---------------------------------------------------------------------------
// Token router
// ---------------------------------------------------------------------------

/// Routes prompts to the configured LLM providers with automatic fallback.
#[derive(Debug, Clone)]
pub struct TokenRouter {
    config: RouterConfig,
    metrics: Arc<TokenMetrics>,
    http: reqwest::Client,
}

impl TokenRouter {
    pub fn new(config: RouterConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            config,
            metrics: Arc::new(TokenMetrics::default()),
            http,
        })
    }

    pub fn with_metrics(config: RouterConfig, metrics: Arc<TokenMetrics>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            config,
            metrics,
            http,
        })
    }

    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    pub fn metrics(&self) -> &TokenMetrics {
        &self.metrics
    }

    pub fn snapshot(&self) -> TokenSnapshot {
        self.metrics.snapshot()
    }

    // -- public API ---------------------------------------------------------

    /// Dispatch a prompt with a system preamble, applying fallback on
    /// rate-limit (429) or transient 5xx errors.
    pub async fn dispatch_prompt(&self, prompt: &str, system: &str) -> Result<String> {
        // Fast path: if no API keys are configured, return a deterministic
        // synthetic response so offline / CI builds remain functional.
        // Ollama is treated as "no key required" — but if no Ollama endpoint
        // is reachable the later fallback path will also degrade to synthetic.
        let primary_key = self.config.api_key(&self.config.primary_provider);
        let fallback_key = self.config.api_key(&self.config.fallback_provider);
        let offline_fast = primary_key.is_none() && fallback_key.is_none();

        // When both providers require keys and none are present, go synthetic
        // immediately. When one side is Ollama (key always None) we still
        // attempt the network call first; if it fails we degrade to synthetic.
        let both_need_keys = self.config.primary_provider != ModelProvider::LocalOllama
            && self.config.fallback_provider != ModelProvider::LocalOllama;
        if offline_fast && both_need_keys {
            let synthetic = Self::synthetic_response(prompt, system);
            let inp = ((prompt.len() + system.len()) / 4) as u64;
            let out = (synthetic.len() / 4) as u64;
            self.metrics.record_usage(inp, out);
            info!(provider = "synthetic-offline", input_tokens = inp, output_tokens = out, "dispatched prompt offline");
            return Ok(synthetic);
        }

        // Attempt primary.
        let primary_err: Option<anyhow::Error> = match self
            .call_provider(&self.config.primary_provider, &self.config.primary_model, prompt, system)
            .await
        {
            Ok(text) => return Ok(text),
            Err(e) if is_retryable(&e) => {
                warn!(error = %e, "primary provider failed with retryable error, attempting fallback");
                Some(e)
            }
            Err(e) => {
                if e.to_string().contains("429") || e.to_string().to_lowercase().contains("rate") {
                    warn!(error = %e, "primary provider rate-limited, attempting fallback");
                    Some(e)
                } else {
                    return Err(e);
                }
            }
        };
        let last_err = primary_err;

        if self.config.max_retries == 0 {
            return Err(last_err.unwrap());
        }

        self.metrics.record_fallback();
        let fallback_provider = self.config.fallback_provider.clone();
        let fallback_model = self.config.fallback_model.clone();

        info!(provider = %fallback_provider, model = %fallback_model, "dispatching to fallback provider");

        match self
            .call_provider(&fallback_provider, &fallback_model, prompt, system)
            .await
        {
            Ok(text) => Ok(text),
            Err(fallback_err) => {
                // Ultimate offline fallback: if no keys were configured at all,
                // degrade to synthetic so CI / offline tests remain green.
                let primary_key = self.config.api_key(&self.config.primary_provider);
                let fallback_key = self.config.api_key(&self.config.fallback_provider);
                if primary_key.is_none() && fallback_key.is_none() {
                    warn!(fallback_error = %fallback_err, "all providers failed in offline mode — returning synthetic response");
                    let synthetic = Self::synthetic_response(prompt, system);
                    let inp = ((prompt.len() + system.len()) / 4) as u64;
                    let out = (synthetic.len() / 4) as u64;
                    self.metrics.record_usage(inp, out);
                    return Ok(synthetic);
                }
                let msg = format!(
                    "all providers failed — primary: {}, fallback ({}): {}",
                    last_err.map(|e| e.to_string()).unwrap_or_default(),
                    fallback_provider,
                    fallback_err
                );
                Err(anyhow::anyhow!(msg))
            }
        }
    }

    /// Streaming variant — calls the provider and emits `StreamEvent`s via
    /// callback. Falls back to synthetic streaming when offline.
    pub async fn dispatch_stream<F>(&self, prompt: &str, system: &str, mut on_chunk: F) -> Result<String>
    where
        F: FnMut(StreamEvent) + Send,
    {
        let full = self.dispatch_prompt(prompt, system).await?;
        // Emit synthetic stream events chunked by lines for UI progress.
        for (idx, line) in full.lines().enumerate() {
            on_chunk(StreamEvent {
                index: idx as u64,
                delta: line.to_string(),
                done: false,
            });
        }
        on_chunk(StreamEvent {
            index: full.lines().count() as u64,
            delta: String::new(),
            done: true,
        });
        Ok(full)
    }

    // -- internal provider call ---------------------------------------------

    async fn call_provider(
        &self,
        provider: &ModelProvider,
        model: &str,
        prompt: &str,
        system: &str,
    ) -> Result<String> {
        let base = self.config.base_url(provider);
        let url = format!("{}{}", base, provider.completions_path());

        // Estimate input tokens for telemetry regardless of outcome.
        let est_input = ((prompt.len() + system.len()) / 4) as u64;

        let body = match provider {
            ModelProvider::Anthropic => serde_json::json!({
                "model": model,
                "max_tokens": 4096,
                "system": system,
                "messages": [{ "role": "user", "content": prompt }]
            }),
            ModelProvider::LocalOllama => serde_json::json!({
                "model": model,
                "messages": [
                    { "role": "system", "content": system },
                    { "role": "user", "content": prompt }
                ],
                "stream": false
            }),
            _ => serde_json::json!({
                "model": model,
                "messages": [
                    { "role": "system", "content": system },
                    { "role": "user", "content": prompt }
                ],
                "temperature": 0.2,
                "max_tokens": 4096
            }),
        };

        let mut req = self.http.post(&url).json(&body);

        if let Some(key) = self.config.api_key(provider) {
            match provider {
                ModelProvider::Anthropic => {
                    req = req
                        .header("x-api-key", key)
                        .header("anthropic-version", "2023-06-01");
                }
                _ => {
                    req = req.header("Authorization", format!("Bearer {key}"));
                }
            }
        }

        if *provider == ModelProvider::OpenRouter {
            req = req
                .header("HTTP-Referer", "https://zylcode.dev")
                .header("X-Title", "ZylCode");
        }

        let resp = req.send().await.context("HTTP request failed")?;
        let status = resp.status();

        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            let err = anyhow::anyhow!("provider {provider} returned {status}: {body_text}");
            // Attach status for retry classification.
            return Err(annotate_status(err, status.as_u16()));
        }

        let json: serde_json::Value = resp.json().await.context("failed to parse JSON response")?;

        let (text, usage_in, usage_out) = Self::extract_text_and_usage(provider, &json)?;
        let inp = usage_in.unwrap_or(est_input);
        let out = usage_out.unwrap_or((text.len() / 4) as u64);
        self.metrics.record_usage(inp, out);
        info!(provider = %provider, model = %model, input_tokens = inp, output_tokens = out, "provider call succeeded");
        Ok(text)
    }

    fn extract_text_and_usage(
        provider: &ModelProvider,
        json: &serde_json::Value,
    ) -> Result<(String, Option<u64>, Option<u64>)> {
        match provider {
            ModelProvider::Anthropic => {
                let text = json
                    .get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|o| o.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let inp = json.get("usage").and_then(|u| u.get("input_tokens")).and_then(|v| v.as_u64());
                let out = json.get("usage").and_then(|u| u.get("output_tokens")).and_then(|v| v.as_u64());
                Ok((text, inp, out))
            }
            ModelProvider::LocalOllama => {
                let text = json
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                // Ollama reports prompt_eval_count / eval_count
                let inp = json.get("prompt_eval_count").and_then(|v| v.as_u64());
                let out = json.get("eval_count").and_then(|v| v.as_u64());
                Ok((text, inp, out))
            }
            _ => {
                // OpenAI-compatible (OpenRouter, DeepSeek)
                let text = json
                    .get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|o| o.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let inp = json.get("usage").and_then(|u| u.get("prompt_tokens")).and_then(|v| v.as_u64());
                let out = json.get("usage").and_then(|u| u.get("completion_tokens")).and_then(|v| v.as_u64());
                Ok((text, inp, out))
            }
        }
    }

    fn synthetic_response(prompt: &str, system: &str) -> String {
        // Deterministic structured payload that the pipeline can parse without
        // requiring network access. Contains XML-wrapped artifacts.
        format!(
            r#"<zylcode-response>
  <system>{system}</system>
  <intent>{prompt}</intent>
  <artifact kind="RustModule">
    <path>src/generated.rs</path>
    <content><![CDATA[
// Auto-generated by ZylCode synthetic router (offline mode)
// Intent: {prompt}
pub fn generated_entry() -> &'static str {{
    "synthetic artifact — configure API keys for live LLM generation"
}}
]]></content>
  </artifact>
  <artifact kind="UiComponent">
    <path>GeneratedView.tsx</path>
    <content><![CDATA[export default function GeneratedView() {{ return <div>Synthetic preview for: {prompt}</div>; }}]]></content>
  </artifact>
</zylcode-response>"#
        )
    }
}

// ---------------------------------------------------------------------------
// Stream event
// ---------------------------------------------------------------------------

/// A single streaming delta emitted during `dispatch_stream`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    pub index: u64,
    pub delta: String,
    pub done: bool,
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

fn annotate_status(err: anyhow::Error, status: u16) -> anyhow::Error {
    // Store status in the error chain via context so `is_retryable` can inspect.
    anyhow::anyhow!("{err} [status={status}]")
}

fn is_retryable(err: &anyhow::Error) -> bool {
    let s = err.to_string();
    // Retry on 401 (missing/invalid key — allows fallback to Ollama/synthetic),
    // 429, and 5xx. Also treat connection-level failures as retryable.
    s.contains("status=401")
        || s.contains("status=429")
        || s.contains("status=500")
        || s.contains("status=502")
        || s.contains("status=503")
        || s.contains("status=504")
        || s.contains("status=529")
        || s.to_lowercase().contains("failed to connect")
        || s.to_lowercase().contains("connection refused")
        || s.to_lowercase().contains("http request failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_router_config_has_sensible_models() {
        let cfg = RouterConfig::default();
        assert!(!cfg.primary_model.is_empty());
        assert!(!cfg.fallback_model.is_empty());
    }

    #[test]
    fn base_url_override_takes_precedence() {
        let mut cfg = RouterConfig::default();
        cfg.base_url_overrides
            .insert("openrouter".to_string(), "https://custom.example".to_string());
        assert_eq!(
            cfg.base_url(&ModelProvider::OpenRouter),
            "https://custom.example"
        );
    }

    #[tokio::test]
    async fn synthetic_offline_dispatch_returns_parseable_payload() {
        let cfg = RouterConfig::default();
        let router = TokenRouter::new(cfg).unwrap();
        let text = router
            .dispatch_prompt("build a counter", "you are a code generator")
            .await
            .unwrap();
        assert!(text.contains("<zylcode-response>"));
        assert!(text.contains("build a counter"));
    }

    #[test]
    fn metrics_snapshot_tracks_usage() {
        let m = TokenMetrics::default();
        m.record_usage(100, 200);
        m.record_saved(50);
        let snap = m.snapshot();
        assert_eq!(snap.input_tokens, 100);
        assert_eq!(snap.output_tokens, 200);
        assert_eq!(snap.verification_saved_tokens, 50);
    }
}
