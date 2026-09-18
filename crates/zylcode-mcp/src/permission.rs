//! Permission gate — the decision layer between a tool request and an executor.
//!
//! # Why this exists
//!
//! Until now `RiskLevel` and `bound_operation()` were *declared* but nothing
//! acted on them. Every dispatch ran unconditionally, `ToolContext::
//! approval_required` was passed in and never read, and
//! `ToolEvidence::approval_decision` was never populated.
//!
//! That is a missing control, not a cosmetic gap. A tool system that classifies
//! risk but never gates on it has documented its policy and implemented none of
//! it. `shell.execute` — the one unbound escape hatch — was reachable exactly
//! like `fs.read`.
//!
//! # Default posture: deny above `Read`
//!
//! [`PermissionPolicy::default`] allows `Read` and requires explicit approval
//! for everything above it. This is deliberately the *restrictive* default: a
//! permissive default would mean every caller who forgot to configure a policy
//! silently got one that permits arbitrary execution, which is the same class of
//! defect as a fabricated success.
//!
//! Tests that need to exercise a mutating executor must say so explicitly with
//! [`PermissionPolicy::permissive`]. That name is the point: a reader can see
//! the choice.

use crate::real_tools::{RiskLevel, ToolContext};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// What the gate decided for one invocation.
///
/// A denial is a first-class outcome, not an error to be swallowed. It is
/// recorded in `ToolEvidence::approval_decision` whether or not the tool ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision", content = "reason")]
pub enum PermissionDecision {
    /// The invocation may proceed.
    Allow(String),
    /// The invocation must not proceed.
    Deny(String),
    /// The invocation may proceed only with explicit human approval, which has
    /// not been given.
    RequireApproval(String),
}

impl PermissionDecision {
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow(_))
    }

    pub fn reason(&self) -> &str {
        match self {
            Self::Allow(r) | Self::Deny(r) | Self::RequireApproval(r) => r,
        }
    }

    /// The string persisted into `ToolEvidence::approval_decision`.
    pub fn evidence_record(&self) -> String {
        let label = match self {
            Self::Allow(_) => "allow",
            Self::Deny(_) => "deny",
            Self::RequireApproval(_) => "require_approval",
        };
        format!("{label}: {}", self.reason())
    }
}

/// The policy a gate applies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    /// The highest risk class permitted without explicit approval.
    ///
    /// `None` means *nothing* is permitted by threshold — every id must be
    /// explicitly allowed or explicitly approved. `Read` is the lowest real
    /// risk class, so `None` is the only way to express "deny by default".
    pub allow_up_to: Option<RiskLevel>,
    /// Ids permitted regardless of risk class.
    pub allowed: HashSet<String>,
    /// Ids refused regardless of risk class. A denial always wins.
    pub denied: HashSet<String>,
}

impl Default for PermissionPolicy {
    /// Deny above `Read`.
    ///
    /// Reads proceed. Anything that writes, executes, mutates git state, is
    /// destructive or is a release step requires an explicit allow-list entry or
    /// `ToolContext::approval_required`.
    fn default() -> Self {
        Self {
            allow_up_to: Some(RiskLevel::Read),
            allowed: HashSet::new(),
            denied: HashSet::new(),
        }
    }
}

impl PermissionPolicy {
    /// Permit everything. **Explicitly named so it cannot be reached by
    /// accident.** Intended for tests and for a caller that has its own gate.
    pub fn permissive() -> Self {
        Self {
            allow_up_to: Some(RiskLevel::Release),
            allowed: HashSet::new(),
            denied: HashSet::new(),
        }
    }

    /// Nothing runs without explicit approval or an explicit allow entry.
    ///
    /// Named for what it does. It is not called `deny_all` because approval
    /// still wins over the threshold: the threshold permits nothing, so
    /// everything needs approval. A true unconditional denial is
    /// [`PermissionPolicy::deny`] applied to each id.
    pub fn approval_required_for_all() -> Self {
        Self {
            allow_up_to: None,
            allowed: HashSet::new(),
            denied: HashSet::new(),
        }
    }

    /// The restrictive default: deny above `Read`.
    pub fn restrictive() -> Self {
        Self::default()
    }

    /// A policy that permits up to `level` without approval.
    pub fn allow_up_to(level: RiskLevel) -> Self {
        Self {
            allow_up_to: Some(level),
            ..Default::default()
        }
    }

    /// Move the approval threshold to `level`.
    pub fn with_threshold(mut self, level: RiskLevel) -> Self {
        self.allow_up_to = Some(level);
        self
    }

    /// Permit `tool_id` regardless of risk class.
    pub fn allow(mut self, tool_id: impl Into<String>) -> Self {
        self.allowed.insert(tool_id.into());
        self
    }

    /// Refuse `tool_id` regardless of risk class.
    pub fn deny(mut self, tool_id: impl Into<String>) -> Self {
        self.denied.insert(tool_id.into());
        self
    }

    /// Decide one invocation.
    ///
    /// Precedence, highest first:
    ///   1. an explicit deny
    ///   2. an explicit allow
    ///   3. explicit human approval on the context
    ///   4. the risk threshold
    ///
    /// Deny wins over everything. Allow wins over the threshold. Approval wins
    /// over the threshold but not over a deny.
    pub fn decide(
        &self,
        tool_id: &str,
        risk: RiskLevel,
        context: &ToolContext,
    ) -> PermissionDecision {
        if self.denied.contains(tool_id) {
            return PermissionDecision::Deny(format!("`{tool_id}` is on the deny list"));
        }
        if self.allowed.contains(tool_id) {
            return PermissionDecision::Allow(format!("`{tool_id}` is on the allow list"));
        }
        if let Some(limit) = self.allow_up_to {
            if risk <= limit {
                return PermissionDecision::Allow(format!(
                    "risk {risk:?} is within the permitted threshold ({limit:?})"
                ));
            }
        }
        if context.approval_required {
            return PermissionDecision::Allow(format!(
                "explicit approval granted for risk {risk:?}"
            ));
        }
        let threshold = match self.allow_up_to {
            Some(limit) => format!("{limit:?}"),
            None => "nothing".to_string(),
        };
        PermissionDecision::RequireApproval(format!(
            "risk {risk:?} exceeds the permitted threshold ({threshold}) \
             and no approval was granted"
        ))
    }
}

/// A gate. Wraps a policy and is what dispatch actually consults.
#[derive(Debug, Clone, Default)]
pub struct PermissionGate {
    policy: PermissionPolicy,
}

impl PermissionGate {
    pub fn new(policy: PermissionPolicy) -> Self {
        Self { policy }
    }

    /// The restrictive default: deny above `Read`.
    pub fn restrictive() -> Self {
        Self::default()
    }

    /// Permit everything. See [`PermissionPolicy::permissive`].
    pub fn permissive() -> Self {
        Self::new(PermissionPolicy::permissive())
    }

    pub fn policy(&self) -> &PermissionPolicy {
        &self.policy
    }

    pub fn decide(&self, tool_id: &str, risk: RiskLevel, context: &ToolContext) -> PermissionDecision {
        self.policy.decide(tool_id, risk, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;

    fn ctx(approval: bool) -> ToolContext {
        ToolContext {
            working_directory: PathBuf::from("."),
            environment: HashMap::new(),
            timeout: Duration::from_secs(5),
            session_id: None,
            actor: Some("test-actor".to_string()),
            approval_required: approval,
        }
    }

    #[test]
    fn read_is_permitted_by_default() {
        let gate = PermissionGate::restrictive();
        for (id, risk) in [
            ("fs.read", RiskLevel::Read),
            ("fs.list", RiskLevel::Read),
            ("git.status", RiskLevel::Read),
            ("search.grep", RiskLevel::Read),
        ] {
            assert!(
                gate.decide(id, risk, &ctx(false)).is_allow(),
                "`{id}` ({risk:?}) should be permitted by default"
            );
        }
    }

    #[test]
    fn everything_above_read_requires_approval_by_default() {
        let gate = PermissionGate::restrictive();
        for (id, risk) in [
            ("fs.write", RiskLevel::Write),
            ("shell.execute", RiskLevel::Execute),
            ("npm.run", RiskLevel::Execute),
            ("cargo.test", RiskLevel::Execute),
            ("git.commit", RiskLevel::GitWrite),
        ] {
            let decision = gate.decide(id, risk, &ctx(false));
            assert_eq!(
                decision,
                PermissionDecision::RequireApproval(decision.reason().to_string()),
                "`{id}` ({risk:?}) must not run unapproved"
            );
            assert!(matches!(decision, PermissionDecision::RequireApproval(_)));
        }
    }

    #[test]
    fn explicit_approval_permits_above_the_threshold() {
        let gate = PermissionGate::restrictive();
        assert!(gate
            .decide("shell.execute", RiskLevel::Execute, &ctx(true))
            .is_allow());
    }

    #[test]
    fn deny_wins_over_allow_and_over_approval() {
        let policy = PermissionPolicy::permissive()
            .deny("shell.execute");
        let gate = PermissionGate::new(policy);
        // even with explicit approval
        let decision = gate.decide("shell.execute", RiskLevel::Execute, &ctx(true));
        assert!(matches!(decision, PermissionDecision::Deny(_)), "{decision:?}");
    }

    #[test]
    fn deny_wins_even_at_read_risk() {
        let gate = PermissionGate::new(PermissionPolicy::permissive().deny("fs.read"));
        let decision = gate.decide("fs.read", RiskLevel::Read, &ctx(true));
        assert!(matches!(decision, PermissionDecision::Deny(_)), "{decision:?}");
    }

    #[test]
    fn allow_list_permits_above_the_threshold_without_approval() {
        let gate = PermissionGate::new(PermissionPolicy::restrictive().allow("cargo.test"));
        assert!(gate.decide("cargo.test", RiskLevel::Execute, &ctx(false)).is_allow());
        // and does not leak to a different tool
        assert!(matches!(
            gate.decide("shell.execute", RiskLevel::Execute, &ctx(false)),
            PermissionDecision::RequireApproval(_)
        ));
    }

    #[test]
    fn approval_required_for_all_gates_the_escape_hatch() {
        let gate = PermissionGate::new(PermissionPolicy::approval_required_for_all());
        // No approval -> refused, even at the lowest risk.
        assert!(matches!(
            gate.decide("fs.read", RiskLevel::Read, &ctx(false)),
            PermissionDecision::RequireApproval(_)
        ));
        // With approval -> permitted. Approval wins over the threshold, but an
        // unconditional denial is expressed with `deny(id)`, which always wins.
        assert!(gate.decide("fs.read", RiskLevel::Read, &ctx(true)).is_allow());
    }

    #[test]
    fn the_default_is_the_restrictive_one() {
        // Guards against someone "fixing" a failing test by making the default
        // permissive.
        let gate = PermissionGate::default();
        assert_eq!(gate.policy().allow_up_to, Some(RiskLevel::Read));
        assert!(matches!(
            gate.decide("shell.execute", RiskLevel::Execute, &ctx(false)),
            PermissionDecision::RequireApproval(_)
        ));
    }

    #[test]
    fn decision_is_recorded_with_its_label() {
        let gate = PermissionGate::restrictive();
        assert!(gate
            .decide("fs.read", RiskLevel::Read, &ctx(false))
            .evidence_record()
            .starts_with("allow: "));
        assert!(gate
            .decide("shell.execute", RiskLevel::Execute, &ctx(false))
            .evidence_record()
            .starts_with("require_approval: "));
        assert!(PermissionGate::new(PermissionPolicy::restrictive().deny("fs.read"))
            .decide("fs.read", RiskLevel::Read, &ctx(false))
            .evidence_record()
            .starts_with("deny: "));
    }

    #[test]
    fn risk_levels_are_ordered_by_severity() {
        assert!(RiskLevel::Read < RiskLevel::Write);
        assert!(RiskLevel::Write < RiskLevel::Execute);
        assert!(RiskLevel::Execute < RiskLevel::Network);
        assert!(RiskLevel::Network < RiskLevel::GitWrite);
        assert!(RiskLevel::GitWrite < RiskLevel::Destructive);
        assert!(RiskLevel::Destructive < RiskLevel::Release);
    }
}
