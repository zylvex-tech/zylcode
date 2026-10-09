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

use std::sync::Arc;

use zylcode_core::competition::natlas::{
    LocalNatlasTransport, NatlasClient, NatlasRequest,
};
use zylcode_core::competition::natlas::intent::NatlasEngineeringIntent;
use zylcode_core::factory::{
    FactoryJob, FactoryRunner, FactoryRunnerConfig, RunStatus, TaskAction, TaskState,
};
use zylcode_mcp::{JsonlEvidenceSink, PermissionGate, PermissionPolicy, RiskLevel, ToolRuntime};

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

/// EV-024: Genuine live N-ATLAS → ZylCode engineering acceptance journey.
///
/// This test performs the full uninterrupted chain:
///   Yoruba instruction → real N-ATLAS → parse with repair → TaskGraph →
///   approval → FactoryRunner → real file mutation → real test → evidence
///
/// It is IGNORED because it requires a live endpoint and a real API key.
/// Run with:
///   export NATLAS_BASE_URL="https://zylvex-natlas-zylcode-bridge.hf.space"
///   export NATLAS_REQUEST_PATH="/v1/chat/completions"
///   export NATLAS_MODEL="NCAIR1/N-ATLaS"
///   export NATLAS_API_KEY="<key>"
///   cargo test -p zylcode-core --test natlas_live -- --ignored --nocapture live_acceptance_journey
#[tokio::test]
#[ignore = "genuine network call; requires a live N-ATLAS endpoint in NATLAS_* env vars"]
async fn live_acceptance_journey() {
    let transport = LocalNatlasTransport::new();
    let client = NatlasClient::from_env(transport, "live-acceptance-test").unwrap_or_else(|e| {
        panic!(
            "N-ATLAS is not configured, so this live test cannot run: {e}\n\
             Set NATLAS_BASE_URL, NATLAS_REQUEST_PATH, NATLAS_MODEL and NATLAS_API_KEY."
        )
    });

    // 1. Send Yoruba request to real N-ATLAS
    let system_preamble = concat!(
        "You are the N-ATLAS engineering planner inside ZylCode. ",
        "A developer will give you a request. Understand it, then ",
        "respond with ONLY a JSON object matching exactly this schema:\n",
        "{\"summary\": \"<one-line English summary>\", ",
        "\"steps\": [ ",
        "{\"kind\":\"approve\",\"description\":\"owner approves the plan\"}, ",
        "{\"kind\":\"implement\",\"description\":\"...\",\"path\":\"<file>\",\"content\":\"<full file text with NEWLINES as ESCAPED backslash-n>\"}, ",
        "{\"kind\":\"test\",\"description\":\"...\",\"command\":[\"python\",\"<file>\"],\"verify\":true} ]}\n",
        "\n",
        "CRITICAL RULES:\n",
        "1. Output JSON ONLY. No prose. No markdown fences.\n",
        "2. All string values must use DOUBLE QUOTES with valid JSON escaping.\n",
        "3. NEVER use triple-quoted strings.\n",
        "4. Embedded newlines in 'content' MUST be written as \\n (backslash + n).\n",
        "5. The 'content' field must contain VALID PYTHON CODE in English.\n",
        "6. No trailing commas."
    );

    let yoruba_request = "Jọwọ, ṣẹda faili Python kékeré tí yoo sọ 'Hello ZylCode' kí o sì ṣe àyẹ̀wò tí yoo ṣe é lábẹ́.";

    let request = NatlasRequest::new(yoruba_request, system_preamble);
    let invocation = client.invoke(&request).await;

    assert!(
        invocation.succeeded(),
        "live N-ATLAS must succeed: {:?} (error: {:?})",
        invocation.status,
        invocation.error
    );

    let response = invocation.response.expect("succeeded invocation has response");
    println!("--- LIVE ACCEPTANCE JOURNEY ---");
    println!("model  : {}", response.model);
    println!("raw    : {}", response.text);

    // 2. Parse with repair (handles triple-quoted strings, raw newlines)
    let (intent, was_repaired) =
        NatlasEngineeringIntent::parse_with_repair(&response.text).unwrap_or_else(|e| {
            panic!("model output must be parseable after repair: {e}")
        });
    println!("repaired: {}", was_repaired);
    println!("summary : {}", intent.summary);
    println!("steps   : {}", intent.steps.len());

    // 3. Create disposable fixture repo
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("workspace");
    let workdir = root.join("project");
    std::fs::create_dir_all(&workdir).unwrap();

    // 4. Build task graph
    let graph = intent.to_task_graph().unwrap();

    // 5. Create factory job
    let mut job = FactoryJob::new(
        intent.summary.clone(),
        "proj-ev024",
        "https://github.com/zylvex-tech/zylcode",
        "main",
        workdir.to_string_lossy().to_string(),
        "live-test",
    );
    job.graph = graph;

    // 6. Create runner with permissive runtime
    let gate = Arc::new(PermissionGate::new(PermissionPolicy::allow_up_to(
        RiskLevel::Execute,
    )));
    let sink = Arc::new(JsonlEvidenceSink::new(root.join("evidence.jsonl")));
    let runtime = ToolRuntime::new(gate, sink);

    let mut runner = FactoryRunner::open(FactoryRunnerConfig {
        root: root.clone(),
        workdir: workdir.clone(),
        job,
        actor: "live-test".to_string(),
        max_steps: 16,
        runtime,
    })
    .unwrap();

    // 7. First run — must park at approval gate
    let first = runner.run().await.unwrap();
    println!("first run status: {:?}", first.status);
    assert_eq!(
        first.status,
        RunStatus::AwaitingApproval,
        "must park at approval gate before any mutation"
    );

    // 8. Verify no mutation before approval
    let mutation_step = runner
        .job
        .graph
        .nodes()
        .iter()
        .find(|n| matches!(n.action, TaskAction::WriteFile { .. }))
        .expect("has a write step");
    assert_eq!(
        mutation_step.state,
        TaskState::Pending,
        "mutation must be Pending, not executed, before approval"
    );
    let expected_file = workdir.join("hello_zylcode.py");
    assert!(
        !expected_file.exists(),
        "no file may exist before human approval"
    );
    println!("pre-approval: file does NOT exist — correct");

    // 9. Approve the gate
    let approval_id = runner
        .job
        .graph
        .nodes()
        .iter()
        .find(|n| matches!(n.action, TaskAction::Approval { .. }))
        .map(|n| n.id.clone())
        .expect("has approval step");
    runner.approve(&approval_id, "owner").unwrap();
    println!("approved by owner");

    // 10. Second run — executes mutation and test
    let second = runner.run().await.unwrap();
    println!("second run status: {:?}", second.status);
    assert_eq!(
        second.status,
        RunStatus::Completed,
        "must complete after approval"
    );

    // 11. Verify mutation
    assert!(
        expected_file.exists(),
        "file must exist after approved execution"
    );
    let content = std::fs::read_to_string(&expected_file).unwrap();
    println!("file content:\n{}", content);
    assert!(
        content.contains("Hello ZylCode"),
        "file must contain the expected greeting"
    );

    // 12. Verify test claim
    let claims = runner.claim_store().list().unwrap();
    let verified: Vec<_> = claims.iter().filter(|c| c.status == zylcode_core::claim::ClaimStatus::Verified).collect();
    println!("verified claims: {}", verified.len());
    assert!(
        verified.iter().any(|c| c.statement.contains("exited 0")),
        "a real exit-0 command must yield a verified claim"
    );

    // 13. Evidence graph provenance
    let doc = runner.graph_store().document().unwrap();
    assert!(doc.nodes.iter().any(|n| n.kind == zylcode_core::evidence_graph::NodeKind::Intent));
    println!("evidence graph has INTENT node — provenance chain intact");

    println!("=== LIVE ACCEPTANCE JOURNEY PASSED ===");
}
