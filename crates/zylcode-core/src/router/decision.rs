//! Pure, side-effect-free permission-decision type for the tool-permission gate.
//!
//! `Decision` replaces the boolean permission model with a typed enum that
//! carries *why* the decision was made — critical for audit trails and the
//! verification ladder (Rung 3).
//!
//! This module also defines `VerificationRung` and the pure classifier that
//! maps pipeline artifacts to the minimum verification rung required.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Verification Rung — the verification ladder (v2 §3)
// ---------------------------------------------------------------------------

/// Verification ladder rung.
///
/// Each rung represents an increasing level of verification rigor. The
/// `classify_verification_rung` function maps a pipeline artifact to the
/// *minimum* rung required to verify it, purely based on artifact kind —
/// no I/O, no side effects, fully deterministic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationRung {
    /// No verification. Default when no artifact or verification is needed.
    Rung0,
    /// Lint + type-check only. Suitable for UI components and plugin manifests.
    Rung1,
    /// Property-based tests (proptest/quickcheck). Required for Rust modules
    /// and formal proof specs.
    Rung2,
    /// Formal specification (lightweight). Not yet implemented.
    Rung3,
    /// Full formal verification (Z3/Dafny). Not yet implemented.
    Rung4,
}

impl VerificationRung {
    /// Human-readable label for UI badges.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rung0 => "None",
            Self::Rung1 => "Lint & Type-Check",
            Self::Rung2 => "Property Tests",
            Self::Rung3 => "Formal Spec",
            Self::Rung4 => "Full Verification",
        }
    }

    /// Short badge text (≤12 chars for UI).
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Rung0 => "None",
            Self::Rung1 => "Lint",
            Self::Rung2 => "Props",
            Self::Rung3 => "Spec",
            Self::Rung4 => "Formal",
        }
    }

    /// CSS color class for the badge background.
    pub fn color_class(&self) -> &'static str {
        match self {
            Self::Rung0 => "bg-gray-500",
            Self::Rung1 => "bg-blue-500",
            Self::Rung2 => "bg-green-500",
            Self::Rung3 => "bg-yellow-500",
            Self::Rung4 => "bg-purple-500",
        }
    }
}

impl std::fmt::Display for VerificationRung {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Pure, side-effect-free classifier: maps a pipeline artifact kind string
/// to the minimum [`VerificationRung`] required.
///
/// # Rules (v2 §3)
///
/// | Artifact kind | Minimum rung |
/// |---|---|
/// | `UiComponent` | Rung1 (lint + type-check) |
/// | `PluginManifest` | Rung1 (lint + type-check) |
/// | `RustModule` | Rung2 (property-based tests) |
/// | `FormalProofSpec` | Rung3 (formal specification, not yet machine-verified) |
/// | anything else | Rung0 (no verification) |
pub fn classify_verification_rung(artifact_kind: &str) -> VerificationRung {
    match artifact_kind {
        "UiComponent" | "ui_component" | "ReactComponent" | "react_component" => {
            VerificationRung::Rung1
        }
        "PluginManifest" | "plugin_manifest" | "McpManifest" | "mcp_manifest" => {
            VerificationRung::Rung1
        }
        "RustModule" | "rust_module" | "RustCrate" | "rust_crate" => VerificationRung::Rung2,
        "FormalProofSpec" | "formal_proof_spec" | "FormalProof" | "formal_proof" => {
            VerificationRung::Rung3
        }
        _ => VerificationRung::Rung0,
    }
}

/// Why a tool invocation was allowed or denied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Tool is allowed — the agent holds a valid permission chain.
    Allow {
        /// The permission rule that granted access (e.g. `"policy:default"`).
        granted_by: String,
    },
    /// Tool is denied — no matching permission rule exists.
    DenyNoRule {
        /// Agent ID that was checked.
        agent_id: String,
        /// Tool that was requested.
        tool_id: String,
    },
    /// Tool is denied — a matching rule explicitly blocks it.
    DenyExplicit {
        /// The rule that caused the denial.
        blocked_by: String,
    },
    /// Tool is denied — the agent's session has expired or is invalid.
    DenySession {
        /// Reason the session is invalid (e.g. `"expired"`, `"revoked"`).
        reason: String,
    },
    /// Tool is denied — the agent is rate-limited.
    DenyRateLimited {
        /// Seconds until the limit resets.
        retry_after_secs: u64,
    },
}

impl Decision {
    /// Returns `true` if the decision is `Allow`.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Decision::Allow { .. })
    }

    /// Returns `true` if the decision is any `Deny*` variant.
    pub fn is_denied(&self) -> bool {
        !self.is_allowed()
    }

    /// Human-readable summary for audit logging.
    pub fn summary(&self) -> String {
        match self {
            Decision::Allow { granted_by } => format!("Allowed ({granted_by})"),
            Decision::DenyNoRule {
                agent_id,
                tool_id,
            } => format!("Denied: no rule for agent={agent_id} tool={tool_id}"),
            Decision::DenyExplicit { blocked_by } => format!("Denied: explicit block ({blocked_by})"),
            Decision::DenySession { reason } => format!("Denied: session {reason}"),
            Decision::DenyRateLimited { retry_after_secs } => {
                format!("Denied: rate-limited (retry in {retry_after_secs}s)")
            }
        }
    }
}

/// Pure, side-effect-free permission check.
///
/// Given an agent ID, tool ID, and a set of permission rules, returns a
/// `Decision` without performing any I/O, network calls, or side effects.
/// This function is deterministic: same inputs always produce the same output.
///
/// # Permission evaluation order
///
/// 1. If the session context indicates the session is invalid → `DenySession`.
/// 2. If an explicit deny rule matches → `DenyExplicit`.
/// 3. If an allow rule matches → `Allow`.
/// 4. If no rule matches → `DenyNoRule`.
pub fn check_permission(
    agent_id: &str,
    tool_id: &str,
    session_valid: bool,
    session_reason: Option<&str>,
    deny_rules: &[PermissionRule],
    allow_rules: &[PermissionRule],
) -> Decision {
    // 1. Session validity check.
    if !session_valid {
        return Decision::DenySession {
            reason: session_reason.unwrap_or("invalid").to_string(),
        };
    }

    // 2. Explicit deny rules take precedence.
    for rule in deny_rules {
        if rule.matches(agent_id, tool_id) {
            return Decision::DenyExplicit {
                blocked_by: rule.name.clone(),
            };
        }
    }

    // 3. Allow rules grant access.
    for rule in allow_rules {
        if rule.matches(agent_id, tool_id) {
            return Decision::Allow {
                granted_by: rule.name.clone(),
            };
        }
    }

    // 4. Default deny.
    Decision::DenyNoRule {
        agent_id: agent_id.to_string(),
        tool_id: tool_id.to_string(),
    }
}

/// A single permission rule used by the pure check function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionRule {
    /// Human-readable name for audit trails (e.g. `"policy:default"`).
    pub name: String,
    /// Agent pattern — `"*"` matches all agents.
    pub agent_pattern: String,
    /// Tool pattern — `"*"` matches all tools.
    pub tool_pattern: String,
}

impl PermissionRule {
    /// Simple glob matching: `"*"` matches everything, otherwise exact match.
    pub fn matches(&self, agent_id: &str, tool_id: &str) -> bool {
        let agent_ok = self.agent_pattern == "*" || self.agent_pattern == agent_id;
        let tool_ok = self.tool_pattern == "*" || self.tool_pattern == tool_id;
        agent_ok && tool_ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_with_valid_session() {
        let rules = vec![PermissionRule {
            name: "policy:default".into(),
            agent_pattern: "*".into(),
            tool_pattern: "*".into(),
        }];
        let d = check_permission("agent-1", "bash", true, None, &[], &rules);
        assert!(d.is_allowed());
        assert_eq!(
            d,
            Decision::Allow {
                granted_by: "policy:default".into()
            }
        );
    }

    #[test]
    fn deny_no_matching_rule() {
        let d = check_permission("agent-1", "bash", true, None, &[], &[]);
        assert!(d.is_denied());
        assert!(matches!(d, Decision::DenyNoRule { .. }));
    }

    #[test]
    fn deny_explicit_overrides_allow() {
        let allow = vec![PermissionRule {
            name: "allow-all".into(),
            agent_pattern: "*".into(),
            tool_pattern: "*".into(),
        }];
        let deny = vec![PermissionRule {
            name: "block-dangerous".into(),
            agent_pattern: "*".into(),
            tool_pattern: "rm".into(),
        }];
        let d = check_permission("agent-1", "rm", true, None, &deny, &allow);
        assert!(d.is_denied());
        assert!(matches!(d, Decision::DenyExplicit { .. }));
    }

    #[test]
    fn deny_session_expired() {
        let d = check_permission(
            "agent-1",
            "bash",
            false,
            Some("expired"),
            &[],
            &[PermissionRule {
                name: "allow-all".into(),
                agent_pattern: "*".into(),
                tool_pattern: "*".into(),
            }],
        );
        assert!(d.is_denied());
        assert!(matches!(d, Decision::DenySession { .. }));
    }

    #[test]
    fn specific_agent_rule() {
        let allow = vec![PermissionRule {
            name: "agent-1-tools".into(),
            agent_pattern: "agent-1".into(),
            tool_pattern: "*".into(),
        }];
        let d_allowed = check_permission("agent-1", "read", true, None, &[], &allow);
        assert!(d_allowed.is_allowed());
        let d_denied = check_permission("agent-2", "read", true, None, &[], &allow);
        assert!(d_denied.is_denied());
    }

    #[test]
    fn summary_messages() {
        let a = Decision::Allow {
            granted_by: "test".into(),
        };
        assert_eq!(a.summary(), "Allowed (test)");
        let d = Decision::DenyNoRule {
            agent_id: "a".into(),
            tool_id: "t".into(),
        };
        assert_eq!(d.summary(), "Denied: no rule for agent=a tool=t");
    }
}
