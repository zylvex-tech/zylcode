//! Model capability metadata registry (wave: model routing foundation).
//!
//! # Why this exists
//!
//! "The model accepts images" and "the model understands images" are
//! different claims. Provider adapters that accept an attachment prove
//! transport, not interpretation. This registry is the single place where
//! model capabilities are declared, and every capability carries its own
//! verification state — `VISION_UNVERIFIED` is a first-class value, not a
//! footnote.
//!
//! # Rules
//!
//! - Attachment acceptance (`ImageInput: Supported`) never implies vision
//!   interpretation (`VisionInterpretation: VisionUnverified` until a real
//!   image-understanding test is recorded).
//! - Provider availability ≠ local availability; both are tracked.
//! - Capability reports are deterministic (contract tests, no network).
//! - Runtime verification is recorded per model, never assumed from config.

use serde::{Deserialize, Serialize};

/// Verification state of one capability of one model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    /// Capability present and confirmed by a recorded runtime test.
    Verified,
    /// Capability present in config/adapter but never runtime-proven.
    Unverified,
    /// Structurally marked capable (e.g. multimodal model) with no real
    /// image-understanding test recorded. Distinct from Unverified for audit.
    VisionUnverified,
    /// Capability absent for this model.
    Unsupported,
    /// Capability exists but is blocked (quota, auth, missing dependency).
    Blocked,
}

/// A model's full capability row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Model identifier as the provider names it.
    pub model_id: String,
    /// Owning provider (`anthropic`, `openrouter`, `ollama`, …).
    pub provider: String,

    // ---- Input transport (what the harness can SEND) ----
    pub text_input: CapabilityState,
    /// The harness can attach image bytes. **Transport only.**
    pub image_input: CapabilityState,
    pub file_input: CapabilityState,

    // ---- Interpretation (what the model can DO) ----
    /// The model actually understands images. Starts VisionUnverified even
    /// for multimodal models until a recorded test proves it.
    pub vision_interpretation: CapabilityState,
    pub structured_output: CapabilityState,
    pub tool_calling: CapabilityState,
    pub streaming: CapabilityState,

    // ---- Availability ----
    /// Runs on this machine (local inference).
    pub local_available: bool,
    /// Provider-side availability (endpoint exists).
    pub provider_available: bool,
    /// Auth configured (key present)? Recorded, never assumed.
    pub auth_state: AuthState,
    /// Quota state, when known.
    pub quota_state: QuotaState,
    /// A real round-trip through this model has been recorded.
    pub runtime_verified: bool,
}

/// Authentication state for a model's provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthState {
    Configured,
    Missing,
    NotRequired,
}

/// Quota state for a model's provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaState {
    Known,
    Unknown,
    Exhausted,
}

impl ModelCapabilities {
    /// The audit rule this module exists to enforce: accepting images does
    /// not prove understanding them.
    pub fn vision_is_proven(&self) -> bool {
        matches!(self.vision_interpretation, CapabilityState::Verified)
    }
}

/// Build the default capability rows for the providers this repository
/// actually wires (see `RouterConfig::provider_configs`). Deterministic:
/// no network, no clock — safe for contract tests.
pub fn default_registry() -> Vec<ModelCapabilities> {
    vec![
        ModelCapabilities {
            model_id: "claude-sonnet-4-5".into(),
            provider: "anthropic".into(),
            text_input: CapabilityState::Verified,
            // Adapter accepts image attachments; no recorded image-
            // understanding test exists in this repo yet.
            image_input: CapabilityState::Unverified,
            file_input: CapabilityState::Unverified,
            vision_interpretation: CapabilityState::VisionUnverified,
            structured_output: CapabilityState::Unverified,
            tool_calling: CapabilityState::Unverified,
            streaming: CapabilityState::Verified,
            local_available: false,
            provider_available: true,
            auth_state: AuthState::Missing,
            quota_state: QuotaState::Unknown,
            runtime_verified: false,
        },
        ModelCapabilities {
            model_id: "openai/gpt-4o".into(),
            provider: "openrouter".into(),
            text_input: CapabilityState::Verified,
            image_input: CapabilityState::Unverified,
            file_input: CapabilityState::Unverified,
            vision_interpretation: CapabilityState::VisionUnverified,
            structured_output: CapabilityState::Unverified,
            tool_calling: CapabilityState::Unverified,
            streaming: CapabilityState::Verified,
            local_available: false,
            provider_available: true,
            auth_state: AuthState::Missing,
            quota_state: QuotaState::Unknown,
            runtime_verified: false,
        },
        ModelCapabilities {
            model_id: "llama3:latest".into(),
            provider: "ollama".into(),
            text_input: CapabilityState::Verified,
            image_input: CapabilityState::Unsupported,
            file_input: CapabilityState::Unsupported,
            vision_interpretation: CapabilityState::Unsupported,
            structured_output: CapabilityState::Unverified,
            tool_calling: CapabilityState::Unverified,
            streaming: CapabilityState::Verified,
            local_available: true,
            provider_available: true,
            auth_state: AuthState::NotRequired,
            quota_state: QuotaState::Unknown,
            runtime_verified: false,
        },
        ModelCapabilities {
            model_id: "synthetic-offline".into(),
            provider: "zylcode".into(),
            text_input: CapabilityState::Verified,
            image_input: CapabilityState::Unsupported,
            file_input: CapabilityState::Unsupported,
            vision_interpretation: CapabilityState::Unsupported,
            structured_output: CapabilityState::Unsupported,
            tool_calling: CapabilityState::Unsupported,
            streaming: CapabilityState::Unsupported,
            local_available: true,
            provider_available: true,
            auth_state: AuthState::NotRequired,
            quota_state: QuotaState::Known,
            // The synthetic provider is deterministic and exercised by the
            // offline test suite on every run.
            runtime_verified: true,
        },
    ]
}

/// Query: the registry row for one model id.
pub fn lookup(model_id: &str) -> Option<ModelCapabilities> {
    default_registry()
        .into_iter()
        .find(|m| m.model_id == model_id)
}

/// The registry serialized for service/UI surfaces (read-only).
pub fn registry_json() -> serde_json::Value {
    serde_json::json!({
        "contract": "model-capability-metadata/1",
        "note": "image_input is transport only; vision_interpretation requires a recorded image-understanding test",
        "models": default_registry(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Contract: attachment acceptance never implies vision understanding.
    #[test]
    fn image_transport_never_implies_vision_interpretation() {
        for m in default_registry() {
            if matches!(
                m.image_input,
                CapabilityState::Unverified | CapabilityState::Verified
            ) {
                assert!(
                    matches!(
                        m.vision_interpretation,
                        CapabilityState::VisionUnverified | CapabilityState::Verified
                    ),
                    "{} accepts images but its vision state is {:?} — transport must not imply interpretation",
                    m.model_id,
                    m.vision_interpretation
                );
            }
        }
    }

    /// Contract: no model in the default registry claims proven vision.
    #[test]
    fn no_default_model_claims_proven_vision() {
        for m in default_registry() {
            assert!(
                !m.vision_is_proven(),
                "{} claims proven vision with no recorded image test",
                m.model_id
            );
        }
    }

    /// Contract: the registry is deterministic (same output every call).
    #[test]
    fn registry_is_deterministic() {
        let a = serde_json::to_string(&default_registry()).unwrap();
        let b = serde_json::to_string(&default_registry()).unwrap();
        assert_eq!(a, b);
    }

    /// Contract: every provider this repo wires is represented.
    #[test]
    fn registry_covers_wired_providers() {
        let registry = default_registry();
        let providers: Vec<&str> = registry.iter().map(|m| m.provider.as_str()).collect();
        for p in ["anthropic", "openrouter", "ollama"] {
            assert!(
                providers.contains(&p),
                "provider {p} is wired in RouterConfig but missing from the registry"
            );
        }
    }

    /// Contract: local vs provider availability are independently tracked.
    #[test]
    fn local_and_provider_availability_are_independent() {
        let anthropic = lookup("claude-sonnet-4-5").unwrap();
        assert!(!anthropic.local_available && anthropic.provider_available);
        let ollama = lookup("llama3:latest").unwrap();
        assert!(ollama.local_available && ollama.provider_available);
    }

    /// Contract: the synthetic provider that the offline suite exercises is
    /// the only one that may claim runtime verification by default.
    #[test]
    fn only_synthetic_claims_default_runtime_verification() {
        for m in default_registry() {
            if m.provider == "zylcode" {
                assert!(m.runtime_verified);
            } else {
                assert!(
                    !m.runtime_verified,
                    "{} claims runtime verification without a recorded round-trip",
                    m.model_id
                );
            }
        }
    }

    /// Contract: registry_json is valid and carries the audit note.
    #[test]
    fn json_payload_is_well_formed() {
        let v = registry_json();
        assert_eq!(v["contract"], "model-capability-metadata/1");
        assert!(v["models"].as_array().unwrap().len() >= 4);
        let note = v["note"].as_str().unwrap();
        assert!(note.contains("transport only"));
    }
}
