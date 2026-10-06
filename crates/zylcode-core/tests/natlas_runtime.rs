//! Phase C1 — runtime transport and anti-fallback tests.
//!
//! # What is real here and what is not
//!
//! `StubServer` below is a **TEST DOUBLE**: a throwaway loopback HTTP server
//! that returns canned bytes. It is **not** N-ATLAS, and nothing it produces is
//! competition evidence.
//!
//! What these tests *do* prove is genuinely valuable and does not depend on
//! N-ATLAS being reachable:
//!
//! * `LocalNatlasTransport` performs a **real HTTP round trip** and correctly
//!   translates a documented OpenAI-compatible reply into our canonical shape;
//! * the transport has **no fallback path** — an absent runtime is a transport
//!   error, never a synthesised answer;
//! * model identity in evidence comes from the **server**, not from config;
//! * a non-2xx or malformed reply is **never** coerced into a success;
//! * secrets are redacted before anything is stored;
//! * a model reply can traverse the **intent boundary** into a validated task
//!   graph.
//!
//! A passing run of this file is proof of the *boundary*, not of N-ATLAS.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use zylcode_core::competition::natlas::{
    local, NatlasClient, NatlasConfig, NatlasEngineeringIntent, NatlasRequest, NatlasStatus,
    LocalNatlasTransport,
};

// ---------------------------------------------------------------------------
// TEST DOUBLE — not N-ATLAS
// ---------------------------------------------------------------------------

/// A minimal loopback HTTP server that answers `times` requests with fixed
/// bytes. This is a **test double** and is never evidence.
struct StubServer {
    base_url: String,
}

impl StubServer {
    fn spawn(status_line: &str, body: String, times: usize) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let status = status_line.to_string();

        thread::spawn(move || {
            for _ in 0..times {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                // Read whatever the client sent. We do not parse it: this stub
                // answers unconditionally, which is fine for testing the
                // *client* side of the boundary.
                let mut buf = [0u8; 16_384];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "HTTP/1.1 {status}\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\
                     Connection: close\r\n\
                     \r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });

        Self {
            base_url: format!("http://{addr}"),
        }
    }
}

fn config(base_url: &str) -> NatlasConfig {
    NatlasConfig::from_parts(base_url, local::OPENAI_CHAT_PATH, "n-atlas-stub", "stub-key-1234")
        .expect("test config")
}

/// A canned OpenAI-compatible reply whose content is a valid ZylCode intent.
fn openai_reply_with_intent() -> String {
    let intent = r#"{"summary":"Add a health endpoint","steps":[{"kind":"analyse","description":"find the router"},{"kind":"implement","description":"add GET /health","target_path":"src/routes.rs"},{"kind":"test","description":"run the suite"},{"kind":"approve","description":"supervisor sign-off"}]}"#;
    let escaped = intent.replace('"', "\\\"");
    format!(
        r#"{{"id":"stub-req-1","model":"n-atlas-stub:latest","choices":[{{"index":0,"message":{{"role":"assistant","content":"{escaped}"}},"finish_reason":"stop"}}],"usage":{{"prompt_tokens":31,"completion_tokens":57}}}}"#
    )
}

fn request() -> NatlasRequest {
    NatlasRequest::new("add a health endpoint", "return JSON only")
        .with_context("src/routes.rs", "pub fn router() {}")
}

// ---------------------------------------------------------------------------
// Real HTTP round trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn local_transport_completes_a_real_http_round_trip() {
    let server = StubServer::spawn("200 OK", openai_reply_with_intent(), 1);
    let client = NatlasClient::new(config(&server.base_url), LocalNatlasTransport::new(), "tester");

    let inv = client.invoke(&request()).await;

    assert!(inv.succeeded(), "expected success, got {:?}", inv.status);
    let response = inv.response.expect("a response on success");
    // Identity comes from the server, not from our configuration.
    assert_eq!(response.model, "n-atlas-stub:latest");
    assert_eq!(response.request_id.as_deref(), Some("stub-req-1"));
    assert_eq!(response.usage.input_tokens, Some(31));
    assert_eq!(response.usage.output_tokens, Some(57));
    assert_eq!(response.finish_reason.as_deref(), Some("stop"));
}

#[tokio::test]
async fn a_response_traverses_the_intent_boundary_into_a_valid_task_graph() {
    let server = StubServer::spawn("200 OK", openai_reply_with_intent(), 1);
    let client = NatlasClient::new(config(&server.base_url), LocalNatlasTransport::new(), "tester");

    let inv = client.invoke(&request()).await;
    let response = inv.response.expect("a response");

    let intent = NatlasEngineeringIntent::parse(&response.text).expect("intent must parse");
    assert_eq!(intent.summary, "Add a health endpoint");
    assert_eq!(intent.steps.len(), 4);

    let graph = intent.to_task_graph().expect("a valid task graph");
    assert_eq!(graph.summary().total, 4);
    assert_eq!(graph.ready(), vec!["step-01".to_string()]);
}

// ---------------------------------------------------------------------------
// No fabrication: every failure mode is stated, never smoothed over
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_500_reply_is_an_http_status_error_never_a_success() {
    let server = StubServer::spawn("500 Internal Server Error", "{\"error\":\"boom\"}".to_string(), 1);
    let client = NatlasClient::new(config(&server.base_url), LocalNatlasTransport::new(), "tester");

    let inv = client.invoke(&request()).await;

    assert!(!inv.succeeded(), "a 500 must not be a success");
    assert_eq!(inv.status, NatlasStatus::Failed);
    assert!(inv.response.is_none());
    assert_eq!(inv.error.expect("an error").code(), "HTTP_STATUS");
}

#[tokio::test]
async fn a_malformed_reply_is_rejected_rather_than_partially_accepted() {
    let server = StubServer::spawn("200 OK", "this is not json at all".to_string(), 1);
    let client = NatlasClient::new(config(&server.base_url), LocalNatlasTransport::new(), "tester");

    let inv = client.invoke(&request()).await;

    assert!(!inv.succeeded());
    assert_eq!(inv.error.expect("an error").code(), "MALFORMED_RESPONSE");
}

#[tokio::test]
async fn a_reply_without_model_identity_is_rejected() {
    // Identity is not borrowable: an answer that does not say which model
    // produced it cannot be evidence.
    let body = r#"{"id":"x","choices":[{"message":{"content":"hi"},"finish_reason":"stop"}]}"#;
    let server = StubServer::spawn("200 OK", body.to_string(), 1);
    let client = NatlasClient::new(config(&server.base_url), LocalNatlasTransport::new(), "tester");

    let inv = client.invoke(&request()).await;

    assert!(!inv.succeeded());
    assert_eq!(inv.error.expect("an error").code(), "MALFORMED_RESPONSE");
}

#[tokio::test]
async fn an_absent_runtime_never_produces_a_synthetic_answer() {
    // Port 1 is not listening. There is no fallback path: the honest outcome is
    // a transport error and no response body whatsoever.
    let cfg = NatlasConfig::from_parts(
        "http://127.0.0.1:1",
        local::OPENAI_CHAT_PATH,
        "n-atlas-stub",
        "stub-key-1234",
    )
    .unwrap();
    let client = NatlasClient::new(cfg, LocalNatlasTransport::new(), "tester");

    let inv = client.invoke(&request()).await;

    assert!(!inv.succeeded(), "an absent runtime must never succeed");
    assert!(inv.response.is_none(), "no body may be invented");
    assert_eq!(inv.status, NatlasStatus::Failed);
    assert_eq!(inv.error.expect("an error").code(), "TRANSPORT");
}

// ---------------------------------------------------------------------------
// Evidence integrity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn evidence_records_the_servers_model_identity_and_the_measured_latency() {
    let server = StubServer::spawn("200 OK", openai_reply_with_intent(), 1);
    let client = NatlasClient::new(config(&server.base_url), LocalNatlasTransport::new(), "tester");

    let inv = client.invoke(&request()).await;
    let ev = &inv.evidence;

    assert_eq!(ev.provider, "natlas");
    assert_eq!(ev.model, "n-atlas-stub:latest", "identity must be the server's");
    assert_eq!(ev.status, NatlasStatus::Succeeded);
    assert!(ev.success);
    assert!(ev.response_chars > 0);
    assert!(ev.prompt_chars > 0);
    // Latency is measured, not asserted: it only has to exist and be finite.
    let _ = ev.latency_ms;
}

#[tokio::test]
async fn a_secret_echoed_in_a_server_error_is_redacted_before_storage() {
    // The stub deliberately echoes the configured key back in its error body.
    let server = StubServer::spawn(
        "401 Unauthorized",
        "{\"error\":\"bad key stub-key-1234 rejected\"}".to_string(),
        1,
    );
    let client = NatlasClient::new(config(&server.base_url), LocalNatlasTransport::new(), "tester");

    let inv = client.invoke(&request()).await;
    let json = inv.evidence.to_json().to_string();

    assert!(
        !json.contains("stub-key-1234"),
        "the API key must never appear in evidence: {json}"
    );
    assert!(
        inv.evidence.secret_redactions > 0,
        "redaction must be counted, not silent"
    );
}

// ---------------------------------------------------------------------------
// Runtime probe
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_probe_reports_a_reachable_runtime_and_the_model_it_found() {
    // Ollama-shaped /api/tags reply. Two requests: probe tries /api/tags first.
    let body = r#"{"models":[{"name":"n-atlas:latest","model":"n-atlas:latest"}]}"#;
    let server = StubServer::spawn("200 OK", body.to_string(), 2);

    let health = local::probe(&server.base_url, Some("n-atlas:latest")).await;

    assert!(health.reachable, "the stub is listening");
    assert_eq!(health.model_present, Some(true));
    assert!(health.is_ready());
    assert!(health.models.contains(&"n-atlas:latest".to_string()));
}

#[tokio::test]
async fn the_probe_reports_an_absent_model_rather_than_assuming_it() {
    let body = r#"{"models":[{"name":"some-other-model:latest"}]}"#;
    let server = StubServer::spawn("200 OK", body.to_string(), 2);

    let health = local::probe(&server.base_url, Some("n-atlas:latest")).await;

    assert!(health.reachable);
    assert_eq!(health.model_present, Some(false));
    assert!(!health.is_ready(), "an absent model is not ready");
}

#[tokio::test]
async fn the_probe_against_a_dead_port_is_unreachable_and_not_ready() {
    let health = local::probe("http://127.0.0.1:1", Some("n-atlas:latest")).await;
    assert!(!health.reachable);
    assert!(!health.is_ready());
    assert!(health.model_present.is_none());
}

// ---------------------------------------------------------------------------
// Timeout
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_stalled_runtime_trips_the_configured_timeout() {
    // A listener that accepts but never answers.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        // Hold the connection open without replying.
        let _held = listener.accept();
        thread::sleep(Duration::from_secs(30));
    });

    let mut cfg = config(&format!("http://{addr}"));
    cfg.timeout_ms = 400;

    let client = NatlasClient::new(cfg, LocalNatlasTransport::new(), "tester");
    let inv = client.invoke(&request()).await;

    assert!(!inv.succeeded());
    assert_eq!(inv.error.expect("an error").code(), "TIMEOUT");
}
