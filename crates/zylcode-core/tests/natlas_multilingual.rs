//! N-ATLAS multilingual regression suite — deterministic, no network.
//!
//! # Why this file exists (NAT-A-004)
//!
//! `docs/competition/natlas/MULTILINGUAL_VALIDATION_MATRIX.md` states that "the
//! SDK propagates a `language` field into the evidence record". Before this
//! suite that was true of the TypeScript SDK only: the Rust
//! `NatlasEvidence` had **no `language` field at all**, so a Rust-side
//! invocation could not record which language it exercised, and Igbo did not
//! appear anywhere in the Rust test tree.
//!
//! This suite closes that gap at the boundary level. It proves the ZylCode-side
//! **plumbing**: a stated language tag travels request → evidence → JSONL
//! unchanged, for every competition target, and is never inferred from the
//! text.
//!
//! # What this file does NOT prove
//!
//! It does **not** prove that N-ATLAS answers in any of these languages. Every
//! response below is produced by `MockNatlasTransport`, a **TEST DOUBLE**.
//! Language *quality* is a property of the model and is evidenced separately
//! (EV-015/016/017 in the matrix); no test here asserts that a real model call
//! occurred, and none may be cited as a multilingual capability claim.

use async_trait::async_trait;

use zylcode_core::competition::natlas::{
    NatlasClient, NatlasConfig, NatlasError, NatlasEvidence, NatlasRawResponse, NatlasRequest,
    NatlasTransport,
};

// ---------------------------------------------------------------------------
// The four competition targets + the non-target Pidgin bonus.
//
// Kept in one place so a reviewer can see the exact set the harness covers and
// compare it against the matrix. `pcm` (Nigerian Pidgin) is marked non-target.
// ---------------------------------------------------------------------------

/// `(bcp47_tag, human_name, is_competition_target)`.
const COVERED_LANGUAGES: &[(&str, &str, bool)] = &[
    ("en-NG", "English (Nigerian-accented)", true),
    ("ha", "Hausa", true),
    ("yo", "Yoruba", true),
    ("ig", "Igbo", true),
    ("pcm", "Nigerian Pidgin", false),
];

// ---------------------------------------------------------------------------
// TEST DOUBLE — explicitly not N-ATLAS.
// ---------------------------------------------------------------------------

/// Returns a fixed canonical body for any request. Performs no I/O.
///
/// **TEST DOUBLE.** A success through it is a statement about this file's
/// plumbing, not about N-ATLAS.
struct MockNatlasTransport;

#[async_trait]
impl NatlasTransport for MockNatlasTransport {
    async fn send(
        &self,
        _config: &NatlasConfig,
        _request: &NatlasRequest,
    ) -> Result<NatlasRawResponse, NatlasError> {
        Ok(NatlasRawResponse {
            status: 200,
            body: serde_json::json!({
                "model": "natlas-mock-model",
                "request_id": "req-mock-ml",
                "text": "ok",
                "usage": { "input_tokens": 1, "output_tokens": 1 },
                "finish_reason": "stop"
            })
            .to_string(),
        })
    }
}

fn client() -> NatlasClient<MockNatlasTransport> {
    NatlasClient::new(
        NatlasConfig::from_parts(
            "https://natlas.invalid", // reserved, never resolves
            "/v1/infer",
            "natlas-mock-model",
            "mock-key-not-a-real-secret-0000",
        )
        .unwrap(),
        MockNatlasTransport,
        "natlas-ml-test",
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Every competition target — and the non-target Pidgin bonus — must be
/// representable as a stated language tag and must survive the full
/// request → evidence → JSON round trip.
#[tokio::test]
async fn every_target_language_survives_the_request_to_evidence_round_trip() {
    let c = client();

    for (tag, name, _target) in COVERED_LANGUAGES {
        // The prompt is written in the target language; the tag merely records
        // which one. (A placeholder is fine here — the mock does not read it.)
        let request = NatlasRequest::new("<prompt in target language>", "system")
            .with_language(*tag);

        let inv = c.invoke(&request).await;
        assert!(inv.succeeded(), "{name} ({tag}) invocation must succeed");

        assert_eq!(
            inv.evidence.language.as_deref(),
            Some(*tag),
            "{name}: evidence must record the stated language tag"
        );

        // The tag must also be present in the serialised record, because that
        // is what an evidence file actually contains.
        let json = inv.evidence.to_json();
        assert_eq!(
            json["language"], *tag,
            "{name}: serialised evidence must carry the language tag"
        );
    }
}

/// The Igbo tag specifically must be representable. This is the language that
/// was entirely absent from the Rust tree before NAT-A-004.
#[tokio::test]
async fn igbo_is_recorded_and_not_silently_dropped() {
    let c = client();
    let request = NatlasRequest::new("depụta otu ọrụ", "system").with_language("ig");
    let inv = c.invoke(&request).await;
    assert_eq!(inv.evidence.language.as_deref(), Some("ig"));
}

/// A language is recorded **only** when the caller states it. It is never
/// guessed from the text — guessing would be exactly the kind of unmeasured
/// claim the evidence record exists to prevent.
#[tokio::test]
async fn language_is_never_inferred_from_the_prompt_text() {
    let c = client();

    // A prompt that is obviously Igbo, but with no stated tag.
    let request = NatlasRequest::new("depụta otu ọrụ na koodu", "system");
    let inv = c.invoke(&request).await;

    assert_eq!(
        inv.evidence.language, None,
        "language must stay unset when the caller does not state one"
    );
    let json = inv.evidence.to_json();
    assert!(
        json.get("language").is_none(),
        "an unstated language must be omitted, not defaulted"
    );
}

/// A blank or whitespace tag is not a language. It must normalise to `None`
/// rather than being stored as an empty string that would later read as
/// "language was recorded".
#[test]
fn blank_language_tags_normalise_to_none() {
    let err = NatlasError::Timeout { timeout_ms: 5 };
    for blank in ["", "   ", "\t"] {
        let ev = NatlasEvidence::for_failure("m", "general", &err, 5, 10, &[])
            .with_language(blank);
        assert_eq!(ev.language, None, "blank tag {blank:?} must normalise to None");
    }
}

/// A tag with surrounding whitespace is trimmed, not stored verbatim.
#[test]
fn language_tags_are_trimmed() {
    let err = NatlasError::Timeout { timeout_ms: 5 };
    let ev = NatlasEvidence::for_failure("m", "general", &err, 5, 10, &[])
        .with_language("  yo  ");
    assert_eq!(ev.language.as_deref(), Some("yo"));
}

/// The failure path must carry the language too, so a multilingual run that
/// fails records which language it was attempting.
#[tokio::test]
async fn language_is_recorded_on_the_failure_path_as_well() {
    /// Always fails at the transport layer.
    struct FailingTransport;

    #[async_trait]
    impl NatlasTransport for FailingTransport {
        async fn send(
            &self,
            _config: &NatlasConfig,
            _request: &NatlasRequest,
        ) -> Result<NatlasRawResponse, NatlasError> {
            Err(NatlasError::Transport {
                message: "unreachable".to_string(),
            })
        }
    }

    let c = NatlasClient::new(
        NatlasConfig::from_parts(
            "https://natlas.invalid",
            "/v1/infer",
            "natlas-mock-model",
            "mock-key-not-a-real-secret-0000",
        )
        .unwrap(),
        FailingTransport,
        "natlas-ml-test",
    );

    let request = NatlasRequest::new("haɗa gwaji", "system").with_language("ha");
    let inv = c.invoke(&request).await;

    assert!(!inv.succeeded());
    assert_eq!(
        inv.evidence.language.as_deref(),
        Some("ha"),
        "a failed Hausa attempt must still record that it was Hausa"
    );
}

/// The set of languages this suite covers must match the matrix's target list,
/// so the two cannot silently drift apart.
#[test]
fn covered_languages_match_the_documented_competition_targets() {
    let targets: Vec<&str> = COVERED_LANGUAGES
        .iter()
        .filter(|(_, _, target)| *target)
        .map(|(tag, _, _)| *tag)
        .collect();

    assert_eq!(
        targets,
        vec!["en-NG", "ha", "yo", "ig"],
        "competition targets must be exactly the four in the matrix"
    );
    assert!(
        COVERED_LANGUAGES.iter().any(|(tag, _, t)| *tag == "pcm" && !*t),
        "Pidgin must be present but marked non-target"
    );
}
