use proptest::prelude::*;
use zylcode_core::router::decision::{classify_verification_rung, VerificationRung};

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
    fn formal_proof_always_rung2(kind in prop_oneof![
        Just("FormalProofSpec"),
        Just("formal_proof_spec"),
        Just("FormalProof"),
        Just("formal_proof"),
    ]) {
        prop_assert_eq!(classify_verification_rung(&kind), VerificationRung::Rung2);
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
        // Rung3 and Rung4 are not yet assigned to any kind
        prop_assert!(
            rung <= VerificationRung::Rung2,
            "unexpected rung {:?} for kind '{}'; only Rung0-Rung2 are currently assigned",
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
