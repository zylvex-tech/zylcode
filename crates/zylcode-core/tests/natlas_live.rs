//! Live N-ATLAS integration test — **IGNORED BY DEFAULT**.
//!
//! # Why this test is ignored
//!
//! The rest of the competition suite runs against a **named test double** or a
//! **dead port**. Those prove the deterministic half of the integration; they
//! prove nothing about a real N-ATLAS service. This test is the opposite: it
//! performs a **genuine network call**, so it must not run in CI, where no
//! endpoint exists.
//!
//! # How to run it (owner action)
//!
//! Point the standard configuration at a live, OpenAI-compatible N-ATLAS
//! endpoint and run with `--ignored`:
//!
//! ```bash
//! export NATLAS_BASE_URL="https://<host>"           # e.g. an N-ATLAS engine
//! export NATLAS_REQUEST_PATH="/v1/chat/completions"
//! export NATLAS_MODEL="NCAIR1/N-ATLaS"
//! export NATLAS_API_KEY="<key>"                     # never commit this
//! cargo test -p zylcode-core --test natlas_live -- --ignored --nocapture
//! ```
//!
//! The contract exercised is **Contract B** in
//! `docs/competition/natlas/NATLAS_CONTRACT_VERIFICATION_2026-10-07.md` — the
//! OpenAI-compatible N-ATLAS engine protocol. The request path is read from the
//! environment and is never hard-coded, so this test cannot invent an endpoint.
//!
//! # What a pass means, and what it does not
//!
//! A pass means: a real N-ATLAS deployment returned real model output to this
//! build. It does **not** mean N-ATLAS is generally available, and it is not a
//! substitute for capturing the exchange as an evidence artefact.

use zylcode_core::competition::natlas::{LocalNatlasTransport, NatlasClient, NatlasRequest};

#[tokio::test]
#[ignore = "genuine network call; requires a live N-ATLAS endpoint in NATLAS_* env vars"]
async fn live_natlas_returns_a_genuine_non_empty_response() {
    let transport = LocalNatlasTransport::new();

    let client = NatlasClient::from_env(transport, "live-integration-test").unwrap_or_else(|e| {
        panic!(
            "N-ATLAS is not configured, so this live test cannot run: {e}\n\
             Set NATLAS_BASE_URL, NATLAS_REQUEST_PATH, NATLAS_MODEL and NATLAS_API_KEY."
        )
    });

    let request = NatlasRequest::new(
        "Reply with exactly the word PONG and nothing else.",
        "You are a precise assistant. Follow the instruction exactly.",
    );

    let invocation = client.invoke(&request).await;

    assert!(
        invocation.succeeded(),
        "a live endpoint must produce a success, not {:?} (error: {:?})",
        invocation.status,
        invocation.error
    );

    let response = invocation
        .response
        .expect("a succeeded invocation must carry a response");

    assert!(
        !response.text.trim().is_empty(),
        "a genuine N-ATLAS response must not be empty"
    );
    assert!(
        !response.model.trim().is_empty(),
        "the server must report its model identity; identity is never assumed"
    );

    // Printed only under `--nocapture`, so a capture can be taken verbatim.
    println!("--- LIVE N-ATLAS INVOCATION ---");
    println!("model reported by server : {}", response.model);
    println!("request_id               : {:?}", response.request_id);
    println!("finish_reason            : {:?}", response.finish_reason);
    println!("usage                    : {:?}", response.usage);
    println!("text                     : {:?}", response.text);
}
