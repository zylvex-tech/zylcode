use proptest::prelude::*;
use zylcode_core::router::decision::{
    check_permission, classify_verification_rung, Decision, PermissionRule, VerificationRung,
};

// ---------------------------------------------------------------------------
// Property: known artifact kinds always map to the expected rung
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn ui_component_always_rung1(kind in prop_oneof![
        Just("UiComponent"),
        Just("ui_component"),
        Just("ReactComponent"),
        Just("react_component"),
    ]) {
        prop_assert_eq!(classify_verification_rung(&kind), VerificationRung::Rung1);
    }

    #[test]
    fn plugin_manifest_always_rung1(kind in prop_oneof![
        Just("PluginManifest"),
        Just("plugin_manifest"),
        Just("McpManifest"),
        Just("mcp_manifest"),
    ]) {
        prop_assert_eq!(classify_verification_rung(&kind), VerificationRung::Rung1);
    }

    #[test]
    fn rust_module_always_rung2(kind in prop_oneof![
        Just("RustModule"),
        Just("rust_module"),
        Just("RustCrate"),
        Just("rust_crate"),
    ]) {
        prop_assert_eq!(classify_verification_rung(&kind), VerificationRung::Rung2);
    }

    #[test]
    fn formal_proof_always_rung3(kind in prop_oneof![
        Just("FormalProofSpec"),
        Just("formal_proof_spec"),
        Just("FormalProof"),
        Just("formal_proof"),
    ]) {
        prop_assert_eq!(classify_verification_rung(&kind), VerificationRung::Rung3);
    }
}

// ---------------------------------------------------------------------------
// Property: unknown strings never produce Rung1 or Rung2
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn unknown_kind_always_rung0(kind in "[a-zA-Z0-9_]{1,60}") {
        let rung = classify_verification_rung(&kind);
        // Only known kinds map to Rung1/Rung2; anything else must be Rung0
        // (this test may fail if a generated string matches a known kind —
        //  we filter those out below).
        match kind.as_str() {
            "UiComponent" | "ui_component" | "ReactComponent" | "react_component"
            | "PluginManifest" | "plugin_manifest" | "McpManifest" | "mcp_manifest"
            | "RustModule" | "rust_module" | "RustCrate" | "rust_crate"
            | "FormalProofSpec" | "formal_proof_spec" | "FormalProof" | "formal_proof" => {
                // Known kind — skip, covered by dedicated tests above.
            }
            _ => {
                prop_assert_eq!(rung, VerificationRung::Rung0,
                    "unknown kind '{}' should map to Rung0, got {:?}", kind, rung);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property: classifier is deterministic
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn deterministic(kind in "[a-zA-Z0-9_]{1,60}") {
        let a = classify_verification_rung(&kind);
        let b = classify_verification_rung(&kind);
        prop_assert_eq!(a, b, "classify_verification_rung not deterministic for '{}'", kind);
    }
}

// ---------------------------------------------------------------------------
// Property: classification only returns valid rung variants
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn only_valid_rungs(kind in "[a-zA-Z0-9_]{1,60}") {
        let rung = classify_verification_rung(&kind);
        // Rung4 is not yet assigned to any kind
        prop_assert!(
            rung <= VerificationRung::Rung3,
            "unexpected rung {:?} for kind '{}'; only Rung0-Rung3 are currently assigned",
            rung, kind
        );
    }
}

// ---------------------------------------------------------------------------
// Property: every assigned rung is reachable
// ---------------------------------------------------------------------------

#[test]
fn rung0_is_reachable() {
    assert_eq!(
        classify_verification_rung("TotallyUnknown"),
        VerificationRung::Rung0
    );
}

#[test]
fn rung1_is_reachable() {
    assert_eq!(
        classify_verification_rung("UiComponent"),
        VerificationRung::Rung1
    );
}

#[test]
fn rung2_is_reachable() {
    assert_eq!(
        classify_verification_rung("RustModule"),
        VerificationRung::Rung2
    );
}

#[test]
fn rung3_is_reachable() {
    assert_eq!(
        classify_verification_rung("FormalProofSpec"),
        VerificationRung::Rung3
    );
}

// ---------------------------------------------------------------------------
// Regression: exact case sensitivity and separator variants
// ---------------------------------------------------------------------------

#[test]
fn case_sensitive_no_match() {
    // PascalCase is handled but camelCase / UPPER_CASE are not.
    assert_eq!(
        classify_verification_rung("uicomponent"),
        VerificationRung::Rung0
    );
    assert_eq!(
        classify_verification_rung("UICOMPONENT"),
        VerificationRung::Rung0
    );
    assert_eq!(
        classify_verification_rung("ui-component"),
        VerificationRung::Rung0
    );
}

#[test]
fn empty_string_is_rung0() {
    assert_eq!(
        classify_verification_rung(""),
        VerificationRung::Rung0
    );
}

// ===========================================================================
// Property tests for check_permission()
// ===========================================================================

/// Strategy: generate a plausible permission rule with exact or wildcard patterns.
fn arb_rule() -> impl Strategy<Value = PermissionRule> {
    (
        "[a-zA-Z0-9_:-]{1,40}",
        prop_oneof![Just("*".to_string()), "[a-zA-Z0-9_]{1,30}"],
        prop_oneof![Just("*".to_string()), "[a-zA-Z0-9_]{1,30}"],
    )
        .prop_map(|(name, agent_pattern, tool_pattern)| PermissionRule {
            name,
            agent_pattern,
            tool_pattern,
        })
}

/// Strategy: generate (agent_id, tool_id) pair.
fn arb_ids() -> impl Strategy<Value = (String, String)> {
    ("[a-zA-Z0-9_]{1,30}", "[a-zA-Z0-9_]{1,30}")
}

// ---------------------------------------------------------------------------
// Property: soundness — Allow implies valid session, no deny rule matched,
// and at least one allow rule matched.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn allow_implies_valid_session(
        agent in "[a-zA-Z0-9_]{1,30}",
        tool in "[a-zA-Z0-9_]{1,30}",
        deny_rules in prop::collection::vec(arb_rule(), 0..5),
        allow_rules in prop::collection::vec(arb_rule(), 1..5),
    ) {
        // Force session valid = true so we can test allow path.
        let decision = check_permission(&agent, &tool, true, None, &deny_rules, &allow_rules);
        if let Decision::Allow { granted_by } = &decision {
            // Soundness: Allow means no deny rule matched.
            for rule in &deny_rules {
                prop_assert!(
                    !rule.matches(&agent, &tool),
                    "Deny rule '{}' matched but decision was Allow({})",
                    rule.name, granted_by
                );
            }
            // Soundness: Allow means at least one allow rule matched.
            let any_allow_matched = allow_rules.iter().any(|r| r.matches(&agent, &tool));
            prop_assert!(
                any_allow_matched,
                "Decision Allow({}) but no allow rule matched",
                granted_by
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Property: completeness — valid session + no deny + matching allow → Allow
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn valid_session_no_deny_matching_allow_implies_allow(
        agent in "[a-zA-Z0-9_]{1,30}",
        tool in "[a-zA-Z0-9_]{1,30}",
    ) {
        // Build an allow rule that matches exactly.
        let allow_rule = PermissionRule {
            name: "test-allow".into(),
            agent_pattern: agent.clone(),
            tool_pattern: tool.clone(),
        };
        let decision = check_permission(&agent, &tool, true, None, &[], &[allow_rule]);
        prop_assert!(
            decision.is_allowed(),
            "Expected Allow but got {:?}",
            decision
        );
    }
}

// ---------------------------------------------------------------------------
// Property: exhaustiveness — every call returns a valid Decision variant
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn always_returns_valid_variant(
        agent in "[a-zA-Z0-9_]{1,30}",
        tool in "[a-zA-Z0-9_]{1,30}",
        session_valid in prop::bool::ANY,
        session_reason in prop::option::of("[a-z]{1,20}"),
        deny_rules in prop::collection::vec(arb_rule(), 0..5),
        allow_rules in prop::collection::vec(arb_rule(), 0..5),
    ) {
        let decision = check_permission(
            &agent, &tool, session_valid, session_reason.as_deref(),
            &deny_rules, &allow_rules,
        );
        // The result must be exactly one of the five variants.
        let valid = matches!(
            decision,
            Decision::Allow { .. }
                | Decision::DenyNoRule { .. }
                | Decision::DenyExplicit { .. }
                | Decision::DenySession { .. }
                | Decision::DenyRateLimited { .. }
        );
        prop_assert!(valid, "Unexpected Decision variant: {:?}", decision);
    }
}

// ---------------------------------------------------------------------------
// Property: idempotence — same inputs always produce the same output
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn idempotent(
        agent in "[a-zA-Z0-9_]{1,30}",
        tool in "[a-zA-Z0-9_]{1,30}",
        session_valid in prop::bool::ANY,
        session_reason in prop::option::of("[a-z]{1,20}"),
        deny_rules in prop::collection::vec(arb_rule(), 0..5),
        allow_rules in prop::collection::vec(arb_rule(), 0..5),
    ) {
        let d1 = check_permission(
            &agent, &tool, session_valid, session_reason.as_deref(),
            &deny_rules, &allow_rules,
        );
        let d2 = check_permission(
            &agent, &tool, session_valid, session_reason.as_deref(),
            &deny_rules, &allow_rules,
        );
        prop_assert_eq!(d1, d2, "check_permission not idempotent");
    }
}

// ---------------------------------------------------------------------------
// Property: invalid session always denies regardless of rules
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn invalid_session_always_denies(
        agent in "[a-zA-Z0-9_]{1,30}",
        tool in "[a-zA-Z0-9_]{1,30}",
        deny_rules in prop::collection::vec(arb_rule(), 0..5),
        allow_rules in prop::collection::vec(arb_rule(), 0..5),
    ) {
        let decision = check_permission(
            &agent, &tool, false, Some("expired"), &deny_rules, &allow_rules,
        );
        prop_assert!(
            decision.is_denied(),
            "Invalid session should always deny, got {:?}",
            decision
        );
        prop_assert!(
            matches!(decision, Decision::DenySession { .. }),
            "Invalid session should return DenySession, got {:?}",
            decision
        );
    }
}

// ---------------------------------------------------------------------------
// Property: deny rules take precedence over allow rules
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn deny_overrides_allow(
        agent in "[a-zA-Z0-9_]{1,30}",
        tool in "[a-zA-Z0-9_]{1,30}",
    ) {
        let allow = PermissionRule {
            name: "allow-all".into(),
            agent_pattern: "*".into(),
            tool_pattern: "*".into(),
        };
        let deny = PermissionRule {
            name: "deny-this".into(),
            agent_pattern: agent.clone(),
            tool_pattern: tool.clone(),
        };
        let decision = check_permission(&agent, &tool, true, None, &[deny], &[allow]);
        prop_assert!(
            matches!(decision, Decision::DenyExplicit { .. }),
            "Deny should override Allow, got {:?}",
            decision
        );
    }
}

// ---------------------------------------------------------------------------
// Property: deny_summary contains agent and tool IDs for DenyNoRule
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn deny_no_rule_captures_ids(
        agent in "[a-zA-Z0-9_]{1,30}",
        tool in "[a-zA-Z0-9_]{1,30}",
    ) {
        let decision = check_permission(&agent, &tool, true, None, &[], &[]);
        match decision {
            Decision::DenyNoRule { agent_id, tool_id } => {
                prop_assert_eq!(agent_id, agent);
                prop_assert_eq!(tool_id, tool);
            }
            other => {
                prop_assert!(false, "Expected DenyNoRule, got {:?}", other);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property: Allow granted_by matches the first matching allow rule
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn allow_granted_by_is_first_matching_rule(
        agent in "[a-zA-Z0-9_]{1,30}",
        tool in "[a-zA-Z0-9_]{1,30}",
    ) {
        let first = PermissionRule {
            name: "first-rule".into(),
            agent_pattern: agent.clone(),
            tool_pattern: tool.clone(),
        };
        let second = PermissionRule {
            name: "second-rule".into(),
            agent_pattern: "*".into(),
            tool_pattern: "*".into(),
        };
        let decision = check_permission(&agent, &tool, true, None, &[], &[first, second]);
        match decision {
            Decision::Allow { granted_by } => {
                prop_assert_eq!(granted_by, "first-rule");
            }
            other => {
                prop_assert!(false, "Expected Allow, got {:?}", other);
            }
        }
    }
}
