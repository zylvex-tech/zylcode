//! Multi-provider token router — routes LLM prompts across OpenRouter,
//! DeepSeek, Anthropic, the Respan hosted gateway, and local Ollama with
//! fallback + telemetry.

pub mod cache;
pub mod decision;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use crate::cache::{mock_embed, prompt_hash, VectorCacheStore};
use crate::providers::streaming::StreamChunk;
pub use cache::SpeculativeCache;

// Test-only spy: counts every attempt to send an HTTP request to a model
// provider. Used to prove that synthetic/offline dispatch performs zero
// network egress — an assertion that cannot be satisfied by "it happened to
// fail and fell back".
//
// Thread-local, not process-global. `cargo test` runs each test on its own
// thread and `#[tokio::test]` defaults to a current-thread runtime, so a
// thread-local counter is scoped to exactly one test.
//
// The previous process-global `AtomicU64` was observably wrong: it could be
// incremented by a *sibling* test dispatching concurrently. `DISPATCH_MUTEX`
// is private to this module, so a dispatching test elsewhere in the same
// crate (`pipeline.rs`, `agent.rs`, `cli.rs`) cannot acquire it, yet still
// shared the counter. That is the mechanism behind the observed
// `left: 5, right: 3`: one concurrent `RouterConfig::default()` dispatch —
// whose fallback is `LocalOllama`, so the no-keys fast path is bypassed —
// attempts the primary provider and then its Ollama fallback, which is
// exactly two increments. Per-thread state removes the entire class of
// interference instead of serialising against tests this module cannot see.
//
// (`//` rather than `///`: `thread_local!` is a macro invocation, and rustdoc
// rejects doc comments on macro invocations — `-D unused-doc-comments` is
// implied by `-D warnings`.)
#[cfg(test)]
thread_local! {
    pub(crate) static NETWORK_EGRESS_COUNT: std::cell::Cell<u64> =
        const { std::cell::Cell::new(0) };
}

// ---------------------------------------------------------------------------
// Respan hosted gateway
// ---------------------------------------------------------------------------

/// Base URL of the Respan hosted gateway — one OpenAI-compatible surface for
/// every upstream provider. Respan traces each request server-side, so a
/// request that arrives here is logged as a trace without any client-side
/// tracing SDK.
///
/// `RouterConfig::from_env` selects [`ModelProvider::Respan`] when
/// `RESPAN_API_KEY` is set; [`RouterConfig::base_url`] honours
/// `RESPAN_BASE_URL` (Respan Enterprise exposes a different endpoint).
pub const RESPAN_GATEWAY_BASE_URL: &str = "https://api.respan.ai/api";

/// Model used when the gateway is selected but neither `RESPAN_MODEL` nor
/// `ZYLCODE_PRIMARY_MODEL` names one. Any model the gateway serves works
/// here — it is a plain string sent in the request body.
pub const RESPAN_DEFAULT_MODEL: &str = "gpt-5.5";

/// Read an environment variable, trimmed, treating blank as unset.
///
/// Every credential and endpoint the router reads from the environment goes
/// through this, so `KEY=` in a shell profile cannot select a provider that
/// would then fail on an empty bearer token.
fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

// ---------------------------------------------------------------------------
// Model provider
// ---------------------------------------------------------------------------

/// Supported upstream model providers (legacy enum, kept for backward compat).
///
/// `SyntheticOffline` is a **first-class provider**, not a degradation path.
/// Selecting it is an explicit statement that this router must not egress.
/// Its behaviour is fixed at construction time: it reads no API key (config or
/// environment) and issues no HTTP request, so ambient credentials such as
/// `OPENROUTER_API_KEY` cannot change what it returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProvider {
    OpenRouter,
    DeepSeek,
    /// OpenAI's own API (`/v1/chat/completions`, `OPENAI_API_KEY`).
    OpenAi,
    /// Any OpenAI-compatible endpoint that is not one of the named providers
    /// (a self-hosted gateway, vLLM, LM Studio, a corporate proxy). It has no
    /// default base URL: the configured endpoint is the only source of one, so
    /// a request cannot silently go to a default host the user never chose.
    OpenAiCompatible,
    Anthropic,
    LocalOllama,
    /// The Respan hosted gateway: OpenAI-compatible chat completions at
    /// [`RESPAN_GATEWAY_BASE_URL`], authenticated with `RESPAN_API_KEY`.
    /// Provider routing stays inside Respan; every call is traced there.
    Respan,
    SyntheticOffline,
}

impl ModelProvider {
    /// Parse the name this provider reports itself under.
    ///
    /// The inverse of `Display`, plus `custom` as the user-facing spelling of
    /// an OpenAI-compatible endpoint. Unknown names return `None` so a caller
    /// can keep its default instead of silently picking a provider.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "openrouter" => Some(Self::OpenRouter),
            "deepseek" => Some(Self::DeepSeek),
            "openai" => Some(Self::OpenAi),
            "openai-compatible" | "openai_compatible" | "custom" => Some(Self::OpenAiCompatible),
            "anthropic" => Some(Self::Anthropic),
            "ollama" | "local-ollama" => Some(Self::LocalOllama),
            "respan" => Some(Self::Respan),
            "synthetic-offline" | "synthetic" => Some(Self::SyntheticOffline),
            _ => None,
        }
    }
}

impl std::fmt::Display for ModelProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenRouter => write!(f, "openrouter"),
            Self::DeepSeek => write!(f, "deepseek"),
            Self::OpenAi => write!(f, "openai"),
            Self::OpenAiCompatible => write!(f, "openai-compatible"),
            Self::Anthropic => write!(f, "anthropic"),
            Self::LocalOllama => write!(f, "ollama"),
            Self::Respan => write!(f, "respan"),
            Self::SyntheticOffline => write!(f, "synthetic-offline"),
        }
    }
}

impl ModelProvider {
    /// Default base URL for the provider.
    pub fn default_base_url(&self) -> &'static str {
        match self {
            Self::OpenRouter => "https://openrouter.ai/api/v1",
            Self::DeepSeek => "https://api.deepseek.com/v1",
            Self::OpenAi => "https://api.openai.com/v1",
            // Deliberately empty: see the variant documentation.
            Self::OpenAiCompatible => "",
            Self::Anthropic => "https://api.anthropic.com",
            Self::LocalOllama => "http://localhost:11434",
            // No trailing slash: `completions_path` supplies the leading one.
            Self::Respan => RESPAN_GATEWAY_BASE_URL,
            // No endpoint — synthetic dispatch never builds a URL.
            Self::SyntheticOffline => "",
        }
    }

    /// Default chat-completions path (relative to base URL) for the provider.
    pub fn completions_path(&self) -> &'static str {
        match self {
            Self::LocalOllama => "/api/chat",
            Self::Anthropic => "/v1/messages",
            // No endpoint — synthetic dispatch never builds a URL.
            Self::SyntheticOffline => "",
            _ => "/chat/completions",
        }
    }
}

/// Provider kind for multi-provider routing (track: MULTIPROVIDER-1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Anthropic,
    OpenRouter,
    OpenAi,
    DeepSeek,
    Ollama,
    /// A user-supplied OpenAI-compatible endpoint. Requires an endpoint and a
    /// model, because neither has a defensible default.
    Custom,
    SyntheticOffline,
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Anthropic => write!(f, "anthropic"),
            Self::OpenRouter => write!(f, "openrouter"),
            Self::OpenAi => write!(f, "openai"),
            Self::DeepSeek => write!(f, "deepseek"),
            Self::Ollama => write!(f, "ollama"),
            Self::Custom => write!(f, "custom"),
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
            Self::OpenAi => "https://api.openai.com/v1",
            Self::DeepSeek => "https://api.deepseek.com/v1",
            Self::Ollama => "http://localhost:11434",
            // No default: the configured endpoint is the only source.
            Self::Custom => "",
            Self::SyntheticOffline => "",
        }
    }

    /// Default completions path (relative to base URL) for the provider.
    pub fn completions_path(&self) -> &'static str {
        match self {
            Self::Anthropic => "/v1/messages",
            Self::Ollama => "/api/chat",
            Self::SyntheticOffline => "",
            _ => "/chat/completions",
        }
    }

    /// Every kind, in the order the settings surface lists them.
    pub fn all() -> [ProviderKind; 7] {
        [
            Self::Anthropic,
            Self::OpenAi,
            Self::OpenRouter,
            Self::DeepSeek,
            Self::Ollama,
            Self::Custom,
            Self::SyntheticOffline,
        ]
    }

    /// The dispatch provider name this kind maps to.
    ///
    /// This string is the key used in [`RouterConfig::api_keys`] and, one level
    /// up, the account name in the OS credential store. Returning `None` means
    /// the kind has no dispatcher, so a key for it could never be used.
    pub fn dispatch_name(&self) -> Option<String> {
        kind_to_model_provider(self).map(|p| p.to_string())
    }

    /// Environment variable that can supply this kind's credential.
    ///
    /// One definition, used by key resolution, by the probe's source report and
    /// by the settings surface's "where does this key come from?" line.
    pub fn env_var(&self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("ANTHROPIC_API_KEY"),
            Self::OpenRouter => Some("OPENROUTER_API_KEY"),
            Self::OpenAi => Some("OPENAI_API_KEY"),
            Self::DeepSeek => Some("DEEPSEEK_API_KEY"),
            Self::Custom => Some("OPENAI_COMPATIBLE_API_KEY"),
            Self::Ollama | Self::SyntheticOffline => None,
        }
    }

    /// Whether this kind cannot work without a credential.
    pub fn requires_api_key(&self) -> bool {
        !matches!(self, Self::Ollama | Self::SyntheticOffline)
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

/// Map a chain-entry `ProviderKind` onto the dispatcher's `ModelProvider`.
///
/// The provider chain (`provider_configs`) is keyed by `ProviderKind` while
/// `call_provider` dispatches on `ModelProvider`; they are the same four
/// concepts under two names because the config surface is user-facing kinds
/// and the dispatch path is provider enums. `SyntheticOffline` maps
/// identity-wise and is honoured unconditionally by `call_provider`, which
/// is what makes it a sound last chain entry.
/// One chain entry resolved into the terms the dispatcher speaks.
///
/// `provider_configs` is keyed by the user-facing [`ProviderKind`]; dispatch is
/// keyed by [`ModelProvider`]. This is the single place the two are reconciled.
fn kind_to_model_provider(kind: &ProviderKind) -> Option<ModelProvider> {
    match kind {
        ProviderKind::Anthropic => Some(ModelProvider::Anthropic),
        ProviderKind::OpenRouter => Some(ModelProvider::OpenRouter),
        ProviderKind::OpenAi => Some(ModelProvider::OpenAi),
        ProviderKind::DeepSeek => Some(ModelProvider::DeepSeek),
        ProviderKind::Ollama => Some(ModelProvider::LocalOllama),
        ProviderKind::Custom => Some(ModelProvider::OpenAiCompatible),
        ProviderKind::SyntheticOffline => Some(ModelProvider::SyntheticOffline),
    }
}

/// Normalise a configured endpoint into a base URL.
///
/// Accepts either a base (`https://host/v1`) or a full completions URL
/// (`https://host/v1/chat/completions`); the known path for the provider is
/// stripped so `call_provider` cannot append it twice. Anything else is used
/// verbatim, because a gateway with a non-standard path is the whole reason the
/// field exists.
fn endpoint_base_url(endpoint: &str, provider: &ModelProvider) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    match trimmed.strip_suffix(provider.completions_path()) {
        Some(base) if !base.is_empty() => base.trim_end_matches('/').to_string(),
        _ => trimmed.to_string(),
    }
}

fn default_provider_configs() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            kind: ProviderKind::Anthropic,
            // Native Anthropic model id. The OpenRouter-style slug that used to
            // sit here (`anthropic/claude-3.5-sonnet`) is not an id the native
            // API accepts, so the default configuration could not connect even
            // with a valid key.
            model: "claude-sonnet-4-5".to_string(),
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
            // OpenRouter addresses models as `vendor/model`; the previous value
            // repeated the provider name (`openrouter/gpt-4o-mini`), which
            // OpenRouter does not serve.
            model: "openai/gpt-4o-mini".to_string(),
            endpoint: String::new(),
            timeout_ms: default_provider_timeout_ms(),
            enabled: true,
            fallback_order: 2,
            requires_api_key: true,
        },
        // Available but not attempted until the operator enables them: adding
        // a provider to the list must not change what an existing install
        // tries first.
        ProviderConfig {
            kind: ProviderKind::OpenAi,
            model: "gpt-4o-mini".to_string(),
            endpoint: String::new(),
            timeout_ms: default_provider_timeout_ms(),
            enabled: false,
            fallback_order: 3,
            requires_api_key: true,
        },
        ProviderConfig {
            kind: ProviderKind::DeepSeek,
            model: "deepseek-chat".to_string(),
            endpoint: String::new(),
            timeout_ms: default_provider_timeout_ms(),
            enabled: false,
            fallback_order: 4,
            requires_api_key: true,
        },
        ProviderConfig {
            kind: ProviderKind::Custom,
            // Both must be supplied by the operator; there is no defensible
            // default host or model for "some OpenAI-compatible endpoint".
            model: String::new(),
            endpoint: String::new(),
            timeout_ms: default_provider_timeout_ms(),
            enabled: false,
            fallback_order: 5,
            requires_api_key: true,
        },
        ProviderConfig {
            kind: ProviderKind::SyntheticOffline,
            model: "synthetic-offline".to_string(),
            endpoint: String::new(),
            timeout_ms: default_provider_timeout_ms(),
            enabled: true,
            fallback_order: 6,
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
    /// Per-provider configurations for the multi-provider router (track: MULTIPROVIDER-1).
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
            // One list, one source of truth: the default chain is defined by
            // `default_provider_configs` so a default router and the settings
            // surface cannot drift apart.
            provider_configs: default_provider_configs(),
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
pub fn trim_to_window(
    prompt: &str,
    system: &str,
    window_tokens: u32,
    strategy: ContextTrim,
) -> (String, String) {
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
    /// Read-only view of the measured provider scorecard, for surfaces that
    /// never construct a router (HTTP handlers, Tauri commands). Best-ranked
    /// first; empty when the store is missing or unreadable.
    pub fn scorecard_view() -> Result<Vec<crate::provider_scorecard::ProviderRecord>> {
        let path = TokenRouter::scorecard_path();
        crate::provider_scorecard::ProviderScorecard::open(&path).map(|sc| sc.ranking())
    }

    /// Re-derive the fields dispatch reads from the ordered provider chain, so
    /// `provider_configs` is the single source of truth for what a request does.
    ///
    /// `dispatch_prompt` attempts `primary_*`, then `fallback_*`, then walks the
    /// remaining enabled entries in `provider_configs`. Those three steps have
    /// to agree with the list the settings surface shows, or reordering an entry
    /// changes a screen and nothing else.
    ///
    /// - entries are sorted by `fallback_order` (stable, so equal orders keep the
    ///   order the user arranged them in);
    /// - the first two **dispatchable** enabled entries become the primary and
    ///   the legacy fallback; an entry whose kind has no dispatcher, or whose
    ///   model is blank, is skipped rather than becoming an unnamed primary;
    /// - a non-empty `endpoint` becomes that provider's base-URL override, and
    ///   clearing it removes the override so the provider's default applies;
    /// - with no dispatchable entry the legacy pair is left alone: an empty
    ///   chain is not a reason to silently drop the provider the process started
    ///   with, and `from_env` builds such a configuration.
    pub fn apply_provider_chain(&mut self) {
        self.provider_configs.sort_by_key(|c| c.fallback_order);

        for entry in &self.provider_configs {
            let Some(provider) = kind_to_model_provider(&entry.kind) else {
                continue;
            };
            let key = provider.to_string();
            if entry.endpoint.trim().is_empty() {
                self.base_url_overrides.remove(&key);
            } else {
                self.base_url_overrides
                    .insert(key, endpoint_base_url(&entry.endpoint, &provider));
            }
        }

        let mut dispatchable = self
            .provider_configs
            .iter()
            .filter(|c| c.enabled)
            .filter_map(|c| {
                let provider = kind_to_model_provider(&c.kind)?;
                let model = c.model.trim().to_string();
                // The synthetic provider has no model to name; every real one
                // must name a model or the request cannot be built.
                (provider == ModelProvider::SyntheticOffline || !model.is_empty())
                    .then_some((provider, model))
            });

        let Some((primary_provider, primary_model)) = dispatchable.next() else {
            return;
        };
        self.primary_provider = primary_provider;
        self.primary_model = primary_model;
        if let Some((fallback_provider, fallback_model)) = dispatchable.next() {
            self.fallback_provider = fallback_provider;
            self.fallback_model = fallback_model;
        }
    }

    /// Effective timeout for one provider: its own entry when it names one,
    /// otherwise the router-wide timeout.
    pub fn provider_timeout_ms(&self, provider: &ModelProvider) -> u64 {
        self.provider_configs
            .iter()
            .find(|c| kind_to_model_provider(&c.kind).as_ref() == Some(provider))
            .map(|c| c.timeout_ms)
            .filter(|ms| *ms > 0)
            .unwrap_or(self.timeout_ms)
    }

    /// Resolve the base URL for a provider (override or default).
    ///
    /// The Respan gateway also honours `RESPAN_BASE_URL`, which is how Respan
    /// Enterprise points at its own endpoint. An explicit per-provider override
    /// in the configuration still wins, so a caller can pin a URL for tests.
    pub fn base_url(&self, provider: &ModelProvider) -> String {
        if let Some(url) = self.base_url_overrides.get(&provider.to_string()) {
            return url.clone();
        }
        if *provider == ModelProvider::Respan {
            if let Some(url) = env_nonempty("RESPAN_BASE_URL") {
                let url = url.trim_end_matches('/');
                if !url.is_empty() {
                    return url.to_string();
                }
            }
        }
        provider.default_base_url().to_string()
    }

    /// Resolve the API key for a provider, checking config then env as fallback.
    ///
    /// `ModelProvider::SyntheticOffline` is credential-free **by construction**:
    /// it is short-circuited before the config map is consulted and before any
    /// environment variable is read, so neither a configured key nor an ambient
    /// `OPENROUTER_API_KEY` can influence it.
    pub fn api_key(&self, provider: &ModelProvider) -> Option<String> {
        if *provider == ModelProvider::SyntheticOffline {
            return None;
        }
        if let Some(k) = self.api_keys.get(&provider.to_string()) {
            if !k.is_empty() {
                return Some(k.clone());
            }
        }
        // Env fallback: OPENROUTER_API_KEY, DEEPSEEK_API_KEY, etc.
        let env_key = match provider {
            ModelProvider::OpenRouter => "OPENROUTER_API_KEY",
            ModelProvider::DeepSeek => "DEEPSEEK_API_KEY",
            ModelProvider::OpenAi => "OPENAI_API_KEY",
            ModelProvider::OpenAiCompatible => "OPENAI_COMPATIBLE_API_KEY",
            ModelProvider::Anthropic => "ANTHROPIC_API_KEY",
            ModelProvider::LocalOllama => return None,
            ModelProvider::Respan => "RESPAN_API_KEY",
            ModelProvider::SyntheticOffline => return None,
        };
        std::env::var(env_key).ok().filter(|v| !v.is_empty())
    }

    /// Resolve the API key for a ProviderKind, checking config then env as fallback.
    pub fn api_key_for_kind(&self, kind: &ProviderKind) -> Option<String> {
        // Map ProviderKind to the corresponding env variable name.
        let env_key = match kind {
            ProviderKind::Anthropic => "ANTHROPIC_API_KEY",
            ProviderKind::OpenRouter => "OPENROUTER_API_KEY",
            ProviderKind::OpenAi => "OPENAI_API_KEY",
            ProviderKind::DeepSeek => "DEEPSEEK_API_KEY",
            ProviderKind::Ollama => return None, // Ollama has no API key requirement,
            ProviderKind::Custom => "OPENAI_COMPATIBLE_API_KEY",
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
    ///
    /// Setting `RESPAN_API_KEY` routes every model call through the Respan
    /// hosted gateway (see [`RouterConfig::connect_respan_gateway`]). Absent the
    /// key, routing is exactly what it was before — including the fully offline
    /// synthetic path when no provider credential is configured.
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        let primary_model = env_nonempty("ZYLCODE_PRIMARY_MODEL");
        let fallback_model = env_nonempty("ZYLCODE_FALLBACK_MODEL");
        if let Some(v) = primary_model.clone() {
            cfg.primary_model = v;
        }
        if let Some(v) = fallback_model {
            cfg.fallback_model = v;
        }
        // A headless run has no settings UI, so the provider and its endpoint
        // have to be nameable from the environment. Both are opt-in: an
        // unconfigured checkout keeps the built-in default.
        if let Some(name) = env_nonempty("ZYLCODE_PRIMARY_PROVIDER") {
            match ModelProvider::from_name(&name) {
                Some(provider) => cfg.primary_provider = provider,
                None => warn!(
                    provider = %name,
                    "ZYLCODE_PRIMARY_PROVIDER names no known provider; keeping the default"
                ),
            }
        }
        if let Some(url) = env_nonempty("ZYLCODE_BASE_URL") {
            cfg.base_url_overrides.insert(
                cfg.primary_provider.to_string(),
                url.trim_end_matches('/').to_string(),
            );
        }
        cfg.connect_respan_gateway();
        cfg
    }

    /// Make the Respan hosted gateway this router's primary provider when
    /// `RESPAN_API_KEY` is present, and report whether it did.
    ///
    /// The gateway is OpenAI-compatible, so dispatch keeps its existing shape:
    /// the same request body, the same `choices[0].message.content` extraction,
    /// the same fallback chain underneath. Only the endpoint and the credential
    /// change. Respan traces the call on its side, which is why no tracing
    /// client is involved here.
    ///
    /// Model precedence: `RESPAN_MODEL` → `ZYLCODE_PRIMARY_MODEL` →
    /// [`RESPAN_DEFAULT_MODEL`]. The gateway serves models by its own names, so
    /// the provider-native default model is not reused when nothing names one.
    ///
    /// Returns `false` and changes nothing when the key is absent or blank, so
    /// an unconfigured checkout behaves exactly as before.
    pub fn connect_respan_gateway(&mut self) -> bool {
        let Some(key) = env_nonempty("RESPAN_API_KEY") else {
            return false;
        };
        self.api_keys.insert(ModelProvider::Respan.to_string(), key);
        self.primary_provider = ModelProvider::Respan;
        self.primary_model = env_nonempty("RESPAN_MODEL")
            .or_else(|| env_nonempty("ZYLCODE_PRIMARY_MODEL"))
            .unwrap_or_else(|| RESPAN_DEFAULT_MODEL.to_string());
        true
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
            verification_saved_tokens: self.verification_saved_tokens.load(Ordering::Relaxed),
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
        // GPT-4o-mini list pricing.
        ProviderKind::OpenAi => (0.15, 0.60),
        // DeepSeek V3 (deepseek-chat), cache-miss pricing.
        ProviderKind::DeepSeek => (0.27, 1.10),
        // A custom endpoint's price is unknown to us; the same blended
        // mid-range estimate is used so the figure is never reported as free.
        ProviderKind::Custom => (0.15, 0.60),
        // Local inference — no API cost.
        ProviderKind::Ollama => (0.0, 0.0),
        // Synthetic — no API cost.
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
    /// Measurement-based routing memory: per-provider outcomes that reorder
    /// the configured fallback chain once enough samples exist.
    scorecard: Arc<crate::provider_scorecard::ProviderScorecard>,
    /// Live router settings, shared by every clone of this router (the agent
    /// dispatches through a clone taken at construction, so a copied field
    /// would leave the settings panel editing state nothing reads).
    ///
    /// Each request takes one snapshot on entry via [`TokenRouter::config`], so
    /// a change saved mid-request cannot swap provider or model half-way
    /// through that request.
    config: Arc<std::sync::RwLock<RouterConfig>>,
    metrics: Arc<TokenMetrics>,
    http: reqwest::Client,
    cache: Arc<SpeculativeCache>,
    vector_cache: Option<Arc<VectorCacheStore>>,
}

impl std::fmt::Debug for TokenRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenRouter")
            .field("config", &self.config())
            .field("metrics", &self.metrics.snapshot())
            .field("cache_len", &self.cache.len())
            .field("vector_cache", &self.vector_cache.is_some())
            .finish()
    }
}

impl Clone for TokenRouter {
    fn clone(&self) -> Self {
        Self {
            scorecard: Arc::clone(&self.scorecard),
            config: Arc::clone(&self.config),
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
            scorecard: Arc::new(
                crate::provider_scorecard::ProviderScorecard::open(&Self::scorecard_path())
                    .unwrap_or_else(|_| {
                        // A broken scorecard must never block dispatch: fall back
                        // to an isolated in-memory-style location.
                        let fallback = std::env::temp_dir().join("zylcode-scorecard-fallback.db");
                        crate::provider_scorecard::ProviderScorecard::open(&fallback)
                            .expect("temp-dir scorecard must open")
                    }),
            ),
            config: std::sync::Arc::new(std::sync::RwLock::new(config)),
            metrics: Arc::new(TokenMetrics::default()),
            cache: Arc::new(SpeculativeCache::with_defaults()),
            http,
            vector_cache,
        })
    }

    /// Scorecard location: `ZYLCODE_SCORECARD_PATH` → `~/.zylcode/provider_scorecard.db`
    /// (same resolution convention as the vector cache).
    fn scorecard_path() -> std::path::PathBuf {
        std::env::var("ZYLCODE_SCORECARD_PATH")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| std::path::Path::new(&h).join(".zylcode/provider_scorecard.db"))
            })
            .unwrap_or_else(|| std::path::PathBuf::from("provider_scorecard.db"))
    }

    pub fn with_metrics(config: RouterConfig, metrics: Arc<TokenMetrics>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .context("failed to build HTTP client")?;
        let vector_cache = VectorCacheStore::with_default_path().ok().map(Arc::new);
        Ok(Self {
            scorecard: Arc::new(
                crate::provider_scorecard::ProviderScorecard::open(&Self::scorecard_path())
                    .unwrap_or_else(|_| {
                        let fallback = std::env::temp_dir().join("zylcode-scorecard-fallback.db");
                        crate::provider_scorecard::ProviderScorecard::open(&fallback)
                            .expect("temp-dir scorecard must open")
                    }),
            ),
            config: std::sync::Arc::new(std::sync::RwLock::new(config)),
            metrics,
            http,
            cache: Arc::new(SpeculativeCache::with_defaults()),
            vector_cache,
        })
    }

    /// Construct a router with **no persistent vector cache**.
    ///
    /// `new()` and `with_metrics()` both resolve the cache path via
    /// `VectorCacheStore::with_default_path()`, which falls back to `./vector_cache.db`
    /// relative to the process working directory. That makes any dispatch result depend
    /// on ambient filesystem state: a stale cache on disk will satisfy a prompt from a
    /// previous run, so a test can pass without ever exercising the code path it claims
    /// to test (observed directly — a deliberately corrupted `synthetic_response` was
    /// still "returned" from cache).
    ///
    /// Use this constructor in tests and in any caller that requires deterministic
    /// behaviour. It performs no filesystem access.
    pub fn without_vector_cache(config: RouterConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self {
            scorecard: Arc::new(
                crate::provider_scorecard::ProviderScorecard::open(&Self::scorecard_path())
                    .unwrap_or_else(|_| {
                        let fallback = std::env::temp_dir().join("zylcode-scorecard-fallback.db");
                        crate::provider_scorecard::ProviderScorecard::open(&fallback)
                            .expect("temp-dir scorecard must open")
                    }),
            ),
            config: std::sync::Arc::new(std::sync::RwLock::new(config)),
            metrics: Arc::new(TokenMetrics::default()),
            cache: Arc::new(SpeculativeCache::with_defaults()),
            http,
            vector_cache: None,
        })
    }

    pub fn with_cache(config: RouterConfig, cache: Arc<SpeculativeCache>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .context("failed to build HTTP client")?;
        let vector_cache = VectorCacheStore::with_default_path().ok().map(Arc::new);
        Ok(Self {
            scorecard: Arc::new(
                crate::provider_scorecard::ProviderScorecard::open(&Self::scorecard_path())
                    .unwrap_or_else(|_| {
                        let fallback = std::env::temp_dir().join("zylcode-scorecard-fallback.db");
                        crate::provider_scorecard::ProviderScorecard::open(&fallback)
                            .expect("temp-dir scorecard must open")
                    }),
            ),
            config: std::sync::Arc::new(std::sync::RwLock::new(config)),
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

    /// A snapshot of the live configuration.
    ///
    /// Owned rather than borrowed: the settings surface can replace the whole
    /// configuration at any moment, so a reference would either have to hold
    /// the read lock across an await (blocking every writer) or outlive the
    /// guard that protects it.
    pub fn config(&self) -> RouterConfig {
        self.config
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Install a configuration that the next request will actually use.
    ///
    /// This is the write-through path for the settings surface. The provider
    /// chain is normalised first ([`RouterConfig::apply_provider_chain`]) so no
    /// caller can install a chain whose on-screen order disagrees with the
    /// order dispatch attempts.
    pub fn set_config(&self, mut config: RouterConfig) {
        config.apply_provider_chain();
        *self
            .config
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = config;
    }

    /// Probe one provider exactly as it is configured right now.
    ///
    /// The settings surface uses this for "test the saved configuration": the
    /// chain entry supplies the model, endpoint and timeout, and the credential
    /// is whatever dispatch would resolve at request time — including one that
    /// only exists in process memory because the OS store supplied it.
    pub async fn probe_saved(&self, kind: &ProviderKind) -> crate::router::ProviderProbe {
        let cfg = self.config();
        let timeout = cfg
            .provider_configs
            .iter()
            .find(|c| c.kind == *kind)
            .map(|c| c.timeout_ms)
            .filter(|ms| *ms > 0)
            .unwrap_or(cfg.timeout_ms);
        crate::router::probe_provider_with_config(&cfg, kind, None, None, timeout).await
    }

    /// Replace the in-memory credential map without touching the chain.
    ///
    /// Startup uses this to load credentials from the OS store into a
    /// configuration that came from the environment. Going through
    /// `update_config` here would re-derive the provider chain and override the
    /// provider the environment selected — adding OpenAI/DeepSeek/Custom to the
    /// default list would then change what an existing install tries first.
    pub fn set_api_keys(&self, keys: std::collections::HashMap<String, String>) {
        self.config
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .api_keys = keys;
    }

    /// Mutate the live configuration in place, then normalise the chain.
    pub fn update_config<F: FnOnce(&mut RouterConfig)>(&self, update: F) {
        let mut guard = self
            .config
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        update(&mut guard);
        guard.apply_provider_chain();
    }

    /// The provider scorecard's measured records (best first). Read-only
    /// view for the surfaces; routing consults the same data.
    pub fn provider_ranking(&self) -> Vec<crate::provider_scorecard::ProviderRecord> {
        self.scorecard.ranking()
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
    /// Adaptive trimming + CTX-COMPRESSION context compression applied first, then
    /// speculative cache is probed before any HTTP egress.
    pub async fn dispatch_prompt(&self, prompt: &str, system: &str) -> Result<String> {
        // One snapshot per request: a settings change saved while this
        // request is in flight applies to the next one, never half-way
        // through this one.
        let cfg = self.config();
        // CTX-COMPRESSION: Context compression against token budget before provider dispatch.
        let budget = cfg.context_window_tokens as usize;
        let (prompt_owned, system_owned) = {
            let est = |s: &str| s.len().div_ceil(4);
            if est(prompt) + est(system) > budget {
                let compressor = crate::compression::ContextCompressor::new(budget);
                let (p, s, _m) = compressor.compress(prompt, system);
                (p, s)
            } else {
                // Budget already satisfied — just trim head as before
                trim_to_window(prompt, system, cfg.context_window_tokens, cfg.trim_strategy)
            }
        };
        let prompt = prompt_owned.as_str();
        let system = system_owned.as_str();

        // Speculative cache probe (hash includes model so fallback model
        // differences don't collide). Record saved tokens on hit.
        let primary_model = cfg.primary_model.clone();
        let cache_key = SpeculativeCache::hash_key(prompt, system, &primary_model);

        if let Some(cached) = self.cache.get(cache_key) {
            let saved = (cached.len() / 4) as u64;
            self.metrics.record_saved(saved);
            info!(cache_hit = true, saved_tokens = saved, provider = %cfg.primary_provider, "speculative cache hit");
            return Ok(cached);
        }

        // ------------------------------------------------------------------
        // Explicit synthetic provider — honoured unconditionally.
        // ------------------------------------------------------------------
        // `ModelProvider::SyntheticOffline` is an explicit provider selection,
        // not a degradation. When it is the primary provider it is honoured
        // *before* any API key is read, before the persistent vector cache is
        // consulted, and before any HTTP request is built. Ambient credentials
        // (`OPENROUTER_API_KEY`, `ANTHROPIC_API_KEY`, ...) and stale on-disk
        // state therefore cannot change this path: the returned payload is a
        // pure function of the prompt and system text.
        //
        // Position is deliberate: *after* the in-memory speculative cache probe
        // (so repeat prompts still hit the cache and record saved tokens) and
        // *before* the on-disk vector cache (so no persisted state can satisfy
        // the request).
        if cfg.primary_provider == ModelProvider::SyntheticOffline {
            let synthetic = Self::synthetic_response(prompt, system);
            let inp = ((prompt.len() + system.len()) / 4) as u64;
            let out = (synthetic.len() / 4) as u64;
            self.metrics.record_usage(inp, out);
            // SCORECARD: the explicit synthetic provider genuinely served this
            // request, so record a measured success. (Degraded synthetic
            // responses further below are deliberately NOT credited to any
            // real provider — the scorecard drives routing and must only
            // reflect outcomes a named provider actually delivered.)
            self.scorecard
                .record(&cfg.primary_provider.to_string(), true, 0);
            self.cache.insert(cache_key, synthetic.clone());
            if let Some(vstore) = &self.vector_cache {
                let emb = mock_embed(prompt, 32);
                if let Err(e) = vstore.insert_entry(prompt, &synthetic, &emb) {
                    tracing::warn!(error = %e, "vector cache insert failed for explicit synthetic response");
                }
            }
            info!(
                provider = "synthetic-offline",
                input_tokens = inp,
                output_tokens = out,
                "dispatched prompt via explicit synthetic provider"
            );
            return Ok(synthetic);
        }

        // VECTOR-CACHE: vector cache similarity retrieval (offline-capable, before remote egress)
        if let Some(vstore) = &self.vector_cache {
            let emb = mock_embed(prompt, 32);
            match vstore.find_similar(&emb, 0.88) {
                Ok(Some(hit)) => {
                    // FIX D1: reject similarity hits whose prompt_hash differs from the
                    // current prompt. The 0.88 threshold can return a different prompt's
                    // cached response (e.g., the plan response served for tool-call/verification
                    // prompts), causing cross-contamination: fabricated tool calls, verification
                    // failures, and spurious success:false reports.
                    // `prompt_hash` is the same helper `insert_entry` stores with, so the
                    // comparison is exact by construction (one hashing implementation).
                    let current_hash = prompt_hash(prompt);
                    if current_hash == hit.prompt_hash {
                        info!(event = "telemetry:cache_hit", prompt_hash = %hit.prompt_hash, "vector cache hit — returning cached response without provider call");
                        self.metrics
                            .record_saved((hit.response_text.len() / 4) as u64);
                        self.cache.insert(cache_key, hit.response_text.clone());
                        return Ok(hit.response_text);
                    } else {
                        tracing::debug!(
                            event = "telemetry:cache_miss_similarity_hash_mismatch",
                            "vector cache similarity hit rejected: stored_hash={} current_hash={}",
                            &hit.prompt_hash[..16],
                            &current_hash[..16]
                        );
                    }
                }
                Ok(None) => {
                    tracing::debug!(
                        event = "telemetry:cache_miss",
                        "vector cache miss — proceeding to provider dispatch"
                    );
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
        let primary_key = cfg.api_key(&cfg.primary_provider);
        let fallback_key = cfg.api_key(&cfg.fallback_provider);
        let offline_fast = primary_key.is_none() && fallback_key.is_none();

        // When both providers require keys and none are present, go synthetic
        // immediately. When one side is Ollama (key always None) we still
        // attempt the network call first; if it fails we degrade to synthetic.
        let both_need_keys = cfg.primary_provider != ModelProvider::LocalOllama
            && cfg.fallback_provider != ModelProvider::LocalOllama;
        if offline_fast && both_need_keys {
            let synthetic = Self::synthetic_response(prompt, system);
            let inp = ((prompt.len() + system.len()) / 4) as u64;
            let out = (synthetic.len() / 4) as u64;
            self.metrics.record_usage(inp, out);
            self.cache.insert(cache_key, synthetic.clone());
            // VECTOR-CACHE: persist synthetic response to vector cache asynchronously (best-effort)
            if let Some(vstore) = &self.vector_cache {
                let emb = mock_embed(prompt, 32);
                if let Err(e) = vstore.insert_entry(prompt, &synthetic, &emb) {
                    tracing::warn!(error = %e, "vector cache insert failed for synthetic response");
                }
            }
            info!(
                provider = "synthetic-offline",
                input_tokens = inp,
                output_tokens = out,
                "dispatched prompt offline"
            );
            return Ok(synthetic);
        }

        // Attempt primary. Provider selection is measurement-aware: when the
        // scorecard has enough samples, the healthier configured provider is
        // attempted first (record outcomes on both paths).
        let primary_latency = crate::provider_scorecard::LatencyGuard::start();
        let primary_err: Option<anyhow::Error> = match self
            .call_provider(&cfg.primary_provider, &cfg.primary_model, prompt, system)
            .await
        {
            Ok(text) => {
                self.scorecard.record(
                    &cfg.primary_provider.to_string(),
                    true,
                    primary_latency.elapsed_ms(),
                );
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
                self.scorecard.record(
                    &cfg.primary_provider.to_string(),
                    false,
                    primary_latency.elapsed_ms(),
                );
                // Log telemetry:fallback event on retryable primary failure.
                self.metrics.record_fallback();
                let provider_name = cfg.primary_provider.to_string();
                info!(event = "telemetry:fallback", provider = %provider_name, error = %e, "primary provider failed, attempting fallback");
                Some(e)
            }
            Err(e) => {
                self.scorecard.record(
                    &cfg.primary_provider.to_string(),
                    false,
                    primary_latency.elapsed_ms(),
                );
                if e.to_string().contains("429") || e.to_string().to_lowercase().contains("rate") {
                    // Log telemetry:fallback event on rate-limited primary.
                    self.metrics.record_fallback();
                    let provider_name = cfg.primary_provider.to_string();
                    info!(event = "telemetry:fallback", provider = %provider_name, error = %e, "primary provider rate-limited, attempting fallback");
                    Some(e)
                } else {
                    return Err(e);
                }
            }
        };
        let last_err = primary_err;

        if cfg.max_retries == 0 {
            return Err(last_err.unwrap());
        }

        self.metrics.record_fallback();
        let fallback_provider = cfg.fallback_provider.clone();
        let fallback_model = cfg.fallback_model.clone();

        info!(event = "telemetry:provider_failover", from = %cfg.primary_provider, to = %fallback_provider, "falling back to configured fallback provider");

        info!(provider = %fallback_provider, model = %fallback_model, "dispatching to fallback provider");

        let fallback_latency = crate::provider_scorecard::LatencyGuard::start();
        match self
            .call_provider(&fallback_provider, &fallback_model, prompt, system)
            .await
        {
            Ok(text) => {
                self.scorecard.record(
                    &fallback_provider.to_string(),
                    true,
                    fallback_latency.elapsed_ms(),
                );
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
                self.scorecard.record(
                    &fallback_provider.to_string(),
                    false,
                    fallback_latency.elapsed_ms(),
                );
                // CHAIN-CONTINUATION (AGENT-JOURNEY fix): primary + configured
                // fallback are not the whole chain. `provider_configs` already
                // advertises an ordered fallback chain ending in
                // SyntheticOffline, but dispatch used to stop after one
                // fallback — so a failed primary (401) plus a failed fallback
                // (Ollama unreachable) killed the run even when the chain
                // still had usable entries, and stale ambient credentials
                // (present key => no synthetic degrade) turned a recoverable
                // provider outage into `Task failed`. Now: walk the enabled
                // chain entries (skipping primary and fallback already tried);
                // a chain entry that succeeds wins — and SyntheticOffline as
                // the last chain entry is honoured unconditionally by
                // `call_provider`, which makes the offline degrade the
                // configuration-driven final resort rather than a
                // "no keys anywhere" special case.
                let tried = [
                    Some(cfg.primary_provider.to_string()),
                    Some(fallback_provider.to_string()),
                ]
                .into_iter()
                .flatten()
                .collect::<std::collections::HashSet<String>>();

                let mut chain: Vec<&crate::router::ProviderConfig> =
                    cfg.provider_configs.iter().filter(|c| c.enabled).collect();
                chain.sort_by_key(|c| c.fallback_order);

                for entry in chain {
                    let model_provider = match kind_to_model_provider(&entry.kind) {
                        Some(p) => p,
                        None => continue,
                    };
                    if tried.contains(&model_provider.to_string()) {
                        continue;
                    }
                    info!(
                        event = "telemetry:provider_failover",
                        from = %fallback_provider,
                        to = %entry.kind,
                        step = "chain continuation",
                        "continuing the configured provider chain after primary+fallback failed"
                    );
                    self.metrics.record_fallback();
                    let latency = crate::provider_scorecard::LatencyGuard::start();
                    let model_name = entry.model.clone();
                    match self
                        .call_provider(&model_provider, &model_name, prompt, system)
                        .await
                    {
                        Ok(text) => {
                            self.scorecard.record(
                                &model_provider.to_string(),
                                true,
                                latency.elapsed_ms(),
                            );
                            self.cache.insert(cache_key, text.clone());
                            if let Some(vstore) = &self.vector_cache {
                                let emb = mock_embed(prompt, 32);
                                if let Err(e) = vstore.insert_entry(prompt, &text, &emb) {
                                    tracing::warn!(error = %e, "vector cache insert failed for chain fallback response");
                                }
                            }
                            return Ok(text);
                        }
                        Err(e) => {
                            self.scorecard.record(
                                &model_provider.to_string(),
                                false,
                                latency.elapsed_ms(),
                            );
                            warn!(provider = %entry.kind, error = %e, "chain-continuation provider failed; moving on");
                        }
                    }
                }

                let msg = format!(
                    "all providers failed — primary: {}, fallback ({}): {}, chain exhausted",
                    last_err.map(|e| e.to_string()).unwrap_or_default(),
                    fallback_provider,
                    fallback_err
                );
                Err(anyhow::anyhow!(msg))
            }
        }
    }

    /// Streaming variant — asks the selected provider for a *real* stream and
    /// forwards its deltas as they arrive.
    ///
    /// This used to call `dispatch_prompt` and then re-chunk the finished string by
    /// line, which looked like streaming in the UI while the provider had already
    /// answered in one piece. Now each provider module owns its own wire protocol
    /// (SSE for Anthropic and OpenAI-compatible, NDJSON for Ollama) and the router
    /// only chooses the provider, forwards chunks, and records usage.
    ///
    /// Offline/synthetic responses are still available and are explicitly labelled
    /// as synthetic rather than dressed up as a provider stream.
    ///
    /// The callback receives the provider-neutral [`StreamChunk`], not a reduced
    /// delta type: usage, finish reason, tool-call deltas, structured errors and
    /// provenance all reach the consumer without a second translation layer. A
    /// consumer that only needs `{ index, delta, done }` projects it at its own
    /// boundary (see `providers::compat`).
    pub async fn dispatch_stream<F>(
        &self,
        prompt: &str,
        system: &str,
        mut on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) + Send,
    {
        // A stream with no external cancel source still exposes the handle shape, so
        // callers can migrate to `dispatch_stream_cancellable` without an API split.
        let cancel = crate::providers::streaming::CancelToken::new();
        self.dispatch_stream_cancellable(prompt, system, cancel, &mut on_chunk)
            .await
    }

    /// Streaming variant with an explicit cancellation token.
    ///
    /// Cancelling the token aborts the in-flight HTTP request (the provider modules
    /// drop the response body, which closes the connection); it does not merely stop
    /// forwarding chunks.
    pub async fn dispatch_stream_cancellable<F>(
        &self,
        prompt: &str,
        system: &str,
        cancel: crate::providers::streaming::CancelToken,
        on_chunk: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) + Send,
    {
        let cfg = self.config();
        let (prompt_owned, system_owned) =
            trim_to_window(prompt, system, cfg.context_window_tokens, cfg.trim_strategy);
        let prompt = prompt_owned.as_str();
        let system = system_owned.as_str();

        // Same offline decision `dispatch_prompt` makes: if neither the primary nor
        // the fallback can possibly have a credential, do not open a socket.
        let primary_key = cfg.api_key(&cfg.primary_provider);
        let fallback_key = cfg.api_key(&cfg.fallback_provider);
        let both_need_keys = cfg.primary_provider != ModelProvider::LocalOllama
            && cfg.fallback_provider != ModelProvider::LocalOllama;
        let offline = (primary_key.is_none() && fallback_key.is_none() && both_need_keys)
            || cfg.primary_provider == ModelProvider::SyntheticOffline;

        if offline {
            // The synthetic path produces the same neutral chunks as a real
            // provider, marked synthetic; the dispatcher forwards them unchanged.
            let outcome = crate::providers::synthetic::stream(prompt, system, on_chunk);
            self.metrics.record_usage(
                ((prompt.len() + system.len()) / 4) as u64,
                (outcome.text.len() / 4) as u64,
            );
            return Ok(outcome.text);
        }

        // Attempt the primary, then the configured fallback, then the remaining
        // enabled chain entries — the same order `dispatch_prompt` uses. A provider
        // that has already forwarded text is never retried on another provider: that
        // would duplicate output in the consumer.
        let mut candidates: Vec<(ModelProvider, String)> = vec![
            (cfg.primary_provider.clone(), cfg.primary_model.clone()),
            (cfg.fallback_provider.clone(), cfg.fallback_model.clone()),
        ];
        if cfg.max_retries > 0 {
            let mut entries = cfg.provider_configs.clone();
            entries.sort_by_key(|e| e.fallback_order);
            for entry in entries {
                if !entry.enabled {
                    continue;
                }
                let Some(provider) = kind_to_model_provider(&entry.kind) else {
                    continue;
                };
                if candidates.iter().any(|(p, _)| *p == provider) {
                    continue;
                }
                candidates.push((provider, entry.model.clone()));
            }
        }

        let mut last_err: Option<anyhow::Error> = None;
        for (provider, model) in candidates {
            let started = crate::provider_scorecard::LatencyGuard::start();
            match self
                .stream_one_provider(&provider, &model, prompt, system, &cfg, &cancel, on_chunk)
                .await
            {
                Ok(text) => {
                    self.scorecard
                        .record(&provider.to_string(), true, started.elapsed_ms());
                    return Ok(text);
                }
                Err(StreamAttemptError::Cancelled) => {
                    self.scorecard
                        .record(&provider.to_string(), false, started.elapsed_ms());
                    return Ok(String::new());
                }
                Err(StreamAttemptError::Failed(err)) => {
                    self.scorecard
                        .record(&provider.to_string(), false, started.elapsed_ms());
                    warn!(provider = %provider, error = %err, "streaming provider failed");
                    last_err = Some(err);
                }
                Err(StreamAttemptError::AlreadyStreamed(err)) => {
                    // Partial output already reached the consumer; retrying elsewhere
                    // would duplicate it.
                    self.scorecard
                        .record(&provider.to_string(), false, started.elapsed_ms());
                    return Err(err);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no streaming provider was configured")))
    }

    /// One streaming attempt against one provider.
    #[allow(clippy::too_many_arguments)]
    async fn stream_one_provider<F>(
        &self,
        provider: &ModelProvider,
        model: &str,
        prompt: &str,
        system: &str,
        cfg: &RouterConfig,
        cancel: &crate::providers::streaming::CancelToken,
        on_chunk: &mut F,
    ) -> std::result::Result<String, StreamAttemptError>
    where
        F: FnMut(StreamChunk) + Send,
    {
        let request = crate::providers::streaming::ChatRequest::new(model, system, prompt);
        let outcome = self
            .stream_request(provider, model, &request, cfg, cancel, on_chunk)
            .await?;
        Ok(outcome.text)
    }

    /// One provider round for a request the caller built.
    ///
    /// Split out of [`Self::stream_one_provider`] so a caller that owns the
    /// conversation — the native tool loop, which must carry the assistant turn
    /// and its tool results — can supply the whole request instead of two
    /// strings. Everything else (capability check, egress accounting, usage,
    /// error classification) is shared, so a tool-bearing request cannot take a
    /// shortcut a plain one could not.
    async fn stream_request<F>(
        &self,
        provider: &ModelProvider,
        model: &str,
        request: &crate::providers::streaming::ChatRequest,
        cfg: &RouterConfig,
        cancel: &crate::providers::streaming::CancelToken,
        on_chunk: &mut F,
    ) -> std::result::Result<crate::providers::streaming::StreamOutcome, StreamAttemptError>
    where
        F: FnMut(StreamChunk) + Send,
    {
        use crate::providers::streaming::FinishReason;

        // A provider that cannot stream is reported as such rather than faked.
        if !crate::providers::capabilities::provider_capabilities(provider).streaming {
            return Err(StreamAttemptError::Failed(anyhow::anyhow!(
                "provider {provider} does not support streaming"
            )));
        }
        let timeout = Duration::from_millis(cfg.provider_timeout_ms(provider));
        let base = cfg.base_url(provider);
        let api_key = cfg.api_key(provider);

        // Atomic, not `Cell`, because the streaming future must stay `Send` for
        // callers that drive it from another thread.
        let emitted = std::sync::atomic::AtomicBool::new(false);
        let mut forward = |chunk: StreamChunk| {
            if chunk.kind.text.is_some() || chunk.done {
                emitted.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            on_chunk(chunk);
        };

        #[cfg(test)]
        NETWORK_EGRESS_COUNT.with(|c| c.set(c.get() + 1));

        let outcome = match provider {
            ModelProvider::Anthropic => {
                crate::providers::anthropic::stream(
                    &self.http,
                    &base,
                    api_key.as_deref(),
                    request,
                    timeout,
                    cancel,
                    &mut forward,
                )
                .await
            }
            ModelProvider::LocalOllama => {
                crate::providers::ollama::stream(
                    &self.http,
                    &base,
                    request,
                    timeout,
                    cancel,
                    &mut forward,
                )
                .await
            }
            ModelProvider::SyntheticOffline => {
                let outcome = crate::providers::synthetic::stream(
                    &request.prompt,
                    &request.system,
                    &mut forward,
                );
                self.metrics.record_usage(
                    ((request.prompt.len() + request.system.len()) / 4) as u64,
                    (outcome.text.len() / 4) as u64,
                );
                return Ok(outcome);
            }
            _ => {
                let extra: Vec<(String, String)> = if *provider == ModelProvider::OpenRouter {
                    vec![
                        (
                            "HTTP-Referer".to_string(),
                            "https://zylcode.dev".to_string(),
                        ),
                        ("X-Title".to_string(), "ZylCode".to_string()),
                    ]
                } else {
                    Vec::new()
                };
                let target = crate::providers::openai_compatible::CompatTarget {
                    base_url: &base,
                    api_key: api_key.as_deref(),
                    provider: &provider.to_string(),
                    timeout,
                    extra_headers: &extra,
                };
                crate::providers::openai_compatible::stream(
                    &self.http,
                    &target,
                    request,
                    cancel,
                    &mut forward,
                )
                .await
            }
        };

        match outcome {
            Ok(outcome) => {
                if outcome.finish == FinishReason::Cancelled {
                    return Err(StreamAttemptError::Cancelled);
                }
                let inp = outcome
                    .usage
                    .as_ref()
                    .and_then(|u| u.input_tokens)
                    .unwrap_or(((request.prompt.len() + request.system.len()) / 4) as u64);
                let out = outcome
                    .usage
                    .as_ref()
                    .and_then(|u| u.output_tokens)
                    .unwrap_or((outcome.text.len() / 4) as u64);
                self.metrics.record_usage(inp, out);
                info!(provider = %provider, model = %model, input_tokens = inp, output_tokens = out, "provider stream completed");
                Ok(outcome)
            }
            Err(err) => {
                let err = anyhow::anyhow!(
                    "provider {provider} stream failed ({:?}): {}",
                    err.kind,
                    err.message
                );
                if emitted.load(std::sync::atomic::Ordering::SeqCst) {
                    Err(StreamAttemptError::AlreadyStreamed(err))
                } else if cancel.is_cancelled() {
                    Err(StreamAttemptError::Cancelled)
                } else {
                    Err(StreamAttemptError::Failed(err))
                }
            }
        }
    }

    /// One round of an ongoing conversation, chosen the same way
    /// [`Self::dispatch_stream_cancellable`] chooses.
    ///
    /// Returns the neutral outcome including any tool calls, so a caller that
    /// owns the conversation can continue it. The first provider that answers
    /// wins; a provider that fails before producing anything is skipped exactly
    /// as it is for a plain streaming request.
    pub async fn stream_turn(
        &self,
        request: &crate::providers::streaming::ChatRequest,
        cancel: &crate::providers::streaming::CancelToken,
    ) -> std::result::Result<
        crate::providers::streaming::StreamOutcome,
        crate::providers::streaming::ProviderError,
    > {
        use crate::providers::streaming::ProviderErrorKind;

        let cfg = self.config();
        let mut last: Option<crate::providers::streaming::ProviderError> = None;

        for (provider, model) in self.provider_candidates(&cfg) {
            // A caller-supplied model pins the round — the tool loop names the
            // model it was configured with. Otherwise the router's own model for
            // that provider wins, as it does everywhere else.
            let model = if request.model.is_empty() {
                model
            } else {
                request.model.clone()
            };
            let mut sent = request.clone();
            sent.model = model.clone();

            let mut sink = |_chunk: StreamChunk| {};
            match self
                .stream_request(&provider, &model, &sent, &cfg, cancel, &mut sink)
                .await
            {
                Ok(outcome) => return Ok(outcome),
                Err(StreamAttemptError::Cancelled) => {
                    return Err(crate::providers::streaming::ProviderError {
                        kind: ProviderErrorKind::Transport,
                        message: "cancelled".to_string(),
                        code: None,
                        raw: None,
                    });
                }
                Err(StreamAttemptError::Failed(err))
                | Err(StreamAttemptError::AlreadyStreamed(err)) => {
                    warn!(provider = %provider, error = %err, "tool-turn provider failed");
                    last = Some(crate::providers::streaming::ProviderError {
                        kind: ProviderErrorKind::ProviderRejected,
                        message: err.to_string(),
                        code: None,
                        raw: None,
                    });
                }
            }
        }

        Err(last.unwrap_or(crate::providers::streaming::ProviderError {
            kind: ProviderErrorKind::InvalidEndpoint,
            message: "no provider was configured for a tool turn".to_string(),
            code: None,
            raw: None,
        }))
    }

    /// The provider order a request walks: primary, fallback, then the enabled
    /// chain entries in `fallback_order`.
    fn provider_candidates(&self, cfg: &RouterConfig) -> Vec<(ModelProvider, String)> {
        let mut candidates: Vec<(ModelProvider, String)> = vec![
            (cfg.primary_provider.clone(), cfg.primary_model.clone()),
            (cfg.fallback_provider.clone(), cfg.fallback_model.clone()),
        ];
        if cfg.max_retries > 0 {
            let mut entries = cfg.provider_configs.clone();
            entries.sort_by_key(|e| e.fallback_order);
            for entry in entries {
                if !entry.enabled {
                    continue;
                }
                let Some(provider) = kind_to_model_provider(&entry.kind) else {
                    continue;
                };
                if candidates.iter().any(|(p, _)| *p == provider) {
                    continue;
                }
                candidates.push((provider, entry.model.clone()));
            }
        }
        candidates
    }

    // -- internal provider call ---------------------------------------------

    async fn call_provider(
        &self,
        provider: &ModelProvider,
        model: &str,
        prompt: &str,
        system: &str,
    ) -> Result<String> {
        let cfg = self.config();
        // Explicit synthetic provider: return the deterministic offline payload
        // without reading a credential and without building or sending a
        // request. This is the second line of defence — `dispatch_prompt`
        // already short-circuits, but a synthetic provider must never be able
        // to reach the network even if reached directly.
        if *provider == ModelProvider::SyntheticOffline {
            return Ok(Self::synthetic_response(prompt, system));
        }

        // Apply adaptive trimming again per-provider (idempotent, cheap).
        let (prompt_owned, system_owned) =
            trim_to_window(prompt, system, cfg.context_window_tokens, cfg.trim_strategy);
        let prompt = prompt_owned.as_str();
        let system = system_owned.as_str();

        let base = cfg.base_url(provider);

        // Estimate input tokens for telemetry regardless of outcome.
        let est_input = ((prompt.len() + system.len()) / 4) as u64;

        // Request construction belongs to the provider module: this function knows
        // which provider to call and how to report the outcome, not what Anthropic's
        // or Ollama's body looks like.
        let request = crate::providers::streaming::ChatRequest::new(model, system, prompt);
        let api_key = cfg.api_key(provider);

        let (url, body, mut extra_headers) = match provider {
            ModelProvider::Anthropic => (
                crate::providers::anthropic::messages_url(&base),
                crate::providers::anthropic::messages_body(&request, false),
                crate::providers::anthropic::headers(api_key.as_deref()),
            ),
            ModelProvider::LocalOllama => (
                crate::providers::ollama::chat_url(&base),
                crate::providers::ollama::chat_body(&request, false),
                crate::providers::ollama::headers(api_key.as_deref()),
            ),
            _ => (
                crate::providers::openai_compatible::chat_url(&base),
                crate::providers::openai_compatible::chat_body(&request, false),
                crate::providers::openai_compatible::headers(api_key.as_deref()),
            ),
        };
        if *provider == ModelProvider::OpenRouter {
            extra_headers.extend(crate::providers::openai_compatible::openrouter_headers());
        }

        // Per-provider timeout when the chain entry names one; the shared
        // HTTP client only carries the router-wide default.
        let mut req = self
            .http
            .post(&url)
            .json(&body)
            .timeout(Duration::from_millis(cfg.provider_timeout_ms(provider)));
        for (name, value) in extra_headers {
            req = req.header(name, value);
        }

        // Test-only spy: record that we are about to egress to a provider.
        #[cfg(test)]
        NETWORK_EGRESS_COUNT.with(|c| c.set(c.get() + 1));

        let resp = req.send().await.context("HTTP request failed")?;
        let status = resp.status();

        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            let err = anyhow::anyhow!("provider {provider} returned {status}: {body_text}");
            // Attach status for retry classification.
            return Err(annotate_status(err, status.as_u16()));
        }

        let json: serde_json::Value = resp.json().await.context("failed to parse JSON response")?;

        // Response shape is the provider module's business too.
        let completion = match provider {
            ModelProvider::Anthropic => crate::providers::anthropic::parse_completion(&json),
            ModelProvider::LocalOllama => crate::providers::ollama::parse_completion(&json),
            _ => crate::providers::openai_compatible::parse_completion(&json),
        };
        let text = completion.text;
        let inp = completion.usage.input_tokens.unwrap_or(est_input);
        let out = completion
            .usage
            .output_tokens
            .unwrap_or((text.len() / 4) as u64);
        self.metrics.record_usage(inp, out);
        info!(provider = %provider, model = %model, input_tokens = inp, output_tokens = out, "provider call succeeded");
        Ok(text)
    }

    fn synthetic_response(prompt: &str, system: &str) -> String {
        // The synthetic payload lives in the provider module now, so the offline
        // answer has exactly one definition and stays explicitly labelled.
        crate::providers::synthetic::response(prompt, system)
    }
}

// ---------------------------------------------------------------------------
// Streaming attempt outcome
// ---------------------------------------------------------------------------

/// Why one streaming attempt did not produce a completed answer.
///
/// The distinction that matters is whether the consumer already received output:
/// a silent failure may be retried on the next provider, while a failure after
/// partial output must be surfaced instead of duplicating text.
enum StreamAttemptError {
    /// The caller cancelled; no retry and no error.
    Cancelled,
    /// The attempt failed before anything was forwarded; another provider may be tried.
    Failed(anyhow::Error),
    /// Output was already forwarded, so this failure is terminal for the request.
    AlreadyStreamed(anyhow::Error),
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Provider connection probe
// ---------------------------------------------------------------------------
//
// The settings surface can list a provider chain, but before this there was no
// way — for the operator or for a test — to ask "does this configuration
// actually work?". The panel's own fields were not evidence: an endpoint and a
// model can be saved and still be wrong, and "an API key is present" says
// nothing about whether the provider accepts it.
//
// [`probe_provider`] answers that question the only way it can be answered: by
// sending one real request through [`TokenRouter::call_provider`] — the same
// function `dispatch_prompt` uses — and classifying the outcome. It is
// deliberately **not** routed through `dispatch_prompt`, because that would
// apply fallback and report a *secondary* provider's success as if the
// configuration under test had worked.

/// Outcome of one provider probe.
///
/// The variants are the distinctions an operator can act on, not a restatement
/// of HTTP codes: `AuthenticationFailed` means "fix the key",
/// `InvalidEndpoint` means "fix the URL", `ModelUnavailable` means "fix the
/// model name", `NetworkError` means "the host is unreachable from here", and
/// `ConfigurationIncomplete` means nothing was sent at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    /// The provider answered with a usable completion.
    Connected,
    /// Synthetic offline: no network is involved by construction.
    OfflineProvider,
    /// A credential the provider requires is missing; no request was sent.
    ConfigurationIncomplete,
    /// 401/403 — the provider rejected the credential.
    AuthenticationFailed,
    /// The URL is not reachable or not an API endpoint (404/405/HTML, bad URL).
    InvalidEndpoint,
    /// The endpoint answered but does not serve the requested model.
    ModelUnavailable,
    /// The provider refused the request (400/422/429/5xx).
    ProviderRejected,
    /// Transport-level failure: DNS, refused connection, timeout, TLS.
    NetworkError,
}

impl ProbeStatus {
    /// Stable machine-readable label, mirrored by the desktop UI.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::OfflineProvider => "offline_provider",
            Self::ConfigurationIncomplete => "configuration_incomplete",
            Self::AuthenticationFailed => "authentication_failed",
            Self::InvalidEndpoint => "invalid_endpoint",
            Self::ModelUnavailable => "model_unavailable",
            Self::ProviderRejected => "provider_rejected",
            Self::NetworkError => "network_error",
        }
    }
}

/// Evidence for one probe: what was contacted, with which model and credential
/// source, what came back, and how long it took.
///
/// `detail` carries the provider's own message (truncated) so a failure can be
/// diagnosed without a second run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProbe {
    pub provider: String,
    pub model: String,
    /// The URL actually requested, or the provider's default when none was set.
    pub endpoint: String,
    /// Where the credential came from: `config`, `env:NAME`, `not_required`,
    /// or `none`.
    pub key_source: String,
    pub status: ProbeStatus,
    /// The provider's HTTP status when a response was received.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    pub latency_ms: u64,
    /// Characters returned by a successful probe (0 otherwise).
    pub response_chars: usize,
    pub detail: String,
}

impl ProviderProbe {
    pub fn ok(&self) -> bool {
        matches!(
            self.status,
            ProbeStatus::Connected | ProbeStatus::OfflineProvider
        )
    }
}

/// Read the `[status=NNN]` annotation [`annotate_status`] embeds in an error.
fn status_in(err: &str) -> Option<u16> {
    let idx = err.find("[status=")? + "[status=".len();
    let digits: String = err[idx..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// The message body of a provider error, for `detail`.
fn body_in(err: &str) -> &str {
    match err.find(": ") {
        Some(i) => err[i + 2..].split(" [status=").next().unwrap_or(""),
        None => err,
    }
}

/// Classify a provider error into an actionable status.
///
/// Pure, so every branch is unit-testable without a socket.
fn classify_probe_error(err: &str) -> (ProbeStatus, Option<u16>) {
    let Some(code) = status_in(err) else {
        // No HTTP status: the request never completed. A URL that reqwest could
        // not even build is a configuration error, not a network fault.
        let lower = err.to_lowercase();
        let status =
            if lower.contains("relative url without a base") || lower.contains("invalid url") {
                ProbeStatus::InvalidEndpoint
            } else {
                ProbeStatus::NetworkError
            };
        return (status, None);
    };
    let body = body_in(err).to_lowercase();
    let mentions_model = body.contains("model");
    let status = match code {
        401 | 403 => ProbeStatus::AuthenticationFailed,
        404 | 405 => {
            if mentions_model {
                ProbeStatus::ModelUnavailable
            } else {
                ProbeStatus::InvalidEndpoint
            }
        }
        400 | 422 => {
            if mentions_model {
                ProbeStatus::ModelUnavailable
            } else {
                ProbeStatus::ProviderRejected
            }
        }
        _ => ProbeStatus::ProviderRejected,
    };
    (status, Some(code))
}

/// Which credential a probe would use, named so the operator can see it.
fn key_source_for(cfg: &RouterConfig, kind: &ProviderKind, provider: &ModelProvider) -> String {
    if *provider == ModelProvider::SyntheticOffline {
        return "not_required".to_string();
    }
    if *provider == ModelProvider::LocalOllama {
        return "not_required".to_string();
    }
    let name = provider.to_string();
    if cfg
        .api_keys
        .get(&name)
        .map(|k| !k.is_empty())
        .unwrap_or(false)
    {
        return "config".to_string();
    }
    let env = match kind {
        ProviderKind::Anthropic => "ANTHROPIC_API_KEY",
        ProviderKind::OpenRouter => "OPENROUTER_API_KEY",
        ProviderKind::OpenAi => "OPENAI_API_KEY",
        ProviderKind::DeepSeek => "DEEPSEEK_API_KEY",
        ProviderKind::Custom => "OPENAI_COMPATIBLE_API_KEY",
        ProviderKind::Ollama => return "not_required".to_string(),
        ProviderKind::SyntheticOffline => return "not_required".to_string(),
    };
    match std::env::var(env).ok().map(|v| !v.trim().is_empty()) {
        Some(true) => format!("env:{env}"),
        _ => "none".to_string(),
    }
}

/// Probe one provider with the settings the operator typed.
///
/// `endpoint` and `model` override the defaults when supplied; the credential
/// is resolved exactly as dispatch resolves it (config, then the provider's
/// environment variable). A provider that requires a key and has none returns
/// [`ProbeStatus::ConfigurationIncomplete`] **without sending a request** —
/// there is nothing to learn from an unauthenticated call that the missing key
/// does not already tell us, and probing must not become an accidental way to
/// hammer a provider.
///
/// A successful probe is not a claim that dispatch uses these values: the
/// caller is testing a candidate configuration, and the desktop surfaces the
/// values it probed in the result.
pub async fn probe_provider(
    kind: &ProviderKind,
    endpoint: Option<&str>,
    model: Option<&str>,
    timeout_ms: u64,
) -> ProviderProbe {
    probe_provider_with_config(&RouterConfig::from_env(), kind, endpoint, model, timeout_ms).await
}

/// Probe against a specific configuration instead of a fresh environment
/// snapshot.
///
/// The settings surface uses this so "Test connection" exercises the same
/// values dispatch will use — including a credential that only exists in the
/// process (never written to disk) and the endpoint normalisation performed by
/// [`RouterConfig::apply_provider_chain`].
pub async fn probe_provider_with_config(
    base: &RouterConfig,
    kind: &ProviderKind,
    endpoint: Option<&str>,
    model: Option<&str>,
    timeout_ms: u64,
) -> ProviderProbe {
    let provider = match kind_to_model_provider(kind) {
        Some(p) => p,
        None => {
            return ProviderProbe {
                provider: kind.to_string(),
                model: model.unwrap_or_default().to_string(),
                endpoint: endpoint.unwrap_or_default().to_string(),
                key_source: "none".to_string(),
                status: ProbeStatus::ConfigurationIncomplete,
                http_status: None,
                latency_ms: 0,
                response_chars: 0,
                detail: format!("provider {kind} has no dispatch implementation"),
            }
        }
    };

    let mut cfg = base.clone();
    cfg.timeout_ms = timeout_ms.clamp(1_000, 300_000);

    // Model: explicit override, else the chain entry's, else the router default.
    let model = model
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .or_else(|| {
            cfg.provider_configs
                .iter()
                .find(|c| c.kind == *kind)
                .map(|c| c.model.clone())
                .filter(|m| !m.is_empty())
        })
        .unwrap_or_else(|| cfg.primary_model.clone());

    // Route a draft endpoint through the same normalisation dispatch uses
    // (`apply_provider_chain`), so a probe cannot pass against a URL the
    // request path would never build.
    if let Some(url) = endpoint
        .map(|e| e.trim().to_string())
        .filter(|e| !e.is_empty())
    {
        match cfg.provider_configs.iter_mut().find(|c| c.kind == *kind) {
            Some(entry) => entry.endpoint = url,
            None => cfg.provider_configs.push(ProviderConfig {
                kind: kind.clone(),
                model: model.clone(),
                endpoint: url,
                timeout_ms,
                enabled: true,
                fallback_order: u32::MAX,
                requires_api_key: !matches!(
                    kind,
                    ProviderKind::Ollama | ProviderKind::SyntheticOffline
                ),
            }),
        }
        cfg.apply_provider_chain();
    }

    let effective_endpoint = format!("{}{}", cfg.base_url(&provider), provider.completions_path());
    let key_source = key_source_for(&cfg, kind, &provider);

    if *kind == ProviderKind::SyntheticOffline {
        return ProviderProbe {
            provider: provider.to_string(),
            model,
            endpoint: "(no network)".to_string(),
            key_source,
            status: ProbeStatus::OfflineProvider,
            http_status: None,
            latency_ms: 0,
            response_chars: 0,
            detail: "synthetic offline is a first-class provider: it never sends a request"
                .to_string(),
        };
    }

    if provider != ModelProvider::LocalOllama && cfg.api_key(&provider).is_none() {
        return ProviderProbe {
            provider: provider.to_string(),
            model,
            endpoint: effective_endpoint,
            key_source,
            status: ProbeStatus::ConfigurationIncomplete,
            http_status: None,
            latency_ms: 0,
            response_chars: 0,
            detail: format!(
                "no credential for {provider}: set {} or save a key for it; nothing was sent",
                match kind {
                    ProviderKind::Anthropic => "ANTHROPIC_API_KEY",
                    ProviderKind::OpenRouter => "OPENROUTER_API_KEY",
                    _ => "an API key",
                }
            ),
        };
    }

    let router = match TokenRouter::without_vector_cache(cfg) {
        Ok(r) => r,
        Err(e) => {
            return ProviderProbe {
                provider: provider.to_string(),
                model,
                endpoint: effective_endpoint,
                key_source,
                status: ProbeStatus::NetworkError,
                http_status: None,
                latency_ms: 0,
                response_chars: 0,
                detail: format!("could not build an HTTP client: {e:#}"),
            }
        }
    };

    let started = std::time::Instant::now();
    let outcome = router
        .call_provider(&provider, &model, "Reply with the single word OK.", "")
        .await;
    let latency_ms = started.elapsed().as_millis() as u64;

    match outcome {
        Ok(text) => ProviderProbe {
            provider: provider.to_string(),
            model,
            endpoint: effective_endpoint,
            key_source,
            status: ProbeStatus::Connected,
            http_status: None,
            latency_ms,
            response_chars: text.chars().count(),
            detail: "provider answered with a completion".to_string(),
        },
        Err(err) => {
            let message = format!("{err:#}");
            let (status, http_status) = classify_probe_error(&message);
            let body: String = body_in(&message).chars().take(300).collect();
            ProviderProbe {
                provider: provider.to_string(),
                model,
                endpoint: effective_endpoint,
                key_source,
                status,
                http_status,
                latency_ms,
                response_chars: 0,
                detail: body,
            }
        }
    }
}

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
    use crate::agent_protocol::AgentDecision;

    /// Serialises tests that mutate process-global provider environment
    /// variables. `cargo test` runs tests on multiple threads within one
    /// process, so without this lock an env-mutating test could be observed by
    /// another.
    ///
    /// It no longer has to cover the egress spy: `NETWORK_EGRESS_COUNT` is
    /// thread-local, so a counter delta cannot be attributed to the wrong test.
    /// This lock cannot be shared with tests in other modules of the crate, so
    /// anything that must be observable across modules has to be per-thread or
    /// per-instance rather than process-global.
    ///
    /// A `tokio::sync::Mutex` is used deliberately: these are `async` tests and
    /// the guard is intentionally held across await points, which a
    /// `std::sync::Mutex` guard must never be.
    static DISPATCH_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[test]
    fn default_router_config_has_sensible_models() {
        let cfg = RouterConfig::default();
        assert!(!cfg.primary_model.is_empty());
        assert!(!cfg.fallback_model.is_empty());
    }

    #[tokio::test]
    async fn router_records_provider_outcomes_to_the_scorecard() {
        // Mutates process-global env (scorecard path) — serialise like the
        // other env-sensitive dispatch tests.
        let _guard = DISPATCH_MUTEX.lock().await;

        // Isolated scorecard for this test.
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("ZYLCODE_SCORECARD_PATH", dir.path().join("sc.db"));

        // Synthetic offline primary is honoured unconditionally (no network);
        // it records a success outcome for the provider that served it.
        let cfg = RouterConfig {
            primary_provider: ModelProvider::SyntheticOffline,
            fallback_provider: ModelProvider::SyntheticOffline,
            ..RouterConfig::default()
        };
        let router = TokenRouter::without_vector_cache(cfg).unwrap();
        let out = router
            .dispatch_prompt("scorecard probe", "system")
            .await
            .unwrap();
        assert!(!out.is_empty());

        let ranking = router.provider_ranking();
        assert!(
            ranking
                .iter()
                .any(|r| r.provider == "synthetic-offline" && r.successes >= 1),
            "dispatch must record a measured success for the provider used: {ranking:?}"
        );
        std::env::remove_var("ZYLCODE_SCORECARD_PATH");
    }

    #[test]
    fn base_url_override_takes_precedence() {
        let mut cfg = RouterConfig::default();
        cfg.base_url_overrides.insert(
            "openrouter".to_string(),
            "https://custom.example".to_string(),
        );
        assert_eq!(
            cfg.base_url(&ModelProvider::OpenRouter),
            "https://custom.example"
        );
    }

    /// Process-global provider environment, saved and restored around a test.
    ///
    /// `cargo test` shares one environment across threads in one process, and
    /// the shell that launched it may already export provider keys, so every
    /// test that sets one must put back what it found — including on panic.
    struct EnvScope {
        saved: Vec<(&'static str, Option<String>)>,
    }

    impl EnvScope {
        fn capture(names: &[&'static str]) -> Self {
            Self {
                saved: names.iter().map(|k| (*k, std::env::var(k).ok())).collect(),
            }
        }
    }

    impl Drop for EnvScope {
        fn drop(&mut self) {
            for (name, value) in &self.saved {
                match value {
                    Some(v) => std::env::set_var(name, v),
                    None => std::env::remove_var(name),
                }
            }
        }
    }

    /// `RESPAN_API_KEY` is the whole switch: with it the hosted gateway becomes
    /// the primary provider and the credential reaches the request; without it
    /// routing is exactly what the default configuration says.
    #[tokio::test]
    async fn respan_gateway_is_selected_by_its_api_key() {
        let _guard = DISPATCH_MUTEX.lock().await;
        let _env = EnvScope::capture(&[
            "RESPAN_API_KEY",
            "RESPAN_MODEL",
            "RESPAN_BASE_URL",
            "ZYLCODE_PRIMARY_MODEL",
        ]);

        // Absent key: nothing about routing moves.
        std::env::remove_var("RESPAN_API_KEY");
        std::env::remove_var("RESPAN_MODEL");
        std::env::remove_var("RESPAN_BASE_URL");
        std::env::remove_var("ZYLCODE_PRIMARY_MODEL");

        let untouched = RouterConfig::from_env();
        assert_eq!(
            untouched.primary_provider,
            RouterConfig::default().primary_provider,
            "an unset RESPAN_API_KEY must not change provider selection"
        );
        assert!(
            untouched.api_key(&ModelProvider::Respan).is_none(),
            "no Respan credential may be resolved when none is configured"
        );

        // Present key: the gateway becomes primary, with the gateway's own
        // endpoint and OpenAI-compatible request path.
        std::env::set_var(
            "RESPAN_API_KEY",
            "placeholder-credential-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        );
        let cfg = RouterConfig::from_env();
        assert_eq!(cfg.primary_provider, ModelProvider::Respan);
        assert_eq!(cfg.primary_model, RESPAN_DEFAULT_MODEL);
        assert_eq!(
            cfg.base_url(&ModelProvider::Respan),
            RESPAN_GATEWAY_BASE_URL
        );
        assert_eq!(
            ModelProvider::Respan.completions_path(),
            "/chat/completions"
        );
        assert!(
            cfg.api_key(&ModelProvider::Respan).is_some(),
            "the configured key must resolve for the gateway provider"
        );
        // The pre-existing chain is what a gateway failure falls back to.
        assert_eq!(
            cfg.fallback_provider,
            RouterConfig::default().fallback_provider
        );

        // Model precedence: RESPAN_MODEL beats ZYLCODE_PRIMARY_MODEL.
        std::env::set_var("ZYLCODE_PRIMARY_MODEL", "gateway-model-from-primary");
        assert_eq!(
            RouterConfig::from_env().primary_model,
            "gateway-model-from-primary"
        );
        std::env::set_var("RESPAN_MODEL", "gateway-model-from-respan");
        assert_eq!(
            RouterConfig::from_env().primary_model,
            "gateway-model-from-respan"
        );

        // Respan Enterprise endpoint, with the trailing slash its docs show.
        std::env::set_var("RESPAN_BASE_URL", "https://endpoint.respan.ai/api/");
        assert_eq!(
            RouterConfig::from_env().base_url(&ModelProvider::Respan),
            "https://endpoint.respan.ai/api",
            "RESPAN_BASE_URL must be honoured and stripped of its trailing slash"
        );

        // An explicit per-provider override still wins (test pinning, mirrors).
        let mut pinned = RouterConfig::from_env();
        pinned.base_url_overrides.insert(
            ModelProvider::Respan.to_string(),
            "http://127.0.0.1:1".to_string(),
        );
        assert_eq!(
            pinned.base_url(&ModelProvider::Respan),
            "http://127.0.0.1:1"
        );
    }

    /// `connect_respan_gateway` is the same switch, exposed for callers that
    /// build their configuration some other way than `from_env`.
    #[tokio::test]
    async fn connect_respan_gateway_reports_whether_it_routed() {
        let _guard = DISPATCH_MUTEX.lock().await;
        let _env = EnvScope::capture(&["RESPAN_API_KEY", "RESPAN_MODEL"]);

        std::env::remove_var("RESPAN_API_KEY");
        let mut cfg = RouterConfig::default();
        assert!(!cfg.connect_respan_gateway(), "no key means no rerouting");
        assert_eq!(
            cfg.primary_provider,
            RouterConfig::default().primary_provider
        );

        std::env::set_var(
            "RESPAN_API_KEY",
            "placeholder-credential-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        );
        assert!(cfg.connect_respan_gateway());
        assert_eq!(cfg.primary_provider, ModelProvider::Respan);

        // A blank key is not a key (`RESPAN_API_KEY=` in a shell profile).
        std::env::set_var("RESPAN_API_KEY", "   ");
        let mut blank = RouterConfig::default();
        assert!(
            !blank.connect_respan_gateway(),
            "a blank RESPAN_API_KEY must not select the gateway"
        );
    }

    /// Build a router whose primary provider is the explicit synthetic
    /// provider. No credentials are configured and no network is reachable.
    fn synthetic_router() -> TokenRouter {
        let cfg = RouterConfig {
            primary_provider: ModelProvider::SyntheticOffline,
            fallback_provider: ModelProvider::SyntheticOffline,
            ..RouterConfig::default()
        };
        // Hermetic: no persistent vector cache. Using `new()` here made the result depend
        // on whatever `./vector_cache.db` existed on the machine — a stale entry would
        // satisfy the prompt and the test would pass without exercising this path at all.
        TokenRouter::without_vector_cache(cfg).unwrap()
    }

    /// The streaming boundary hands consumers the neutral contract itself, not a
    /// reduced projection of it: an offline answer can be told apart from a real
    /// one, and the reason the stream ended is visible without parsing payload text.
    #[tokio::test]
    async fn dispatch_stream_exposes_the_neutral_contract_at_the_boundary() {
        use crate::providers::streaming::FinishReason;

        let _guard = DISPATCH_MUTEX.lock().await;
        let router = synthetic_router();
        let egress_before = NETWORK_EGRESS_COUNT.with(std::cell::Cell::get);

        let mut chunks: Vec<StreamChunk> = Vec::new();
        let text = router
            .dispatch_stream("build a counter", "you are a code generator", |chunk| {
                chunks.push(chunk)
            })
            .await
            .unwrap();

        assert_eq!(
            NETWORK_EGRESS_COUNT.with(std::cell::Cell::get),
            egress_before,
            "the offline boundary must not perform network egress"
        );
        assert!(!chunks.is_empty(), "the boundary must forward chunks");
        assert!(
            chunks.iter().all(|c| c.synthetic),
            "every offline chunk must carry its provenance"
        );

        let terminals: Vec<&StreamChunk> = chunks.iter().filter(|c| c.done).collect();
        assert_eq!(terminals.len(), 1, "exactly one terminal chunk per stream");
        assert_eq!(
            terminals[0].finish_reason,
            Some(FinishReason::Stop),
            "the terminal chunk must say why the stream ended"
        );

        let streamed: String = chunks
            .iter()
            .filter_map(|c| c.kind.text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            streamed.trim_end(),
            text,
            "what the consumer saw must be what the call returned"
        );
    }

    #[tokio::test]
    async fn synthetic_offline_dispatch_returns_parseable_payload() {
        let _guard = DISPATCH_MUTEX.lock().await;
        let router = synthetic_router();

        let egress_before = NETWORK_EGRESS_COUNT.with(std::cell::Cell::get);
        let text = router
            .dispatch_prompt("build a counter", "you are a code generator")
            .await
            .unwrap();
        let egress_after = NETWORK_EGRESS_COUNT.with(std::cell::Cell::get);

        // The synthetic provider must be reached *without* attempting any HTTP
        // request. Asserting this is what makes the test hermetic: the previous
        // version of this test used the default provider chain and therefore
        // made a real call to openrouter.ai and localhost:11434 on every run,
        // passing only because both failed and the code degraded to synthetic.
        assert_eq!(
            egress_after, egress_before,
            "synthetic provider must not perform network egress"
        );

        // The synthetic offline path must emit a payload that the agent protocol can
        // actually consume. The contract is AgentDecision JSON (see `agent.rs`, which
        // instructs the model to "respond with valid JSON matching the AgentDecision
        // protocol"), NOT the legacy `<zylcode-response>` XML envelope. Asserting the
        // real contract — a successful parse — is stronger than a substring match,
        // because a substring match cannot catch a payload the consumer would reject.
        let decision: AgentDecision = serde_json::from_str(text.trim()).unwrap_or_else(|e| {
            panic!("synthetic response is not a valid AgentDecision: {e}\npayload: {text}")
        });
        assert!(
            matches!(decision, AgentDecision::Complete { .. }),
            "synthetic offline dispatch must complete the turn, got: {decision:?}"
        );
        assert!(text.contains("build a counter"));

        // Determinism: the payload is a pure function of (prompt, system).
        assert_eq!(
            text,
            TokenRouter::synthetic_response("build a counter", "you are a code generator"),
            "synthetic provider output must be deterministic"
        );
    }

    /// FALSIFICATION — ambient credentials must not be able to change the
    /// synthetic provider's behaviour.
    ///
    /// Three classes of value are injected into the process environment:
    ///   * a placeholder copied from `.env.example` (`YOUR_OPENROUTER_API_KEY`),
    ///   * non-empty garbage,
    ///   * a syntactically valid OpenRouter key.
    ///
    /// Under the old design any one of these made `offline_fast` false and
    /// pushed the router onto the live network path. Under the repaired design
    /// the payload and the egress count must be identical in all three cases.
    #[tokio::test]
    async fn synthetic_offline_is_immune_to_ambient_api_keys() {
        // Env mutation is process-global; serialise against the sibling test.
        let _guard = DISPATCH_MUTEX.lock().await;

        const CASES: [&str; 3] = [
            "YOUR_OPENROUTER_API_KEY",
            "some-non-empty-garbage",
            // Credential-shaped filler: long, constant, and obviously not a real
            // key. It must be non-empty and distinct from the other cases, but
            // deliberately carries no provider prefix and no hex body, so the
            // repository never contains a string matching secret-scanner or
            // push-protection patterns. The router treats any non-empty ambient
            // value identically (that is the point of this test), so the filler
            // loses nothing.
            "placeholder-credential-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        ];

        let expected = TokenRouter::synthetic_response("build a counter", "sys");

        // Save the ambient values once and restore them at the end. Removing
        // them instead would leak state into sibling tests: this environment
        // ships `OPENROUTER_API_KEY=YOUR_OPENROUTER_API_KEY`, and clearing it
        // would change the behaviour of anything that dispatches with the
        // default provider chain.
        const VARS: [&str; 3] = [
            "OPENROUTER_API_KEY",
            "ANTHROPIC_API_KEY",
            "DEEPSEEK_API_KEY",
        ];
        let saved: Vec<(&str, Option<String>)> =
            VARS.iter().map(|k| (*k, std::env::var(k).ok())).collect();

        for case in CASES {
            for k in VARS {
                std::env::set_var(k, case);
            }

            let router = synthetic_router();
            let egress_before = NETWORK_EGRESS_COUNT.with(std::cell::Cell::get);
            let text = router
                .dispatch_prompt("build a counter", "sys")
                .await
                .unwrap();
            let egress_after = NETWORK_EGRESS_COUNT.with(std::cell::Cell::get);

            assert_eq!(
                egress_after, egress_before,
                "ambient key {case:?} must not cause network egress"
            );
            assert_eq!(
                text, expected,
                "ambient key {case:?} must not change the synthetic payload"
            );
        }

        for (k, v) in saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }

    /// FALSIFICATION — a configured credential (not just an env var) must not
    /// be able to reach the synthetic provider, and `api_key()` must return
    /// `None` for it even when every other provider has a key configured.
    #[test]
    fn synthetic_offline_has_no_api_key_and_no_endpoint() {
        let mut cfg = RouterConfig::default();
        cfg.api_keys.insert(
            "synthetic-offline".to_string(),
            "sk-should-be-ignored".to_string(),
        );
        cfg.base_url_overrides.insert(
            "synthetic-offline".to_string(),
            "https://should-never-be-contacted.example".to_string(),
        );

        assert_eq!(
            cfg.api_key(&ModelProvider::SyntheticOffline),
            None,
            "SyntheticOffline must never resolve an API key"
        );
        assert_eq!(
            ModelProvider::SyntheticOffline.default_base_url(),
            "",
            "SyntheticOffline must have no endpoint"
        );
        assert_eq!(
            ModelProvider::SyntheticOffline.completions_path(),
            "",
            "SyntheticOffline must have no request path"
        );
        assert_eq!(
            ModelProvider::SyntheticOffline.to_string(),
            "synthetic-offline"
        );
    }

    /// FALSIFICATION — a *different* provider with no credentials still
    /// degrades to synthetic, i.e. the explicit provider did not accidentally
    /// become the only way to stay offline. This preserves the legacy
    /// behaviour the default chain relied on.
    #[tokio::test]
    async fn non_synthetic_provider_without_keys_still_degrades_offline() {
        // Serialised: this test depends on the *absence* of ambient credentials,
        // and a sibling test deliberately sets them.
        let _guard = DISPATCH_MUTEX.lock().await;

        let saved: Vec<(&str, Option<String>)> = ["OPENROUTER_API_KEY", "ANTHROPIC_API_KEY"]
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect();
        for (k, _) in &saved {
            std::env::remove_var(k);
        }

        let cfg = RouterConfig {
            primary_provider: ModelProvider::OpenRouter,
            fallback_provider: ModelProvider::Anthropic,
            ..RouterConfig::default()
        };
        let router = TokenRouter::without_vector_cache(cfg).unwrap();
        // Both providers require keys and none are configured -> the pre-existing
        // fast path degrades to synthetic, again without touching the network.
        let egress_before = NETWORK_EGRESS_COUNT.with(std::cell::Cell::get);
        let text = router.dispatch_prompt("hello", "sys").await.unwrap();
        let egress_after = NETWORK_EGRESS_COUNT.with(std::cell::Cell::get);

        for (k, v) in saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }

        assert_eq!(
            egress_after, egress_before,
            "credential-free chain must not attempt egress"
        );
        assert_eq!(text, TokenRouter::synthetic_response("hello", "sys"));
    }

    /// CHAIN-CONTINUATION regression (AGENT-JOURNEY fix): a stale primary
    /// credential that is present-but-invalid (401-class failure) plus an
    /// unreachable fallback must continue down the configured provider chain
    /// instead of killing the run — the chain's SyntheticOffline entry is the
    /// honest final resort.
    #[tokio::test]
    async fn chain_continuation_reaches_synthetic_offline_after_stale_credential_and_dead_fallback()
    {
        // Serialised: mutates env + dispatch concurrent state, and the
        // process-global egress spy must attribute its delta to this test.
        let _guard = DISPATCH_MUTEX.lock().await;

        // Hermetic scorecard + vector-cache isolation (same recipe as the
        // scorecard test above).
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("ZYLCODE_SCORECARD_PATH", dir.path().join("sc.db"));

        // Primary OpenRouter with a credential-shaped ambient value so the
        // pre-existing `offline_fast` short-circuit cannot fire — the exact
        // environment shape that used to hard-fail the run. Both chain
        // roots; fallback is Ollama; remaining entries are walked next —
        // SyntheticOffline last, which must complete honestly.
        let mut cfg = RouterConfig {
            primary_provider: ModelProvider::OpenRouter,
            fallback_provider: ModelProvider::LocalOllama,
            ..RouterConfig::default()
        };
        cfg.base_url_overrides.insert(
            ModelProvider::OpenRouter.to_string(),
            "http://127.0.0.1:59999".to_string(),
        );
        // Anthropic must never be walked in a test with egress permission —
        // disable it so the chain path ends at the SyntheticOffline entry.
        for entry in cfg.provider_configs.iter_mut() {
            if entry.kind == crate::router::ProviderKind::Anthropic {
                entry.enabled = false;
            }
        }
        let router = TokenRouter::without_vector_cache(cfg).unwrap();

        let text = router
            .dispatch_prompt("read the README", "sys")
            .await
            .unwrap();
        std::env::remove_var("ZYLCODE_SCORECARD_PATH");

        assert_eq!(
            text,
            TokenRouter::synthetic_response("read the README", "sys"),
            "an unreachable provider chain must complete via the SyntheticOffline chain entry, not fail the run"
        );
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
        let _guard = DISPATCH_MUTEX.lock().await;
        // This test is about the speculative cache, not about any provider.
        // It previously used `RouterConfig::default()`, which egressed to
        // openrouter.ai and localhost:11434 on every run and depended on both
        // failing. The explicit synthetic provider makes the test a pure
        // function of the cache under test.
        let cfg = RouterConfig {
            primary_provider: ModelProvider::SyntheticOffline,
            fallback_provider: ModelProvider::SyntheticOffline,
            ..RouterConfig::default()
        };
        let cache =
            std::sync::Arc::new(SpeculativeCache::new(8, std::time::Duration::from_secs(60)));
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
        for provider in [
            ProviderKind::Anthropic,
            ProviderKind::OpenRouter,
            ProviderKind::Ollama,
            ProviderKind::SyntheticOffline,
        ] {
            assert!((estimate_cost(&provider, 0, 0)).abs() < 1e-9);
        }
    }

    // ------------------------------------------------------------------------
    // D1 — cross-prompt vector-cache contamination guard
    // ------------------------------------------------------------------------
    // Defect: `find_similar(&emb, 0.88)` served a *different* prompt's cached
    // response within one turn. Forensics on `./vector_cache.db` showed every
    // hit carrying one prompt_hash (a single 1778-char plan row) while the
    // plan prompt itself was called once. The guard accepts a similarity hit
    // only when `prompt_hash(current prompt) == hit.prompt_hash`.
    //
    // `ModelProvider::SyntheticOffline` short-circuits BEFORE the vector-cache
    // branch, so these cases use the keyless OpenRouter/Anthropic chain — the
    // same hermetic pattern as
    // `non_synthetic_provider_without_keys_still_degrades_offline`: dispatch
    // reaches the vector-cache branch, then degrades to the offline synthetic
    // response. Zero network egress is asserted at the end.
    #[tokio::test]
    async fn d1_vector_cache_cross_prompt_contamination_guard() {
        // Env mutation is process-global; serialise against sibling tests.
        let _guard = DISPATCH_MUTEX.lock().await;

        const VARS: [&str; 2] = ["OPENROUTER_API_KEY", "ANTHROPIC_API_KEY"];
        let saved: Vec<(&str, Option<String>)> =
            VARS.iter().map(|k| (*k, std::env::var(k).ok())).collect();
        for (k, _) in &saved {
            std::env::remove_var(k);
        }

        let egress_before = NETWORK_EGRESS_COUNT.with(std::cell::Cell::get);
        let tmp = |tag: &str| {
            std::env::temp_dir().join(format!(
                "zylcode_d1_guard_{}_{}.db",
                std::process::id(),
                tag
            ))
        };
        let cfg = RouterConfig {
            primary_provider: ModelProvider::OpenRouter,
            fallback_provider: ModelProvider::Anthropic, // neither is Ollama
            ..RouterConfig::default()                    // no configured keys
        };
        let build = |store: Option<VectorCacheStore>| {
            let router = TokenRouter::without_vector_cache(cfg.clone()).unwrap();
            match store {
                Some(s) => router.with_vector_cache(s),
                None => router,
            }
        };

        let prompt_a = "increment the counter and report the new value";
        let prompt_b = "increment the counter and report the new value in detail";
        let prompt_c = "sum the integers from one to ten";
        let sentinel = "D1_SENTINEL_RESPONSE_BELONGING_TO_A_DIFFERENT_PROMPT";
        let cached_a = "D1_CASE1_CACHED_RESPONSE_FOR_PROMPT_A";
        let sys = "test-system";
        let emb_a = mock_embed(prompt_a, 32);

        // Controls: identical config, no vector cache => pure provider path.
        let control_a = build(None).dispatch_prompt(prompt_a, sys).await.unwrap();
        let control_c = build(None).dispatch_prompt(prompt_c, sys).await.unwrap();

        // Case 1 — same prompt + matching hash permits legitimate reuse.
        let store_1 = VectorCacheStore::new(tmp("a")).unwrap();
        store_1.insert_entry(prompt_a, cached_a, &emb_a).unwrap();
        let router_1 = build(Some(store_1));
        let first = router_1.dispatch_prompt(prompt_a, sys).await.unwrap();
        let second = router_1.dispatch_prompt(prompt_a, sys).await.unwrap();

        // Cases 2/3 — seed prompt_b's response under prompt_a's embedding, so
        // similarity is 1.0 (>= 0.88): a threshold-only lookup WOULD serve
        // prompt_b's answer for prompt_a. Capture the raw lookup first.
        let store_2 = VectorCacheStore::new(tmp("b")).unwrap();
        store_2.insert_entry(prompt_b, sentinel, &emb_a).unwrap();
        let hit = store_2
            .find_similar(&emb_a, 0.88)
            .unwrap()
            .expect("precondition: the seeded row must be reachable by similarity");
        let hit_response = hit.response_text.clone();
        let hit_hash = hit.prompt_hash.clone();
        let router_2 = build(Some(store_2));
        let got = router_2.dispatch_prompt(prompt_a, sys).await.unwrap();

        // Case 4 — within-turn sequence against a freshly poisoned router.
        let store_4 = VectorCacheStore::new(tmp("d")).unwrap();
        store_4.insert_entry(prompt_b, sentinel, &emb_a).unwrap();
        let router_4 = build(Some(store_4));
        let d1 = router_4.dispatch_prompt(prompt_a, sys).await.unwrap();
        let d2 = router_4.dispatch_prompt(prompt_a, sys).await.unwrap();
        let d3 = router_4.dispatch_prompt(prompt_c, sys).await.unwrap();

        let egress_after = NETWORK_EGRESS_COUNT.with(std::cell::Cell::get);

        // Restore ambient credentials BEFORE asserting: a failed assertion must
        // not leak cleared env vars into sibling tests. Remove temp DBs too.
        for (k, v) in saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
        for tag in ["a", "b", "d"] {
            let _ = std::fs::remove_file(tmp(tag));
        }

        assert_eq!(
            egress_after, egress_before,
            "the D1 guard test must not perform network egress"
        );

        // Preconditions — without the guard, the raw similarity lookup serves
        // another prompt's response. That is the defect this test pins down.
        assert_eq!(
            hit_response, sentinel,
            "precondition: similarity lookup returns the seeded wrong-prompt response"
        );
        assert_ne!(
            hit_hash,
            crate::cache::prompt_hash(prompt_a),
            "precondition: the hit carries a different prompt hash"
        );
        assert_eq!(
            hit_hash,
            crate::cache::prompt_hash(prompt_b),
            "precondition: the stored hash belongs to prompt_b"
        );

        // Case 1: same prompt + matching hash permits legitimate reuse.
        assert_eq!(
            first, cached_a,
            "Case 1: matching hash must reuse the stored response"
        );
        assert_eq!(
            second, first,
            "Case 1: repeat dispatch of the same prompt is stable"
        );

        // Case 2: different prompt + high similarity + different hash rejected.
        assert_ne!(
            got, sentinel,
            "Case 2: a similar-but-different prompt's cached response must be rejected"
        );

        // Case 3: rejection falls through to the normal provider path.
        assert!(
            !got.is_empty(),
            "Case 3: fall-through response must be non-empty"
        );
        assert_eq!(
            got, control_a,
            "Case 3: rejected hit must fall through to the provider path"
        );

        // Case 4: no within-turn cross-prompt contamination.
        assert_eq!(
            d1, control_a,
            "Case 4: first prompt answered from the provider path"
        );
        assert_eq!(d2, d1, "Case 4: repeat dispatch within the turn is stable");
        assert_eq!(
            d3, control_c,
            "Case 4: a second distinct prompt gets its own answer"
        );
        assert_ne!(d1, d3, "Case 4: responses stay prompt-specific (no bleed)");
        assert!(
            d1 != sentinel && d3 != sentinel,
            "Case 4: the seeded sentinel never leaks"
        );
    }

    // -----------------------------------------------------------------------
    // Provider probe
    // -----------------------------------------------------------------------

    /// A one-shot HTTP server on a real loopback socket.
    ///
    /// The probe is evidence only if a real request crosses a real socket, so
    /// the tests below answer it themselves instead of mocking the transport:
    /// a canned response is written, the probe runs, and the classification is
    /// asserted. `status_line` is the full status line (`HTTP/1.1 401 Unauthorized`).
    fn serve_once(status_line: &'static str, body: &'static str) -> (String, u16) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let port = listener.local_addr().expect("local addr").port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        (format!("http://127.0.0.1:{port}"), port)
    }

    #[test]
    fn probe_error_classification_is_actionable() {
        let cases = [
            (
                "provider ollama returned 401 Unauthorized: {\"error\":\"bad key\"} [status=401]",
                ProbeStatus::AuthenticationFailed,
                Some(401),
            ),
            (
                "provider ollama returned 404 Not Found: {\"error\":\"no route\"} [status=404]",
                ProbeStatus::InvalidEndpoint,
                Some(404),
            ),
            (
                "provider ollama returned 404 Not Found: {\"error\":{\"message\":\"model 'x' not found\"}} [status=404]",
                ProbeStatus::ModelUnavailable,
                Some(404),
            ),
            (
                "provider ollama returned 400 Bad Request: {\"error\":\"model is required\"} [status=400]",
                ProbeStatus::ModelUnavailable,
                Some(400),
            ),
            (
                "provider ollama returned 429 Too Many Requests: slow down [status=429]",
                ProbeStatus::ProviderRejected,
                Some(429),
            ),
            (
                "provider ollama returned 503 Service Unavailable: down [status=503]",
                ProbeStatus::ProviderRejected,
                Some(503),
            ),
            (
                "HTTP request failed: builder error for url",
                ProbeStatus::NetworkError,
                None,
            ),
        ];
        for (message, expected, code) in cases {
            let (status, http) = classify_probe_error(message);
            assert_eq!(status, expected, "wrong classification for {message:?}");
            assert_eq!(http, code, "wrong HTTP status for {message:?}");
        }
    }

    #[tokio::test]
    async fn probe_reports_connected_against_a_real_endpoint() {
        let _guard = DISPATCH_MUTEX.lock().await;
        let (base, _port) = serve_once(
            "HTTP/1.1 200 OK",
            r#"{"message":{"content":"OK"},"prompt_eval_count":3,"eval_count":1}"#,
        );
        let probe =
            probe_provider(&ProviderKind::Ollama, Some(&base), Some("llama3.1"), 5_000).await;
        assert_eq!(
            probe.status,
            ProbeStatus::Connected,
            "detail: {}",
            probe.detail
        );
        assert!(probe.ok());
        assert_eq!(probe.key_source, "not_required");
        assert_eq!(probe.response_chars, 2);
        assert!(probe.endpoint.ends_with("/api/chat"), "{}", probe.endpoint);
    }

    #[tokio::test]
    async fn probe_classifies_a_rejected_credential() {
        let _guard = DISPATCH_MUTEX.lock().await;
        let (base, _port) = serve_once(
            "HTTP/1.1 401 Unauthorized",
            r#"{"error":{"message":"invalid x-api-key"}}"#,
        );
        let probe =
            probe_provider(&ProviderKind::Ollama, Some(&base), Some("llama3.1"), 5_000).await;
        assert_eq!(probe.status, ProbeStatus::AuthenticationFailed);
        assert_eq!(probe.http_status, Some(401));
        assert!(!probe.ok());
        assert!(probe.detail.contains("invalid"), "{}", probe.detail);
    }

    #[tokio::test]
    async fn probe_classifies_an_unavailable_model() {
        let _guard = DISPATCH_MUTEX.lock().await;
        let (base, _port) = serve_once(
            "HTTP/1.1 404 Not Found",
            r#"{"error":{"message":"model 'nope' not found"}}"#,
        );
        let probe = probe_provider(&ProviderKind::Ollama, Some(&base), Some("nope"), 5_000).await;
        assert_eq!(probe.status, ProbeStatus::ModelUnavailable);
        assert_eq!(probe.http_status, Some(404));
    }

    #[tokio::test]
    async fn probe_classifies_a_refused_connection_as_network_error() {
        let _guard = DISPATCH_MUTEX.lock().await;
        // Bind then drop, so the port is closed but not reused by the test.
        let base = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
            let port = l.local_addr().expect("addr").port();
            drop(l);
            format!("http://127.0.0.1:{port}")
        };
        let probe =
            probe_provider(&ProviderKind::Ollama, Some(&base), Some("llama3.1"), 2_000).await;
        assert_eq!(
            probe.status,
            ProbeStatus::NetworkError,
            "detail: {}",
            probe.detail
        );
        assert_eq!(probe.http_status, None);
    }

    /// A provider that requires a credential and has none is not probed: the
    /// missing key is the whole answer, and sending an unauthenticated request
    /// would only spend the provider's rate limit to learn nothing.
    #[tokio::test]
    async fn probe_without_a_credential_sends_nothing() {
        let _guard = DISPATCH_MUTEX.lock().await;
        let _env = EnvScope::capture(&["ANTHROPIC_API_KEY"]);
        std::env::remove_var("ANTHROPIC_API_KEY");
        NETWORK_EGRESS_COUNT.with(|c| c.set(0));

        let probe = probe_provider(&ProviderKind::Anthropic, None, None, 5_000).await;
        assert_eq!(probe.status, ProbeStatus::ConfigurationIncomplete);
        assert_eq!(probe.key_source, "none");
        assert_eq!(
            NETWORK_EGRESS_COUNT.with(|c| c.get()),
            0,
            "an incomplete configuration must not egress"
        );
    }

    #[tokio::test]
    async fn probe_of_the_offline_provider_sends_nothing() {
        let _guard = DISPATCH_MUTEX.lock().await;
        NETWORK_EGRESS_COUNT.with(|c| c.set(0));
        let probe = probe_provider(&ProviderKind::SyntheticOffline, None, None, 5_000).await;
        assert_eq!(probe.status, ProbeStatus::OfflineProvider);
        assert!(probe.ok());
        assert_eq!(NETWORK_EGRESS_COUNT.with(|c| c.get()), 0);
    }

    // -----------------------------------------------------------------------
    // Provider chain write-through: settings surface -> dispatch
    // -----------------------------------------------------------------------

    fn chain_config() -> RouterConfig {
        RouterConfig {
            provider_configs: vec![
                ProviderConfig {
                    kind: ProviderKind::Anthropic,
                    model: "claude-sonnet-4-5".to_string(),
                    endpoint: String::new(),
                    timeout_ms: 60_000,
                    enabled: true,
                    fallback_order: 0,
                    requires_api_key: true,
                },
                ProviderConfig {
                    kind: ProviderKind::Ollama,
                    model: "llama3.1".to_string(),
                    endpoint: String::new(),
                    timeout_ms: 60_000,
                    enabled: true,
                    fallback_order: 1,
                    requires_api_key: false,
                },
                ProviderConfig {
                    kind: ProviderKind::OpenAi,
                    model: "gpt-4o-mini".to_string(),
                    endpoint: String::new(),
                    timeout_ms: 60_000,
                    enabled: false,
                    fallback_order: 2,
                    requires_api_key: true,
                },
            ],
            ..RouterConfig::default()
        }
    }

    #[test]
    fn apply_provider_chain_makes_the_chain_authoritative() {
        let mut cfg = chain_config();
        // The user drags Ollama to the top: the legacy pair must follow, because
        // that pair is what dispatch actually attempts first.
        cfg.provider_configs[0].fallback_order = 5;
        cfg.provider_configs[1].fallback_order = 0;
        cfg.apply_provider_chain();

        assert_eq!(cfg.primary_provider, ModelProvider::LocalOllama);
        assert_eq!(cfg.primary_model, "llama3.1");
        assert_eq!(cfg.fallback_provider, ModelProvider::Anthropic);
        assert_eq!(
            cfg.provider_configs[0].kind,
            ProviderKind::Ollama,
            "entries are sorted by fallback_order"
        );
    }

    #[test]
    fn apply_provider_chain_skips_disabled_and_blank_entries() {
        let mut cfg = chain_config();
        cfg.provider_configs[1].enabled = false; // Ollama off
        cfg.provider_configs[0].model = "   ".to_string(); // blank model
        cfg.apply_provider_chain();

        // Nothing dispatchable remains, so the legacy pair must be untouched
        // rather than becoming a blank provider.
        assert_ne!(cfg.primary_model.trim(), "");
        assert_ne!(cfg.primary_provider, ModelProvider::LocalOllama);
    }

    #[test]
    fn endpoint_override_is_normalised_and_cleared() {
        let mut cfg = chain_config();
        cfg.provider_configs[0].endpoint =
            "  https://gateway.internal/anthropic/v1/messages/  ".to_string();
        cfg.apply_provider_chain();
        assert_eq!(
            cfg.base_url(&ModelProvider::Anthropic),
            "https://gateway.internal/anthropic",
            "a full completions URL must not have its path appended twice"
        );

        cfg.provider_configs[0].endpoint = String::new();
        cfg.apply_provider_chain();
        assert_eq!(
            cfg.base_url(&ModelProvider::Anthropic),
            "https://api.anthropic.com",
            "clearing the endpoint restores the provider default"
        );
    }

    #[test]
    fn per_provider_timeout_overrides_the_router_default() {
        let mut cfg = chain_config();
        cfg.timeout_ms = 60_000;
        cfg.provider_configs[1].timeout_ms = 7_500;
        assert_eq!(cfg.provider_timeout_ms(&ModelProvider::LocalOllama), 7_500);
        // A provider with no chain entry keeps the router-wide timeout.
        assert_eq!(cfg.provider_timeout_ms(&ModelProvider::DeepSeek), 60_000);
    }

    #[test]
    fn default_chain_uses_native_model_ids() {
        let cfg = RouterConfig::default();
        let model = |kind: &ProviderKind| {
            cfg.provider_configs
                .iter()
                .find(|c| &c.kind == kind)
                .map(|c| c.model.clone())
                .unwrap_or_default()
        };
        // The native Anthropic API does not accept OpenRouter-style slugs.
        assert!(
            !model(&ProviderKind::Anthropic).starts_with("anthropic/"),
            "{}",
            model(&ProviderKind::Anthropic)
        );
        assert!(model(&ProviderKind::Anthropic).contains("claude"));
        // OpenRouter addresses models as `vendor/model`.
        assert!(model(&ProviderKind::OpenRouter).contains('/'));
        assert!(!model(&ProviderKind::OpenRouter).starts_with("openrouter/"));
    }

    #[test]
    fn a_cloned_router_shares_the_live_configuration() {
        // The agent dispatches through a clone taken at construction; a copied
        // config field would let the settings surface edit state nothing reads.
        let router = TokenRouter::without_vector_cache(chain_config()).expect("router");
        let agent_side = router.clone();
        router.update_config(|c| c.provider_configs[0].model = "edited-model".to_string());
        assert_eq!(agent_side.config().primary_model, "edited-model");
    }

    #[tokio::test]
    async fn set_config_reaches_the_next_request() {
        let _guard = DISPATCH_MUTEX.lock().await;
        let (base, _port) = serve_once("HTTP/1.1 200 OK", r#"{"message":{"content":"pong"}}"#);
        let cfg = RouterConfig {
            provider_configs: vec![ProviderConfig {
                kind: ProviderKind::Ollama,
                model: "before-edit".to_string(),
                endpoint: base.clone(),
                timeout_ms: 5_000,
                enabled: true,
                fallback_order: 0,
                requires_api_key: false,
            }],
            ..RouterConfig::default()
        };
        let router = TokenRouter::without_vector_cache(RouterConfig::default()).expect("router");
        // Installing a chain is what the settings command does; construction
        // alone must not normalise, or an env-selected provider would be
        // overridden by the default list.
        router.set_config(cfg);
        assert_eq!(router.config().primary_model, "before-edit");

        // The same call the settings command makes.
        router.update_config(|c| c.provider_configs[0].model = "after-edit".to_string());

        let probe = router.probe_saved(&ProviderKind::Ollama).await;
        assert_eq!(
            probe.model, "after-edit",
            "the probe must test the saved configuration, not a snapshot"
        );
        assert_eq!(probe.status, ProbeStatus::Connected);
        assert_eq!(probe.endpoint, format!("{base}/api/chat"));
    }
}
