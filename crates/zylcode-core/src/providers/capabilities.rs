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
//! `tools` means "this client can declare a tool and translate the provider's
//! call signal" — an implementation claim. It is deliberately *not* a runtime
//! claim, because an implementation that has never talked to the real endpoint is
//! not a verified capability. The second half of this module reports how far each
//! stage of that lifecycle has actually been proven, per provider
//! ([`tool_call_evidence`]), using [`CapabilityEvidence`]. Read `tools` together
//! with that row, never alone.
//!
//! This module is the single source of truth for both tables.

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
    /// **Implementation claim, not a runtime claim.** It says a request builder
    /// emits the declaration and a decoder reads the call back; it says nothing
    /// about whether a real endpoint ever completed a turn. Use
    /// [`tool_call_evidence`] for that, and report the two together.
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

/// How far one capability claim has actually been proven.
///
/// The ladder is ordered: a later variant asserts everything an earlier one
/// does. `LiveVerified` and `DesktopVerified` are *both* reachable only from a
/// recorded run — the first against the provider's real endpoint, the second
/// through the desktop application's own IPC and window. Neither can be set from
/// config, a model name, or a provider's public documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEvidence {
    /// No code path for this stage at all.
    NotImplemented,
    /// A code path exists and compiles, but no automated test exercises it.
    ImplementedOnly,
    /// Automated tests exercise the stage (synthetic frames, scripted turns,
    /// loopback HTTP). This is the ceiling for a stage nobody has run live.
    UnitTested,
    /// Exercised against the provider's real API, with the run recorded.
    LiveVerified,
    /// Exercised through the desktop application: Tauri IPC, the real command,
    /// the real window. A session-level test with a scripted provider does *not*
    /// reach this rung.
    DesktopVerified,
}

impl CapabilityEvidence {
    pub const fn is_proven(self) -> bool {
        matches!(
            self,
            CapabilityEvidence::UnitTested
                | CapabilityEvidence::LiveVerified
                | CapabilityEvidence::DesktopVerified
        )
    }

    /// Label used in reports and documentation, so prose and code cannot drift.
    pub const fn label(self) -> &'static str {
        match self {
            CapabilityEvidence::NotImplemented => "NOT_IMPLEMENTED",
            CapabilityEvidence::ImplementedOnly => "IMPLEMENTED_ONLY",
            CapabilityEvidence::UnitTested => "UNIT_TESTED",
            CapabilityEvidence::LiveVerified => "LIVE_VERIFIED",
            CapabilityEvidence::DesktopVerified => "DESKTOP_VERIFIED",
        }
    }
}

/// Evidence for each stage of the native tool-calling lifecycle, per provider.
///
/// The stages are the ones a caller can observe: the request declares the tool,
/// the response decodes the call, the loop resolves the wire name to a canonical
/// id, the gate decides, the executor runs, the ledger records, the result goes
/// back under the same call id, and the model continues.
///
/// Splitting them this way is the point: `declaration` being `UnitTested` says
/// nothing about whether anyone ever saw this provider complete a round trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallEvidence {
    /// The request body carries the provider's tool declaration.
    pub declaration: CapabilityEvidence,
    /// A complete (non-streaming) response decodes into a neutral `ToolCall`.
    pub decoding: CapabilityEvidence,
    /// Tool calls split across stream frames are reassembled.
    pub streaming_decoding: CapabilityEvidence,
    /// Wire names resolve to canonical ids; undeclared names are refused.
    pub resolution: CapabilityEvidence,
    /// A resolved call reaches the registry executor and performs its effect.
    pub execution: CapabilityEvidence,
    /// The permission gate decides before execution, and fails closed.
    pub gate: CapabilityEvidence,
    /// The invocation is recorded in the evidence ledger.
    pub evidence: CapabilityEvidence,
    /// The tool result returns to the model under the original call id.
    pub result_return: CapabilityEvidence,
    /// The loop issues a continuation round that sees the result.
    pub continuation: CapabilityEvidence,
    /// A whole turn ran end-to-end over this provider's own transport.
    pub wire_end_to_end: CapabilityEvidence,
    /// A whole turn ran through the desktop command and window.
    pub desktop_surface: CapabilityEvidence,
}

impl ToolCallEvidence {
    /// Shared-loop stages: the code is provider-independent, so its evidence is
    /// the same for every provider that can hand the loop a tool call. Kept as
    /// one constructor so a provider cannot quietly claim a different level.
    const SHARED_LOOP: ToolCallEvidence = ToolCallEvidence {
        declaration: CapabilityEvidence::NotImplemented,
        decoding: CapabilityEvidence::NotImplemented,
        streaming_decoding: CapabilityEvidence::NotImplemented,
        resolution: CapabilityEvidence::UnitTested,
        execution: CapabilityEvidence::UnitTested,
        gate: CapabilityEvidence::UnitTested,
        evidence: CapabilityEvidence::UnitTested,
        result_return: CapabilityEvidence::UnitTested,
        continuation: CapabilityEvidence::UnitTested,
        wire_end_to_end: CapabilityEvidence::ImplementedOnly,
        desktop_surface: CapabilityEvidence::UnitTested,
    };
}

/// The tool-calling evidence row for one provider.
///
/// Every provider that is wired to a tool protocol is `UnitTested` at the three
/// wire stages, because each module has a body-shape test, a completion-decoding
/// test and a stream-reassembly test against synthetic frames. None of them is
/// `LiveVerified`: no recorded run against a real endpoint exists in this
/// repository, so `wire_end_to_end` stays `ImplementedOnly` and
/// `desktop_surface` stays at the session-test ceiling rather than claiming the
/// window. Change these only together with the recording that proves it.
pub fn tool_call_evidence(provider: &ModelProvider) -> ToolCallEvidence {
    match provider {
        ModelProvider::SyntheticOffline => ToolCallEvidence {
            declaration: CapabilityEvidence::NotImplemented,
            decoding: CapabilityEvidence::NotImplemented,
            streaming_decoding: CapabilityEvidence::NotImplemented,
            resolution: CapabilityEvidence::NotImplemented,
            execution: CapabilityEvidence::NotImplemented,
            gate: CapabilityEvidence::NotImplemented,
            evidence: CapabilityEvidence::NotImplemented,
            result_return: CapabilityEvidence::NotImplemented,
            continuation: CapabilityEvidence::NotImplemented,
            wire_end_to_end: CapabilityEvidence::NotImplemented,
            desktop_surface: CapabilityEvidence::NotImplemented,
        },
        ModelProvider::Anthropic
        | ModelProvider::OpenAi
        | ModelProvider::OpenRouter
        | ModelProvider::DeepSeek
        | ModelProvider::Respan
        | ModelProvider::OpenAiCompatible
        | ModelProvider::LocalOllama => ToolCallEvidence {
            declaration: CapabilityEvidence::UnitTested,
            decoding: CapabilityEvidence::UnitTested,
            streaming_decoding: CapabilityEvidence::UnitTested,
            ..ToolCallEvidence::SHARED_LOOP
        },
    }
}

/// Static capability set for each dispatcher-facing model provider.
///
/// `tools` is `true` exactly where a provider module both sends the provider's
/// own tool declaration and translates its tool-call signal back into the
/// neutral contract: the Anthropic Messages module, the OpenAI-compatible
/// module (used by OpenAI, OpenRouter, DeepSeek, Respan and any compatible
/// endpoint) and the Ollama module. `SyntheticOffline` stays `false`: it has
/// no wire and cannot ask for anything. The coverage is asserted in
/// `tool_calling_tests`, so this table cannot drift ahead of the parsers.
///
/// `max_client_tokens` is `Some(4096)` wherever the request builder actually sends
/// `max_tokens`/`num_predict`, which is every provider except the offline path.
pub fn provider_capabilities(provider: &ModelProvider) -> ProviderCapabilitySet {
    match provider {
        ModelProvider::Anthropic => ProviderCapabilitySet {
            streaming: true,
            tools: true,
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
            tools: true,
            vision: false,
            logprobs: false,
            usage: true,
            max_client_tokens: Some(4096),
        },
        ModelProvider::LocalOllama => ProviderCapabilitySet {
            streaming: true,
            tools: true,
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

/// Providers with a *recorded* run against their real endpoint.
///
/// Empty on purpose. Adding a provider here is the act of claiming it, and the
/// test below fails if a row is promoted without an entry — the mechanism that
/// stops "implemented" from drifting into "verified" in a later refactor.
pub const LIVE_RUNS_RECORDED: &[&str] = &[];

/// Providers exercised through the desktop application's own surface.
///
/// Also empty: the desktop session test uses a scripted provider, which proves
/// the wiring and the lifecycle, not the window.
pub const DESKTOP_RUNS_RECORDED: &[&str] = &[];

#[cfg(test)]
mod evidence_tests {
    use super::*;

    fn all() -> Vec<ModelProvider> {
        vec![
            ModelProvider::OpenAi,
            ModelProvider::OpenAiCompatible,
            ModelProvider::OpenRouter,
            ModelProvider::DeepSeek,
            ModelProvider::Respan,
            ModelProvider::Anthropic,
            ModelProvider::LocalOllama,
            ModelProvider::SyntheticOffline,
        ]
    }

    /// An implementation claim with an untested wire, or a tested wire attached
    /// to a provider that does not claim tools, would both be reporting bugs.
    #[test]
    fn wire_evidence_matches_the_implementation_claim() {
        for provider in all() {
            let caps = provider_capabilities(&provider);
            let evidence = tool_call_evidence(&provider);
            if caps.tools {
                assert!(
                    evidence.declaration.is_proven(),
                    "{provider}: declares tools, so the request-shape test must exist"
                );
                assert!(
                    evidence.decoding.is_proven(),
                    "{provider}: declares tools, so the decoding test must exist"
                );
                assert!(
                    evidence.streaming_decoding.is_proven(),
                    "{provider}: declares tools, so the stream-reassembly test must exist"
                );
            } else {
                assert_eq!(
                    evidence.declaration,
                    CapabilityEvidence::NotImplemented,
                    "{provider}: no tool protocol, so nothing to test"
                );
                assert_eq!(
                    evidence.wire_end_to_end,
                    CapabilityEvidence::NotImplemented,
                    "{provider}: no wire, so no end-to-end run"
                );
            }
        }
    }

    /// The specific correction this module exists for: no provider has been run
    /// against its real endpoint or through the app window, so nothing may say
    /// otherwise.
    #[test]
    fn live_and_desktop_claims_require_a_recorded_run() {
        for provider in all() {
            let evidence = tool_call_evidence(&provider);
            let name = provider.to_string();
            if evidence.wire_end_to_end >= CapabilityEvidence::LiveVerified {
                assert!(
                    LIVE_RUNS_RECORDED.contains(&name.as_str()),
                    "{name}: claims live verification with no recorded run"
                );
            }
            if evidence.desktop_surface >= CapabilityEvidence::DesktopVerified {
                assert!(
                    DESKTOP_RUNS_RECORDED.contains(&name.as_str()),
                    "{name}: claims desktop verification with no recorded run"
                );
            }
            assert!(
                !LIVE_RUNS_RECORDED.contains(&name.as_str())
                    || evidence.wire_end_to_end >= CapabilityEvidence::LiveVerified,
                "{name}: a recorded run exists, so the row understates it"
            );
        }
    }

    /// The shared stages are provider-independent code. A provider cannot be
    /// better than the loop it feeds.
    #[test]
    fn no_provider_outruns_the_shared_loop() {
        for provider in all() {
            let evidence = tool_call_evidence(&provider);
            if evidence.declaration == CapabilityEvidence::NotImplemented {
                continue;
            }
            for stage in [
                evidence.resolution,
                evidence.execution,
                evidence.gate,
                evidence.evidence,
                evidence.result_return,
                evidence.continuation,
            ] {
                assert!(
                    stage.is_proven(),
                    "{provider}: the shared loop is tested, so no stage may be unproven"
                );
            }
        }
    }

    #[test]
    fn labels_are_the_reported_vocabulary() {
        assert_eq!(
            CapabilityEvidence::NotImplemented.label(),
            "NOT_IMPLEMENTED"
        );
        assert_eq!(
            CapabilityEvidence::ImplementedOnly.label(),
            "IMPLEMENTED_ONLY"
        );
        assert_eq!(CapabilityEvidence::UnitTested.label(), "UNIT_TESTED");
        assert_eq!(CapabilityEvidence::LiveVerified.label(), "LIVE_VERIFIED");
        assert_eq!(
            CapabilityEvidence::DesktopVerified.label(),
            "DESKTOP_VERIFIED"
        );
        assert!(CapabilityEvidence::UnitTested > CapabilityEvidence::ImplementedOnly);
        assert!(CapabilityEvidence::DesktopVerified > CapabilityEvidence::LiveVerified);
    }
}
