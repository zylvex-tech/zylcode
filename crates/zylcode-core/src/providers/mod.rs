//! Provider-neutral surface for model-provider interaction.
//!
//! This crate already has a real HTTP surface via [`crate::router::TokenRouter`].
//! The modules here split provider *protocol* out of that router so the router can
//! stay orchestration-only: selecting which provider module to use, wiring the
//! neutral streaming contract, and mapping provider errors into one shape.
//!
//! Provider-specific request construction, auth headers, SSE/NDJSON parsing, and
//! tool-use block translation live in the provider submodules and are not allowed
//! to leak back into the neutral types.

pub mod anthropic;
pub mod capabilities;
pub mod compat;
pub mod error;
pub mod http;
pub mod ollama;
pub mod openai_compatible;
pub mod respan;
pub mod sse;
pub mod streaming;
pub mod synthetic;

#[cfg(test)]
mod test_server;
#[cfg(test)]
mod tool_calling_tests;
#[cfg(test)]
mod transport_tests;

#[cfg(test)]
mod tests {
    use crate::router::ModelProvider;

    use super::capabilities::{provider_capabilities, ProviderCapabilitySet};

    #[test]
    fn capability_set_is_copyable_and_stable() {
        let a = provider_capabilities(&ModelProvider::Anthropic);
        let b = a;
        assert_eq!(a, b);
        assert!(a.streaming);
        assert!(a.usage);
        // Implemented-feature-only: no request builder sends images or asks for
        // log probabilities, so neither may be claimed.
        assert!(!a.vision);
        assert!(!a.logprobs);
        assert_eq!(a.max_client_tokens, Some(4096));
    }

    #[test]
    fn openai_compatible_capabilities_are_the_openai_family_default() {
        let caps = provider_capabilities(&ModelProvider::OpenAiCompatible);
        assert!(caps.streaming);
        assert!(caps.usage);
        assert!(!caps.vision);
        assert!(!caps.logprobs);
        assert_eq!(caps.max_client_tokens, Some(4096));
    }

    #[test]
    fn ollama_is_streaming_but_not_vision() {
        let caps = provider_capabilities(&ModelProvider::LocalOllama);
        assert!(caps.streaming);
        assert!(!caps.vision);
        assert!(!caps.logprobs);
        assert!(caps.usage);
        assert_eq!(caps.max_client_tokens, Some(4096));
    }

    #[test]
    fn native_tool_calling_is_claimed_only_where_it_is_translated() {
        for provider in [
            ModelProvider::Anthropic,
            ModelProvider::OpenAi,
            ModelProvider::OpenRouter,
            ModelProvider::DeepSeek,
            ModelProvider::Respan,
            ModelProvider::OpenAiCompatible,
            ModelProvider::LocalOllama,
        ] {
            assert!(
                provider_capabilities(&provider).tools,
                "{provider} has a tool declaration and a parser, so it must say so"
            );
        }
        assert!(
            !provider_capabilities(&ModelProvider::SyntheticOffline).tools,
            "the offline path has no wire and cannot call a tool"
        );
    }

    #[test]
    fn synthetic_offline_claims_nothing_real() {
        let caps = provider_capabilities(&ModelProvider::SyntheticOffline);
        assert!(!caps.streaming);
        assert!(!caps.tools);
        assert!(!caps.vision);
        assert!(!caps.logprobs);
        assert!(!caps.usage);
        assert_eq!(caps.max_client_tokens, None);
    }

    /// The table may only claim what the provider modules actually do. Streaming is
    /// the one capability that is genuinely implemented, and it is exactly the set of
    /// providers whose body builders/decoders exist and are covered by loopback
    /// tests; vision, logprobs and tools are not implemented anywhere.
    #[test]
    fn no_capability_is_claimed_without_an_implementation() {
        for provider in [
            ModelProvider::Anthropic,
            ModelProvider::OpenAi,
            ModelProvider::OpenRouter,
            ModelProvider::DeepSeek,
            ModelProvider::Respan,
            ModelProvider::OpenAiCompatible,
            ModelProvider::LocalOllama,
            ModelProvider::SyntheticOffline,
        ] {
            let caps = provider_capabilities(&provider);
            if provider == ModelProvider::SyntheticOffline {
                assert!(!caps.tools, "{provider}: no wire, so no tool calls");
            } else {
                assert!(caps.tools, "{provider}: tool translation is implemented");
            }
            assert!(!caps.vision, "{provider}: no image content is ever sent");
            assert!(!caps.logprobs, "{provider}: no logprobs are requested");
        }

        // Streaming differs per provider rather than being assumed identical: the
        // offline path chunks locally and must not be presented as a provider stream.
        let streaming: Vec<bool> = [
            ModelProvider::Anthropic,
            ModelProvider::OpenAiCompatible,
            ModelProvider::LocalOllama,
            ModelProvider::SyntheticOffline,
        ]
        .iter()
        .map(|p| provider_capabilities(p).streaming)
        .collect();
        assert_eq!(streaming, vec![true, true, true, false]);
    }

    #[test]
    fn every_provider_has_a_capability_set() {
        for provider in [
            ModelProvider::Anthropic,
            ModelProvider::OpenAi,
            ModelProvider::OpenRouter,
            ModelProvider::DeepSeek,
            ModelProvider::Respan,
            ModelProvider::OpenAiCompatible,
            ModelProvider::LocalOllama,
            ModelProvider::SyntheticOffline,
        ] {
            let caps: ProviderCapabilitySet = provider_capabilities(&provider);
            // Only the synthetic path may claim it cannot do real work.
            assert_eq!(caps.streaming, provider != ModelProvider::SyntheticOffline);
        }
    }
}
