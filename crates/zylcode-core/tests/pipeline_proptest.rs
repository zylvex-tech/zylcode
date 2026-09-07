use proptest::prelude::*;
use zylcode_core::pipeline::validate_artifact_kind;

// ---------------------------------------------------------------------------
// Generators for file paths, content, and LLM-reported kinds
// ---------------------------------------------------------------------------

fn arb_extension() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("rs".into()),
        Just("tsx".into()),
        Just("jsx".into()),
        Just("ts".into()),
        Just("js".into()),
        Just("json".into()),
        Just("md".into()),
        Just("lean".into()),
        Just("unknown".into()),
    ]
}

fn arb_reported_kind() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("RustModule".into()),
        Just("rust_module".into()),
        Just("UiComponent".into()),
        Just("ui_component".into()),
        Just("ReactComponent".into()),
        Just("PluginManifest".into()),
        Just("plugin_manifest".into()),
        Just("FormalProofSpec".into()),
        Just("formal_proof".into()),
        Just("FormalProof".into()),
        Just("McpManifest".into()),
        Just("SomeRandomGarbage".into()),
        Just("".into()),
    ]
}

fn arb_content_for_kind(kind: &str) -> impl Strategy<Value = String> {
    match kind {
        "rs" => Just("fn main() { println!(\"hello\"); }".to_string()),
        "tsx" => {
            Just("import React from 'react'; export const App: FC = () => <div />;".to_string())
        }
        "jsx" => Just("import React from 'react'; export const App = () => <div />;".to_string()),
        "ts" => Just("import { useState } from 'react'; export const App: FC = () => {};".to_string()),
        "js" => Just("import React from 'react'; export const App = () => {};".to_string()),
        "json" => Just(r#"{"name":"test","version":"1.0.0"}"#.to_string()),
        "md" => {
            Just("# Theorem: correctness\n\nThis obligation must be proved by the model.".to_string())
        }
        "lean" => Just("theorem hello : 1 + 1 = 2 := by\n  simp".to_string()),
        _ => Just("some random content with no keywords".to_string()),
    }
    .boxed()
}

fn kind_for_ext(ext: &str) -> &'static str {
    match ext {
        "rs" => "RustModule",
        "tsx" | "jsx" | "ts" | "js" => "UiComponent",
        "json" | "jsonc" => "PluginManifest",
        "md" | "tex" | "lean" | "v" | "thy" => "FormalProofSpec",
        _ => "unknown",
    }
}

fn kind_matches_ext(kind: &str, ext: &str) -> bool {
    let kind_lower = kind.to_ascii_lowercase().replace('_', "").replace('-', "");
    let expected = kind_for_ext(ext);
    let expected_lower = expected.to_ascii_lowercase();
    kind_lower == expected_lower
}

// ---------------------------------------------------------------------------
// Property: kind never survives an extension mismatch uncorrected
//
// For every path with a recognized extension, validate_artifact_kind must
// return a kind consistent with that extension, regardless of what the LLM
// reported.  The only exception is paths with unrecognized extensions, where
// content heuristics or the LLM report may win.
// ---------------------------------------------------------------------------

proptest! {
    /// When the file has a recognized extension, the corrected kind always
    /// matches the extension, even if the LLM reported something random.
    #[test]
    fn corrected_kind_matches_extension(
        ext in arb_extension(),
        reported in arb_reported_kind(),
    ) {
        let path = format!("src/components/Generated.{}", ext);
        let content = "some content";
        let corrected = validate_artifact_kind(&path, content, &reported);
        // For recognized extensions, the corrected kind must match the extension.
        let expected = kind_for_ext(&ext);
        if expected != "unknown" {
            prop_assert!(
                kind_matches_ext(&corrected, &ext),
                "path={}, reported={}, corrected={}, expected_kind={}",
                path, reported, corrected, expected,
            );
        }
    }

    /// When the extension IS the source of truth, the LLM-reported kind is
    /// overridden — a downgrade attack (.rs claimed as UiComponent) is corrected.
    #[test]
    fn downgrade_attack_corrected(
        reported in prop_oneof![
            Just("UiComponent"),
            Just("PluginManifest"),
            Just("FormalProofSpec"),
            Just("SomeRandomThing"),
        ],
    ) {
        let path = "src/lib.rs";
        let corrected = validate_artifact_kind(path, "fn helper() {}", &reported);
        // .rs extension → always RustModule, regardless of what the LLM claimed.
        prop_assert_eq!(
            corrected.as_str(),
            "RustModule",
            "downgrade attack survived: reported={}, corrected={}",
            reported, corrected,
        );
    }

    /// When the extension IS the source of truth, the LLM-reported kind is
    /// overridden — an upgrade attack (.json claimed as RustModule) is corrected.
    #[test]
    fn upgrade_attack_corrected(
        reported in prop_oneof![
            Just("RustModule"),
            Just("FormalProofSpec"),
            Just("SomeRandomThing"),
        ],
    ) {
        let path = "plugin-manifest.json";
        let corrected = validate_artifact_kind(path, r#"{"name":"test"}"#, &reported);
        // .json extension → always PluginManifest, regardless of what the LLM claimed.
        prop_assert_eq!(
            corrected.as_str(),
            "PluginManifest",
            "upgrade attack survived: reported={}, corrected={}",
            reported, corrected,
        );
    }

    /// For unknown extensions, content heuristics may override the reported kind,
    /// but the result is always a valid kind string (never empty).
    #[test]
    fn unknown_extension_never_returns_empty(
        reported in arb_reported_kind(),
    ) {
        let path = "artifact/file.unknown";
        let corrected = validate_artifact_kind(path, "some content", &reported);
        prop_assert!(
            !corrected.is_empty(),
            "validate_artifact_kind returned empty for unknown extension, reported={}",
            reported,
        );
    }

    /// For recognized extensions, the corrected kind is ALWAYS one of the four
    /// canonical kinds, even when the LLM reports garbage.
    #[test]
    fn recognized_extension_always_canonical(
        ext in prop_oneof![
            Just("rs"), Just("tsx"), Just("jsx"), Just("ts"), Just("js"),
            Just("json"), Just("md"), Just("lean"),
        ],
        reported in arb_reported_kind(),
    ) {
        let path = format!("src/file.{}", ext);
        let corrected = validate_artifact_kind(&path, "content", &reported);
        let canonical = [
            "RustModule", "UiComponent", "PluginManifest", "FormalProofSpec",
        ];
        prop_assert!(
            canonical.contains(&corrected.as_str()),
            "non-canonical kind for recognized extension: ext={}, corrected={}, reported={}",
            ext, corrected, reported,
        );
    }

    /// The corrected kind is case-insensitive: normalize_kind(kind) must equal
    /// normalize_kind(kind_for_ext(ext)) for any recognized extension.
    #[test]
    fn corrected_kind_is_normalized(
        ext in arb_extension(),
        reported in arb_reported_kind(),
    ) {
        let path = format!("src/file.{}", ext);
        let corrected = validate_artifact_kind(&path, "content", &reported);
        let expected = kind_for_ext(&ext);
        if expected != "unknown" {
            let corrected_norm = corrected.to_ascii_lowercase().replace('_', "").replace('-', "");
            let expected_norm = expected.to_ascii_lowercase();
            prop_assert_eq!(
                corrected_norm, expected_norm,
                "normalized mismatch: ext={}, corrected={}, expected={}",
                ext, corrected, expected,
            );
        }
    }
}
