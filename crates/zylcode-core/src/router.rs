//! Multi-provider token router — routes LLM prompts across OpenRouter,
//! DeepSeek, Anthropic, and local Ollama with fallback + telemetry.

pub mod cache;
pub mod decision;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

pub use cache::SpeculativeCache;
use crate::cache::{mock_embed, VectorCacheStore};

// ---------------------------------------------------------------------------
// Model provider
// ---------------------------------------------------------------------------

/// Supported upstream model providers (legacy enum, kept for backward compat).
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

/// Provider kind for Phase 7.2 multi-provider routing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Anthropic,
    OpenRouter,
    Ollama,
    SyntheticOffline,
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Anthropic => write!(f, "anthropic"),
            Self::OpenRouter => write!(f, "openrouter"),
            Self::Ollama => write!(f, "ollama"),
            Self::SyntheticOffline => write!(f, "synthetic-offline"),
        }
    }
}

impl ProviderKind {
    /// Default base URL for the provider.
    pub fn default_base_url(&self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com",
            Self::OpenRouter => "https://openrouter.ai/api/v1",
            Self::Ollama => "http://localhost:11434",
            Self::SyntheticOffline => "",
        }
    }

    /// Default completions path (relative to base URL) for the provider.
    pub fn completions_path(&self) -> &'static str {
        match self {
            Self::Anthropic => "/v1/messages",
            Self::OpenRouter => "/chat/completions",
            Self::Ollama => "/api/chat",
            Self::SyntheticOffline => "",
        }
    }
}

/// Per-provider configuration for the multi-provider router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Logical kind (Anthropic, OpenRouter, Ollama, SyntheticOffline).
    pub kind: ProviderKind,
    /// Model identifier for this provider (e.g. `anthropic/claude-3.5-sonnet`).
    pub model: String,
    /// Custom endpoint override (override the default base URL + path).
    #[serde(default)]
    pub endpoint: String,
    /// Request timeout in milliseconds.
    #[serde(default = "default_provider_timeout_ms")]
    pub timeout_ms: u64,
    /// Whether this provider is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Fallback order index (0 = first attempt, higher = later in chain).
    #[serde(default)]
    pub fallback_order: u32,
    /// Whether this provider requires an API key.
    #[serde(default)]
    pub requires_api_key: bool,
}

fn default_provider_timeout_ms() -> u64 {
    60_000
}

fn default_provider_configs() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            kind: ProviderKind::Anthropic,
            model: "anthropic/claude-3.5-sonnet".to_string(),
            endpoint: String::new(),
            timeout_ms: default_provider_timeout_ms(),
            enabled: true,
            fallback_order: 0,
            requires_api_key: true,
        },
        ProviderConfig {
            kind: ProviderKind::Ollama,
            model: "llama3.1".to_string(),
            endpoint: String::new(),
            timeout_ms: default_provider_timeout_ms(),
            enabled: true,
            fallback_order: 1,
            requires_api_key: false,
        },
        ProviderConfig {
            kind: ProviderKind::OpenRouter,
            model: "openrouter/gpt-4o-mini".to_string(),
            endpoint: String::new(),
            timeout_ms: default_provider_timeout_ms(),
            enabled: true,
            fallback_order: 2,
            requires_api_key: true,
        },
        ProviderConfig {
            kind: ProviderKind::SyntheticOffline,
            model: "synthetic-offline".to_string(),
            endpoint: String::new(),
            timeout_ms: default_provider_timeout_ms(),
            enabled: true,
            fallback_order: 3,
            requires_api_key: false,
        },
    ]
}

// ---------------------------------------------------------------------------
// Router configuration
// ---------------------------------------------------------------------------

/// How to trim prompts that exceed the context window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContextTrim {
    TruncateHead,
    #[default]
    SlidingWindow,
}

/// Configuration controlling routing, fallback, and authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Primary provider to attempt first (legacy compat).
    pub primary_provider: ModelProvider,
    /// Fallback provider when the primary is rate-limited or errors (legacy compat).
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
    /// Per-provider configurations for the multi-provider router (Phase 7.2).
    #[serde(default = "default_provider_configs")]
    pub provider_configs: Vec<ProviderConfig>,
    /// Request timeout in milliseconds (applies to all providers).
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Maximum fallback attempts across all providers (0 = no fallback).
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Adaptive context window in tokens (estimated as `len/4`). System prompt
    /// is always preserved; excess prompt head is trimmed.
    #[serde(default = "default_context_window_tokens")]
    pub context_window_tokens: u32,
    /// Strategy used when trimming.
    #[serde(default)]
    pub trim_strategy: ContextTrim,
}

fn default_timeout_ms() -> u64 {
    60_000
}
fn default_max_retries() -> u32 {
    1
}
fn default_context_window_tokens() -> u32 {
    8192
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
            provider_configs: vec![
                ProviderConfig {
                    kind: ProviderKind::Anthropic,
                    model: "anthropic/claude-3.5-sonnet".to_string(),
                    endpoint: String::new(),
                    timeout_ms: default_provider_timeout_ms(),
                    enabled: true,
                    fallback_order: 0,
                    requires_api_key: true,
                },
                ProviderConfig {
                    kind: ProviderKind::Ollama,
                    model: "llama3.1".to_string(),
                    endpoint: String::new(),
                    timeout_ms: default_provider_timeout_ms(),
                    enabled: true,
                    fallback_order: 1,
                    requires_api_key: false,
                },
                ProviderConfig {
                    kind: ProviderKind::OpenRouter,
                    model: "openrouter/gpt-4o-mini".to_string(),
                    endpoint: String::new(),
                    timeout_ms: default_provider_timeout_ms(),
                    enabled: true,
                    fallback_order: 2,
                    requires_api_key: true,
                },
                ProviderConfig {
                    kind: ProviderKind::SyntheticOffline,
                    model: "synthetic-offline".to_string(),
                    endpoint: String::new(),
                    timeout_ms: default_provider_timeout_ms(),
                    enabled: true,
                    fallback_order: 3,
                    requires_api_key: false,
                },
            ],
            timeout_ms: default_timeout_ms(),
            max_retries: default_max_retries(),
            context_window_tokens: default_context_window_tokens(),
            trim_strategy: ContextTrim::default(),
        }
    }
}

/// Trim `(prompt, system)` to fit `context_window_tokens`. System is preserved;
/// prompt head is dropped so tail (most recent intent) remains.
/// Returns owned trimmed strings — slices would borrow transient temporaries.
pub fn trim_to_window(prompt: &str, system: &str, window_tokens: u32, strategy: ContextTrim) -> (String, String) {
    let window = window_tokens.max(256) as usize;
    let est = |s: &str| s.len().div_ceil(4);
    let system_tokens = est(system);
    if system_tokens >= window {
        // System itself exceeds window — truncate system head conservatively.
        let keep_chars = (window - 16) * 4;
        let trimmed_system = system[system.len().saturating_sub(keep_chars)..].to_string();
        return (String::new(), trimmed_system);
    }
    let remaining = window - system_tokens;
    let prompt_tokens = est(prompt);
    if prompt_tokens <= remaining {
        return (prompt.to_string(), system.to_string());
    }
    let keep_prompt_tokens = remaining.saturating_sub(8);
    let keep_chars = keep_prompt_tokens * 4;
    let prompt_trimmed = match strategy {
        ContextTrim::TruncateHead | ContextTrim::SlidingWindow => {
            // Keep tail of prompt; for SlidingWindow we could keep a midpoint,
            // but head-trim is optimal for intent tail relevance.
            prompt[prompt.len().saturating_sub(keep_chars)..].to_string()
        }
    };
    (prompt_trimmed, system.to_string())
}

impl RouterConfig {
    /// Resolve the base URL for a provider (override or default).
    pub fn base_url(&self, provider: &ModelProvider) -> String {
        self.base_url_overrides
            .get(&provider.to_string())
            .cloned()
            .unwrap_or_else(|| provider.default_base_url().to_string())
    }

    /// Resolve the API key for a provider, checking config then env as fallback.
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

    /// Resolve the API key for a ProviderKind, checking config then env as fallback.
    pub fn api_key_for_kind(&self, kind: &ProviderKind) -> Option<String> {
        // Map ProviderKind to the corresponding env variable name.
        let env_key = match kind {
            ProviderKind::Anthropic => "ANTHROPIC_API_KEY",
            ProviderKind::OpenRouter => "OPENROUTER_API_KEY",
            ProviderKind::Ollama => return None, // Ollama has no API key requirement,
            ProviderKind::SyntheticOffline => return None,
        };
        // First check explicit API keys keyed by the kind display name.
        if let Some(k) = self.api_keys.get(&kind.to_string()) {
            if !k.is_empty() {
                return Some(k.clone());
            }
        }
        // Env fallback.
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
// Cost estimation
// ---------------------------------------------------------------------------

/// Per-provider pricing: (input_per_1m, output_per_1m) in USD.
/// Ollama and SyntheticOffline are free (local inference).
fn provider_pricing(provider: &ProviderKind) -> (f64, f64) {
    match provider {
        // Anthropic Claude 3.5 Sonnet (2024-10-22 pricing)
        ProviderKind::Anthropic => (3.0, 15.0),
        // OpenRouter routes through various providers; use a blended mid-range
        // estimate (GPT-4o-mini class) so cost is conservative.
        ProviderKind::OpenRouter => (0.15, 0.60),
        // DeepSeek V3 (deepseek-chat) pricing
        ProviderKind::Ollama => (0.0, 0.0),
        // Local / synthetic — no API cost.
        ProviderKind::SyntheticOffline => (0.0, 0.0),
    }
}

/// Estimate the dollar cost of a request for a given provider and token counts.
/// Returns `0.0` for local/free providers (Ollama, SyntheticOffline).
pub fn estimate_cost(provider: &ProviderKind, input_tokens: u64, output_tokens: u64) -> f64 {
    let (in_rate, out_rate) = provider_pricing(provider);
    let input_cost = (input_tokens as f64 / 1_000_000.0) * in_rate;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * out_rate;
    // Round to 6 decimal places (~micro-dollar precision) to avoid float noise.
    ((input_cost + output_cost) * 1_000_000.0).round() / 1_000_000.0
}

// ---------------------------------------------------------------------------
// Token router
// ---------------------------------------------------------------------------

/// Routes prompts to the configured LLM providers with automatic fallback.
pub struct TokenRouter {
    config: RouterConfig,
    metrics: Arc<TokenMetrics>,
    http: reqwest::Client,
    cache: Arc<SpeculativeCache>,
    vector_cache: Option<Arc<VectorCacheStore>>,
}

impl std::fmt::Debug for TokenRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenRouter")
            .field("config", &self.config)
            .field("metrics", &self.metrics.snapshot())
            .field("cache_len", &self.cache.len())
            .field("vector_cache", &self.vector_cache.is_some())
            .finish()
    }
}

impl Clone for TokenRouter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            metrics: Arc::clone(&self.metrics),
            http: self.http.clone(),
            cache: Arc::clone(&self.cache),
            vector_cache: self.vector_cache.clone(),
        }
    }
}

impl TokenRouter {
    pub fn new(config: RouterConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .context("failed to build HTTP client")?;
        // Best-effort init of local vector cache (offline-capable)
        let vector_cache = VectorCacheStore::with_default_path().ok().map(Arc::new);
        Ok(Self {
            config,
            metrics: Arc::new(TokenMetrics::default()),
            cache: Arc::new(SpeculativeCache::with_defaults()),
            http,
            vector_cache,
        })
    }

    pub fn with_metrics(config: RouterConfig, metrics: Arc<TokenMetrics>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .context("failed to build HTTP client")?;
        let vector_cache = VectorCacheStore::with_default_path().ok().map(Arc::new);
        Ok(Self {
            config,
            metrics,
            http,
            cache: Arc::new(SpeculativeCache::with_defaults()),
            vector_cache,
        })
    }

    pub fn with_cache(config: RouterConfig, cache: Arc<SpeculativeCache>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .context("failed to build HTTP client")?;
        let vector_cache = VectorCacheStore::with_default_path().ok().map(Arc::new);
        Ok(Self {
            config,
            metrics: Arc::new(TokenMetrics::default()),
            cache,
            http,
            vector_cache,
        })
    }

    /// Attach a custom vector cache (useful for tests / per-workspace isolation).
    pub fn with_vector_cache(mut self, store: VectorCacheStore) -> Self {
        self.vector_cache = Some(Arc::new(store));
        self
    }

    pub fn vector_cache(&self) -> Option<Arc<VectorCacheStore>> {
        self.vector_cache.clone()
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

    pub fn cache(&self) -> &SpeculativeCache {
        &self.cache
    }

    // -- public API ---------------------------------------------------------

    /// Dispatch a prompt with a system preamble, applying fallback on
    /// rate-limit (429) or transient 5xx errors.
    /// Adaptive trimming + Phase 8.1 context compression applied first, then
    /// speculative cache is probed before any HTTP egress.
    pub async fn dispatch_prompt(&self, prompt: &str, system: &str) -> Result<String> {
        // Phase 8.1: Context compression against token budget before provider dispatch.
        let budget = self.config.context_window_tokens as usize;
        let (prompt_owned, system_owned) = {
            let est = |s: &str| s.len().div_ceil(4);
            if est(prompt) + est(system) > budget {
                let compressor = crate::compression::ContextCompressor::new(budget);
                let (p, s, _m) = compressor.compress(prompt, system);
                (p, s)
            } else {
                // Budget already satisfied — just trim head as before
                trim_to_window(
                    prompt,
                    system,
                    self.config.context_window_tokens,
                    self.config.trim_strategy,
                )
            }
        };
        let prompt = prompt_owned.as_str();
        let system = system_owned.as_str();

        // Speculative cache probe (hash includes model so fallback model
        // differences don't collide). Record saved tokens on hit.
        let primary_model = self.config.primary_model.clone();
        let cache_key = SpeculativeCache::hash_key(prompt, system, &primary_model);
        if let Some(cached) = self.cache.get(cache_key) {
            let saved = (cached.len() / 4) as u64;
            self.metrics.record_saved(saved);
            info!(cache_hit = true, saved_tokens = saved, provider = %self.config.primary_provider, "speculative cache hit");
            return Ok(cached);
        }

        // Phase 8.2: vector cache similarity retrieval (offline-capable, before remote egress)
        if let Some(vstore) = &self.vector_cache {
            let emb = mock_embed(prompt, 32);
            match vstore.find_similar(&emb, 0.88) {
                Ok(Some(hit)) => {
                    info!(event = "telemetry:cache_hit", prompt_hash = %hit.prompt_hash, threshold = 0.88, "vector cache hit — returning cached response without provider call");
                    self.metrics.record_saved((hit.response_text.len() / 4) as u64);
                    self.cache.insert(cache_key, hit.response_text.clone());
                    return Ok(hit.response_text);
                }
                Ok(None) => {
                    tracing::debug!(event = "telemetry:cache_miss", "vector cache miss — proceeding to provider dispatch");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "vector cache lookup failed — proceeding to provider dispatch");
                }
            }
        }

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
            self.cache.insert(cache_key, synthetic.clone());
            // Phase 8.2: persist synthetic response to vector cache asynchronously (best-effort)
            if let Some(vstore) = &self.vector_cache {
                let emb = mock_embed(prompt, 32);
                if let Err(e) = vstore.insert_entry(prompt, &synthetic, &emb) {
                    tracing::warn!(error = %e, "vector cache insert failed for synthetic response");
                }
            }
            info!(provider = "synthetic-offline", input_tokens = inp, output_tokens = out, "dispatched prompt offline");
            return Ok(synthetic);
        }

        // Attempt primary.
        let primary_err: Option<anyhow::Error> = match self
            .call_provider(&self.config.primary_provider, &self.config.primary_model, prompt, system)
            .await
        {
            Ok(text) => {
                self.cache.insert(cache_key, text.clone());
                if let Some(vstore) = &self.vector_cache {
                    let emb = mock_embed(prompt, 32);
                    if let Err(e) = vstore.insert_entry(prompt, &text, &emb) {
                        tracing::warn!(error = %e, "vector cache insert failed for primary response");
                    }
                }
                return Ok(text);
            }
            Err(e) if is_retryable(&e) => {
                // Log telemetry:fallback event on retryable primary failure.
                self.metrics.record_fallback();
                let provider_name = self.config.primary_provider.to_string();
                info!(event = "telemetry:fallback", provider = %provider_name, error = %e, "primary provider failed, attempting fallback");
                Some(e)
            }
            Err(e) => {
                if e.to_string().contains("429") || e.to_string().to_lowercase().contains("rate") {
                    // Log telemetry:fallback event on rate-limited primary.
                    self.metrics.record_fallback();
                    let provider_name = self.config.primary_provider.to_string();
                    info!(event = "telemetry:fallback", provider = %provider_name, error = %e, "primary provider rate-limited, attempting fallback");
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

        info!(event = "telemetry:provider_failover", from = %self.config.primary_provider, to = %fallback_provider, "falling back to configured fallback provider");

        info!(provider = %fallback_provider, model = %fallback_model, "dispatching to fallback provider");

        match self
            .call_provider(&fallback_provider, &fallback_model, prompt, system)
            .await
        {
            Ok(text) => {
                // Insert under fallback model key as well so subsequent same-model
                // calls hit, but also under primary key for primary-model callers.
                let fallback_key = SpeculativeCache::hash_key(prompt, system, &fallback_model);
                self.cache.insert(fallback_key, text.clone());
                self.cache.insert(cache_key, text.clone());
                if let Some(vstore) = &self.vector_cache {
                    let emb = mock_embed(prompt, 32);
                    if let Err(e) = vstore.insert_entry(prompt, &text, &emb) {
                        tracing::warn!(error = %e, "vector cache insert failed for fallback response");
                    }
                }
                Ok(text)
            }
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
                    self.cache.insert(cache_key, synthetic.clone());
                    if let Some(vstore) = &self.vector_cache {
                        let emb = mock_embed(prompt, 32);
                        if let Err(e) = vstore.insert_entry(prompt, &synthetic, &emb) {
                            tracing::warn!(error = %e, "vector cache insert failed for fallback synthetic");
                        }
                    }
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
        // Apply adaptive trimming again per-provider (idempotent, cheap).
        let (prompt_owned, system_owned) = trim_to_window(
            prompt,
            system,
            self.config.context_window_tokens,
            self.config.trim_strategy,
        );
        let prompt = prompt_owned.as_str();
        let system = system_owned.as_str();

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

    #[test]
    fn trim_to_window_preserves_system() {
        let system = "system prompt that is short";
        let prompt = "a".repeat(40000);
        let (trimmed, sys) = trim_to_window(&prompt, system, 100, ContextTrim::SlidingWindow);
        assert_eq!(sys, system);
        assert!(trimmed.len() < prompt.len());
        // window is clamped to min 256 tokens => 1024 chars before system overhead
        assert!(trimmed.len() <= 1024);
    }

    #[test]
    fn trim_to_window_noop_when_fits() {
        let (p, s) = trim_to_window("hello", "system", 8192, ContextTrim::TruncateHead);
        assert_eq!(p, "hello");
        assert_eq!(s, "system");
    }

    #[tokio::test]
    async fn speculative_cache_hit_returns_cached() {
        let cfg = RouterConfig::default();
        let cache = std::sync::Arc::new(SpeculativeCache::new(8, std::time::Duration::from_secs(60)));
        let router = TokenRouter::with_cache(cfg, cache.clone()).unwrap();
        let p = "unique prompt for cache test";
        let s = "system";
        let first = router.dispatch_prompt(p, s).await.unwrap();
        let second = router.dispatch_prompt(p, s).await.unwrap();
        assert_eq!(first, second);
        assert_eq!(cache.len(), 1);
        // Second hit should have recorded saved tokens
        assert!(router.snapshot().verification_saved_tokens > 0);
    }

    #[test]
    fn provider_pricing_anthropic() {
        let (input, output) = provider_pricing(&ProviderKind::Anthropic);
        assert!((input - 3.0).abs() < 1e-9);
        assert!((output - 15.0).abs() < 1e-9);
    }

    #[test]
    fn provider_pricing_openrouter() {
        let (input, output) = provider_pricing(&ProviderKind::OpenRouter);
        assert!((input - 0.15).abs() < 1e-9);
        assert!((output - 0.60).abs() < 1e-9);
    }

    #[test]
    fn provider_pricing_ollama_is_zero() {
        let (input, output) = provider_pricing(&ProviderKind::Ollama);
        assert!((input).abs() < 1e-9);
        assert!((output).abs() < 1e-9);
    }

    #[test]
    fn provider_pricing_offline_is_zero() {
        let (input, output) = provider_pricing(&ProviderKind::SyntheticOffline);
        assert!((input).abs() < 1e-9);
        assert!((output).abs() < 1e-9);
    }

    #[test]
    fn estimate_cost_anthropic_small() {
        // 1000 input tokens @ $3/M = $0.003, 500 output tokens @ $15/M = $0.0075
        // total = $0.0105
        let cost = estimate_cost(&ProviderKind::Anthropic, 1000, 500);
        assert!((cost - 0.0105).abs() < 1e-9, "cost was {cost}");
    }

    #[test]
    fn estimate_cost_anthropic_large() {
        // 1_000_000 input tokens @ $3/M = $3.00, 1_000_000 output tokens @ $15/M = $15.00
        let cost = estimate_cost(&ProviderKind::Anthropic, 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 1e-9, "cost was {cost}");
    }

    #[test]
    fn estimate_cost_offline_is_zero() {
        let cost = estimate_cost(&ProviderKind::SyntheticOffline, 50_000, 10_000);
        assert!((cost).abs() < 1e-9, "cost was {cost}");
    }

    #[test]
    fn estimate_cost_ollama_is_zero() {
        let cost = estimate_cost(&ProviderKind::Ollama, 50_000, 10_000);
        assert!((cost).abs() < 1e-9, "cost was {cost}");
    }

    #[test]
    fn estimate_cost_rounding_is_micro_dollar() {
        // 1 input token @ Anthropic $3/M = $0.000003
        // 1 output token @ Anthropic $15/M = $0.000015
        // total = $0.000018
        let cost = estimate_cost(&ProviderKind::Anthropic, 1, 1);
        assert!((cost - 0.000018).abs() < 1e-9, "cost was {cost}");
        // Confirm it has 6 decimal places of precision
        let rounded = (cost * 1_000_000.0).round();
        assert_eq!(rounded, 18.0);
    }

    #[test]
    fn estimate_cost_zero_tokens_is_zero() {
        for provider in [ProviderKind::Anthropic, ProviderKind::OpenRouter, ProviderKind::Ollama, ProviderKind::SyntheticOffline] {
            assert!((estimate_cost(&provider, 0, 0)).abs() < 1e-9);
        }
    }
}
