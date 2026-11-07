//! Static, evidence-bound capability claims for each dispatcher-facing provider.
//!
//! These are a property of *which provider is configured*, not a claim a model
//! fabricates about itself. The router and the agent loop may consult them before
//! dispatching to decide whether streaming, native tool calling, vision, logit
//! bias, or a client-side max_tokens cap are even meaningful for the chosen
//! provider.
//!
//! A flag is `true` only when *this client* actually implements that shape for
//! that provider — not merely when the provider's public API offers it. Two
//! consequences follow, and both are deliberate:
//!
//! - `streaming` is `true` only where a provider module parses that provider's own
//!   increment protocol (Anthropic and OpenAI-compatible SSE, Ollama NDJSON), and
//!   is covered by a loopback test that sees two distinct deltas. `SyntheticOffline`
//!   emits chunked feedback with no wire, so it is `false`.
//! - `vision` and `logprobs` are `false` everywhere: no request builder sends image
//!   blocks and nothing requests or surfaces log probabilities, so claiming them
//!   would make this table describe someone else's client.
//!
//! This module is the single source of truth for that table.

use crate::router::ModelProvider;
use serde::{Deserialize, Serialize};

/// What a provider can actually do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProviderCapabilitySet {
    /// Whether the provider can deliver incremental response deltas over the wire.
    ///
    /// When false, real streaming is not available from this provider and the
    /// dispatcher must label any chunked UI feedback as synthetic/offline-shaped
    /// rather than pretending the provider streamed.
    pub streaming: bool,
    /// Whether the provider has a native tool/function-calling protocol the
    /// provider module is wired to translate.
    ///
    /// This is about the provider's own tool signal, not about ZylCode's existing
    /// in-process tool runtime, which already lives elsewhere and remains the
    /// execution/security boundary.
    pub tools: bool,
    /// Whether this client can send image content blocks to the provider.
    ///
    /// The provider APIs mostly accept them; no request builder here constructs
    /// them, so this stays `false` until one does.
    pub vision: bool,
    /// Whether this client requests and surfaces log probabilities.
    ///
    /// `false` until a request builder sends `logprobs` and the stream contract
    /// carries the result back.
    pub logprobs: bool,
    /// Whether the provider exposes usage (input/output tokens).
    pub usage: bool,
    /// Maximum client-specified max_tokens the provider's chat endpoint honours.
    ///
    /// `None` means "no client-side cap the provider documents as meaningful".
    pub max_client_tokens: Option<u64>,
}

impl Default for ProviderCapabilitySet {
    fn default() -> Self {
        Self {
            streaming: false,
            tools: false,
            vision: false,
            logprobs: false,
            usage: true,
            max_client_tokens: Some(4096),
        }
    }
}

impl ProviderCapabilitySet {
    /// Offline synthetic feedback may still be rendered as chunked for the UI,
    /// but that is an app feedback shape, not evidence that the provider streamed.
    pub fn streaming_offline_fallback(self) -> bool {
        true
    }
}

/// Static capability set for each dispatcher-facing model provider.
///
/// `tools` is `false` everywhere: native provider tool calling is behind an
/// acceptance gate that has not passed, so no provider module builds `tools` into a
/// request. The provider APIs can do it; this client does not yet, and claiming
/// otherwise would make the capability table lie to the router.
///
/// `max_client_tokens` is `Some(4096)` wherever the request builder actually sends
/// `max_tokens`/`num_predict`, which is every provider except the offline path.
pub fn provider_capabilities(provider: &ModelProvider) -> ProviderCapabilitySet {
    match provider {
        ModelProvider::Anthropic => ProviderCapabilitySet {
            streaming: true,
            tools: false,
            vision: false,
            logprobs: false,
            usage: true,
            max_client_tokens: Some(4096),
        },
        ModelProvider::OpenAi
        | ModelProvider::OpenRouter
        | ModelProvider::DeepSeek
        | ModelProvider::Respan
        | ModelProvider::OpenAiCompatible => ProviderCapabilitySet {
            streaming: true,
            tools: false,
            vision: false,
            logprobs: false,
            usage: true,
            max_client_tokens: Some(4096),
        },
        ModelProvider::LocalOllama => ProviderCapabilitySet {
            streaming: true,
            tools: false,
            vision: false,
            logprobs: false,
            usage: true,
            max_client_tokens: Some(4096),
        },
        ModelProvider::SyntheticOffline => ProviderCapabilitySet {
            streaming: false,
            tools: false,
            vision: false,
            logprobs: false,
            usage: false,
            max_client_tokens: None,
        },
    }
}
