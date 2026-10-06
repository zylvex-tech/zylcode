//! N-ATLAS boundary tests — deterministic, no network, no N-ATLAS.
//!
//! # What these tests do and do not prove
//!
//! They prove the **boundary** behaves correctly: configuration handling,
//! blocked state, response parsing, provider failure, timeout, malformed
//! response, evidence capture, secret redaction, and the handoff into the
//! factory task graph.
//!
//! They do **not** prove N-ATLAS integration. Every success below is produced
//! by `MockNatlasTransport`, which is a **TEST DOUBLE** and never competition
//! evidence. There is no real N-ATLAS call anywhere in this file, and no test
//! asserts that one occurred.
//!
//! The double implements the public `NatlasTransport` trait from the test side,
//! so no mock ships in the library.

use std::time::Duration;

use async_trait::async_trait;

use zylcode_core::competition::natlas::{
    redact_secrets, BlockedNatlasTransport, NatlasClient, NatlasConfig, NatlasEngineeringIntent,
    NatlasError, NatlasRawResponse, NatlasRequest, NatlasStatus, NatlasTransport,
    BLOCKED_NATLAS_ACCESS,
};
use zylcode_core::factory::graph::TaskKind;

// ---------------------------------------------------------------------------
// TEST DOUBLE — explicitly not N-ATLAS.
// ---------------------------------------------------------------------------

/// A programmable stand-in for the N-ATLAS transport.
///
/// **TEST DOUBLE.** It performs no I/O and knows nothing about N-ATLAS. A
/// `Succeeded` result produced through it is a statement about this file's
/// parser, not about the service.
struct MockNatlasTransport {
    behaviour: Behaviour,
}

enum Behaviour {
    /// Return a raw body with a given HTTP status.
    Respond { status: u16, body: String },
    /// Fail at the transport layer with a message.
    Fail(String),
    /// Wait, then answer — used to exercise the client-side timeout.
    Stall { delay: Duration, body: String },
}

#[async_trait]
impl NatlasTransport for MockNatlasTransport {
    async fn send(
        &self,
        _config: &NatlasConfig,
        _request: &NatlasRequest,
    ) -> Result<NatlasRawResponse, NatlasError> {
        match &self.behaviour {
            Behaviour::Respond { status, body } => Ok(NatlasRawResponse {
                status: *status,
                body: body.clone(),
            }),
            Behaviour::Fail(message) => Err(NatlasError::Transport {
                message: message.clone(),
            }),
            Behaviour::Stall { delay, body } => {
                tokio::time::sleep(*delay).await;
                Ok(NatlasRawResponse {
                    status: 200,
                    body: body.clone(),
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const MOCK_KEY: &str = "mock-key-not-a-real-secret-0000";

fn config(timeout_ms: u64) -> NatlasConfig {
    let mut c = NatlasConfig::from_parts(
        "https://natlas.invalid", // .invalid is reserved and never resolves
        "/v1/infer",
        "natlas-mock-model",
        MOCK_KEY,
    )
    .unwrap();
    c.timeout_ms = timeout_ms;
    c
}

fn client(behaviour: Behaviour, timeout_ms: u64) -> NatlasClient<MockNatlasTransport> {
    NatlasClient::new(
        config(timeout_ms),
        MockNatlasTransport { behaviour },
        "natlas-test",
    )
}

/// A body in the ZylCode-side response contract.
fn canonical(text: &str) -> String {
    serde_json::json!({
        "model": "natlas-mock-model",
        "request_id": "req-mock-1",
        "text": text,
        "usage": { "input_tokens": 11, "output_tokens": 22 },
        "finish_reason": "stop"
    })
    .to_string()
}

fn respond(body: String) -> Behaviour {
    Behaviour::Respond { status: 200, body }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[test]
fn missing_configuration_lists_every_absent_variable() {
    // from_parts applies the same "all required" rule as from_env, so this is
    // deterministic and does not touch process-global environment state.
    let err = NatlasConfig::from_parts("", "", "", "").unwrap_err();
    assert_eq!(err.missing.len(), 4, "all four variables must be reported");
    assert!(err.missing.contains(&"NATLAS_API_KEY"));
    assert!(err.to_string().contains("NATLAS_BASE_URL"));
}

#[test]
fn missing_api_key_is_a_configuration_error_not_a_default() {
    let err =
        NatlasConfig::from_parts("https://natlas.invalid", "/v1/infer", "m", "").unwrap_err();
    assert_eq!(err.missing, vec!["NATLAS_API_KEY"]);
}

#[test]
fn config_debug_and_redacted_view_never_contain_the_key() {
    let c = config(1000);
    assert!(!format!("{c:?}").contains(MOCK_KEY), "Debug must redact");
    let r = c.redacted();
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains(MOCK_KEY));
    assert!(r.api_key_present);
    assert_eq!(r.api_key_len, MOCK_KEY.len());
}

#[test]
fn endpoint_is_assembled_from_configuration_without_invented_parts() {
    let c = config(1000);
    assert_eq!(c.endpoint(), "https://natlas.invalid/v1/infer");
}

// ---------------------------------------------------------------------------
// Blocked state (the real Phase C0 runtime)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn blocked_transport_yields_blocked_natlas_access_not_failure() {
    let c = NatlasClient::new(config(1000), BlockedNatlasTransport::default_reason(), "op");
    let inv = c.invoke(&NatlasRequest::new("do something", "sys")).await;

    assert_eq!(inv.status, NatlasStatus::BlockedNatlasAccess);
    assert!(inv.blocked());
    assert!(!inv.succeeded());
    assert!(inv.response.is_none(), "a blocked call returns no response");
    assert_eq!(
        inv.evidence.error_code.as_deref(),
        Some(BLOCKED_NATLAS_ACCESS)
    );
    assert!(!inv.evidence.success);
}

// ---------------------------------------------------------------------------
// Success path (via the test double — NOT competition evidence)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_well_formed_response_is_parsed_and_recorded() {
    let c = client(respond(canonical("hello from the double")), 1000);
    let inv = c.invoke(&NatlasRequest::new("implement a thing", "sys")).await;

    assert_eq!(inv.status, NatlasStatus::Succeeded);
    let resp = inv.response.as_ref().expect("parsed response");
    assert_eq!(resp.text, "hello from the double");
    assert_eq!(resp.model, "natlas-mock-model");
    assert_eq!(resp.usage.output_tokens, Some(22));

    let ev = &inv.evidence;
    assert!(ev.success);
    assert_eq!(ev.model, "natlas-mock-model");
    assert_eq!(ev.request_id.as_deref(), Some("req-mock-1"));
    assert_eq!(ev.request_classification, "implementation");
    assert_eq!(ev.response_chars, resp.text.len());
}

// ---------------------------------------------------------------------------
// Failure paths
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_transport_failure_is_recorded_with_its_code() {
    let c = client(Behaviour::Fail("connection refused".to_string()), 1000);
    let inv = c.invoke(&NatlasRequest::new("x", "y")).await;

    assert_eq!(inv.status, NatlasStatus::Failed);
    assert_eq!(inv.evidence.error_code.as_deref(), Some("TRANSPORT"));
    assert!(!inv.evidence.success);
}

#[tokio::test]
async fn a_slow_provider_trips_the_client_timeout() {
    let c = client(
        Behaviour::Stall {
            delay: Duration::from_millis(400),
            body: canonical("too late"),
        },
        50, // timeout well below the stall
    );
    let inv = c.invoke(&NatlasRequest::new("x", "y")).await;

    assert_eq!(inv.status, NatlasStatus::Failed);
    assert_eq!(inv.evidence.error_code.as_deref(), Some("TIMEOUT"));
    assert!(inv.response.is_none());
}

#[tokio::test]
async fn a_non_2xx_status_is_not_coerced_into_a_response() {
    let c = client(
        Behaviour::Respond {
            status: 503,
            body: "upstream unavailable".to_string(),
        },
        1000,
    );
    let inv = c.invoke(&NatlasRequest::new("x", "y")).await;

    assert_eq!(inv.evidence.error_code.as_deref(), Some("HTTP_STATUS"));
    assert!(inv.response.is_none());
}

#[tokio::test]
async fn a_malformed_body_is_rejected_rather_than_partially_accepted() {
    let c = client(respond("this is not json".to_string()), 1000);
    let inv = c.invoke(&NatlasRequest::new("x", "y")).await;

    assert_eq!(inv.evidence.error_code.as_deref(), Some("MALFORMED_RESPONSE"));
    assert!(inv.response.is_none());
}

#[tokio::test]
async fn a_response_without_model_identity_is_rejected() {
    let body = serde_json::json!({ "text": "hi" }).to_string();
    let c = client(respond(body), 1000);
    let inv = c.invoke(&NatlasRequest::new("x", "y")).await;

    assert_eq!(inv.evidence.error_code.as_deref(), Some("MALFORMED_RESPONSE"));
}

// ---------------------------------------------------------------------------
// Secret handling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_secret_in_a_failure_message_is_redacted_before_storage() {
    let leaky = format!("auth failed: Bearer {MOCK_KEY}");
    let c = client(Behaviour::Fail(leaky), 1000);
    let inv = c.invoke(&NatlasRequest::new("x", "y")).await;

    let detail = inv.evidence.error_detail.clone().unwrap_or_default();
    assert!(!detail.contains(MOCK_KEY), "the key must not survive: {detail}");
    assert!(inv.evidence.secret_redactions >= 1);
}

#[test]
fn redaction_is_counted_and_reported() {
    let (out, n) = redact_secrets(&format!("a {MOCK_KEY} b {MOCK_KEY}"), &[MOCK_KEY]);
    assert_eq!(n, 2);
    assert!(!out.contains(MOCK_KEY));
}

// ---------------------------------------------------------------------------
// Evidence persistence
// ---------------------------------------------------------------------------

#[tokio::test]
async fn evidence_is_written_as_jsonl_and_contains_no_secret() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".zylcode").join("competition").join("natlas.jsonl");

    let c = client(respond(canonical("ok")), 1000);
    let inv = c.invoke(&NatlasRequest::new("explain the parser", "sys")).await;
    inv.evidence.append_jsonl(&path).unwrap();

    let body = std::fs::read_to_string(&path).unwrap();
    assert_eq!(body.lines().count(), 1);
    assert!(!body.contains(MOCK_KEY), "no secret may reach the evidence log");
    assert!(body.contains("\"provider\":\"natlas\""));
    assert!(body.contains("\"status\":\"succeeded\""));
    // `explain` classifies as analysis, not implementation.
    assert!(body.contains("\"request_classification\":\"analysis\""));
}

// ---------------------------------------------------------------------------
// Handoff into the bounded ZylCode workflow
// ---------------------------------------------------------------------------

const INTENT_JSON: &str = r#"{
    "summary": "Add a health endpoint",
    "steps": [
        {"kind": "analyse", "description": "find the router"},
        {"kind": "implement", "description": "add GET /health", "target_path": "src/routes.rs"},
        {"kind": "test", "description": "run the suite"},
        {"kind": "document", "description": "note the endpoint"},
        {"kind": "approve", "description": "supervisor sign-off"}
    ]
}"#;

#[tokio::test]
async fn a_response_becomes_a_validated_factory_task_graph() {
    // The double returns the intent as the response text — exactly the shape a
    // real model is asked to produce.
    let c = client(respond(canonical(INTENT_JSON)), 1000);
    let inv = c
        .invoke(&NatlasRequest::new("implement a health endpoint", "sys"))
        .await;
    assert!(inv.succeeded(), "precondition: the boundary parsed the response");

    let text = &inv.response.unwrap().text;
    let intent = NatlasEngineeringIntent::parse(text).expect("intent contract holds");
    let graph = intent.to_task_graph().expect("handoff produces a valid graph");

    let summary = graph.summary();
    assert_eq!(summary.total, 5);
    assert_eq!(summary.pending, 5, "nothing has run yet");
    assert_eq!(
        graph.ready(),
        vec!["step-01".to_string()],
        "only the first step is dependency-satisfied"
    );
    assert_eq!(
        graph.get("step-05").unwrap().kind,
        TaskKind::HumanApproval,
        "the approval step is a real gate"
    );
    assert_eq!(
        graph.get("step-03").unwrap().kind,
        TaskKind::Test,
        "the test step is typed as a test task"
    );
}

#[tokio::test]
async fn a_model_response_that_is_not_an_intent_fails_the_handoff_cleanly() {
    let c = client(respond(canonical("just some prose, no JSON here")), 1000);
    let inv = c.invoke(&NatlasRequest::new("x", "y")).await;

    assert!(inv.succeeded(), "the boundary itself succeeded");
    let text = &inv.response.unwrap().text;
    let err = NatlasEngineeringIntent::parse(text).unwrap_err();
    assert_eq!(err.code(), "MALFORMED_RESPONSE");
}

// ---------------------------------------------------------------------------
// Honesty guards
// ---------------------------------------------------------------------------

#[test]
fn mock_success_is_not_runtime_verification() {
    // A guard so this distinction cannot quietly erode: the crate exposes no
    // API that turns a mock result into an N-ATLAS runtime claim, and the
    // blocked status is the only one Phase C0 can reach without a real call.
    let status = zylcode_core::competition::natlas::runtime_status();
    assert_ne!(
        status,
        NatlasStatus::Succeeded,
        "runtime_status must never report success: no real call has been made"
    );
    assert!(matches!(
        status,
        NatlasStatus::NotConfigured | NatlasStatus::BlockedNatlasAccess
    ));
}
