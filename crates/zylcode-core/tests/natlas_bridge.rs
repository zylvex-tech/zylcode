//! Hermetic tests for the N-ATLAS → ZylCode engineering bridge.
//!
//! These tests prove the bridge *mechanics* — NL intent contract → validated
//! task graph → real factory execution with a human approval gate → real tool
//! actions → real test → evidence graph — without requiring a live N-ATLAS
//! endpoint. The contract-conformant intents used here stand in for what a live
//! model must emit; the genuine model emission is proven separately (EV-017).
//!
//! Every test that touches the factory uses a throwaway fixture directory, never
//! the real ZylCode source, and never leaves an artifact behind.

use std::path::Path;
use std::sync::Arc;

use zylcode_core::claim::ClaimStatus;
use zylcode_core::competition::natlas::client::NatlasClient;
use zylcode_core::competition::natlas::config::NatlasConfig;
use zylcode_core::competition::natlas::evidence::{redact_secrets, NatlasEvidence};
use zylcode_core::competition::natlas::intent::NatlasEngineeringIntent;
use zylcode_core::competition::natlas::transport::{NatlasRawResponse, NatlasTransport};
use zylcode_core::competition::natlas::types::{NatlasError, NatlasRequest, NatlasResilienceState};
use zylcode_core::factory::graph::TaskKind;
use zylcode_core::factory::{
    FactoryJob, FactoryRunner, FactoryRunnerConfig, RunStatus, TaskAction, TaskState,
};
use zylcode_mcp::{JsonlEvidenceSink, PermissionGate, PermissionPolicy, RiskLevel, ToolRuntime};

/// A runtime that permits read/write/execute and records evidence to disk.
fn permissive_runtime(dir: &Path) -> ToolRuntime {
    let gate = Arc::new(PermissionGate::new(PermissionPolicy::allow_up_to(
        RiskLevel::Execute,
    )));
    let sink = Arc::new(JsonlEvidenceSink::new(dir.join("evidence.jsonl")));
    ToolRuntime::new(gate, sink)
}

struct Env {
    _tmp: tempfile::TempDir,
    root: std::path::PathBuf,
    workdir: std::path::PathBuf,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("workspace");
    let workdir = root.join("project");
    std::fs::create_dir_all(&workdir).unwrap();
    Env {
        _tmp: tmp,
        root,
        workdir,
    }
}

fn config() -> NatlasConfig {
    NatlasConfig::from_parts(
        "https://example.invalid",
        "/v1/chat/completions",
        "NCAIR1/N-ATLaS",
        "secret-api-key-value",
    )
    .unwrap()
}

/// Build a factory job directly from a parsed intent on a fixture workdir.
fn job_from_intent(intent: &NatlasEngineeringIntent, workdir: &Path) -> FactoryJob {
    let graph = intent.to_task_graph().unwrap();
    let mut job = FactoryJob::new(
        intent.summary.clone(),
        "proj-zylcode",
        "https://github.com/zylvex-tech/zylcode",
        "main",
        workdir.to_string_lossy().to_string(),
        "tester",
    );
    job.graph = graph;
    job
}

// ---------------------------------------------------------------------------
// Intent translation (contract → real actions)
// ---------------------------------------------------------------------------

#[test]
fn implement_with_content_becomes_a_writefile_action() {
    let json = r#"{
        "summary": "add greeting module",
        "steps": [
            {"kind": "approve", "description": "owner sign-off"},
            {"kind": "implement", "description": "write greeting.txt",
             "path": "greeting.txt", "content": "hello zylcode\n"}
        ]
    }"#;
    let intent = NatlasEngineeringIntent::parse(json).unwrap();
    let graph = intent.to_task_graph().unwrap();
    let write = graph.get("step-02").unwrap();
    match &write.action {
        TaskAction::WriteFile { path, content } => {
            assert_eq!(path, "greeting.txt");
            assert_eq!(content, "hello zylcode\n");
        }
        other => panic!("expected WriteFile, got {other:?}"),
    }
}

#[test]
fn test_with_verify_becomes_a_verify_action_and_run_becomes_runcommand() {
    let json = r#"{
        "summary": "build and verify",
        "steps": [
            {"kind": "approve", "description": "owner sign-off"},
            {"kind": "run", "description": "format", "command": ["echo", "fmt"]},
            {"kind": "test", "description": "check", "command": ["echo", "check"], "verify": true}
        ]
    }"#;
    let intent = NatlasEngineeringIntent::parse(json).unwrap();
    let graph = intent.to_task_graph().unwrap();
    assert!(matches!(
        graph.get("step-02").unwrap().action,
        TaskAction::RunCommand { .. }
    ));
    assert!(matches!(
        graph.get("step-03").unwrap().action,
        TaskAction::Verify { .. }
    ));
}

// ---------------------------------------------------------------------------
// Malformed output is rejected, never partially accepted
// ---------------------------------------------------------------------------

#[test]
fn malformed_intent_is_rejected_not_silent() {
    let not_json = "the model returned prose, not JSON";
    let err = NatlasEngineeringIntent::parse(not_json).unwrap_err();
    assert_eq!(err.code(), "MALFORMED_RESPONSE");
    assert!(err.to_string().contains("JSON"));
}

#[test]
fn intent_with_unknown_kind_is_rejected() {
    let bad = r#"{"summary":"s","steps":[{"kind":"deploy","description":"x"}]}"#;
    let err = NatlasEngineeringIntent::parse(bad).unwrap_err();
    assert_eq!(err.code(), "MALFORMED_RESPONSE");
}

// ---------------------------------------------------------------------------
// Approval enforcement: no real mutation before human approval
// ---------------------------------------------------------------------------

#[tokio::test]
async fn approval_gate_parks_the_run_and_no_file_is_written_before_approval() {
    let json = r#"{
        "summary": "add greeting module",
        "steps": [
            {"kind": "approve", "description": "owner sign-off"},
            {"kind": "implement", "description": "write greeting.txt",
             "path": "greeting.txt", "content": "hello zylcode\n"}
        ]
    }"#;
    let intent = NatlasEngineeringIntent::parse(json).unwrap();
    let e = env();
    let job = job_from_intent(&intent, &e.workdir);

    let mut runner = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job,
        actor: "tester".to_string(),
        max_steps: 8,
        runtime: permissive_runtime(&e.root),
    })
    .unwrap();

    // First run must park at the approval gate; the file must NOT exist yet.
    let first = runner.run().await.unwrap();
    assert_eq!(first.status, RunStatus::AwaitingApproval);
    assert!(
        !e.workdir.join("greeting.txt").exists(),
        "no file may be written before human approval"
    );
    assert_eq!(
        runner.job.graph.get("step-01").unwrap().state,
        TaskState::AwaitingApproval
    );

    // Approving the wrong task is refused.
    assert!(runner.approve("step-02", "owner").is_err());

    // Approve the gate, then run: now the write lands.
    runner.approve("step-01", "owner").unwrap();
    let second = runner.run().await.unwrap();
    assert_eq!(second.status, RunStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(e.workdir.join("greeting.txt")).unwrap(),
        "hello zylcode\n"
    );
}

// ---------------------------------------------------------------------------
// Approved mutation executes; a failed command fails honestly
// ---------------------------------------------------------------------------

#[tokio::test]
async fn approved_chain_runs_write_then_verify_and_produces_a_verified_claim() {
    let json = r#"{
        "summary": "write a file and verify",
        "steps": [
            {"kind": "approve", "description": "owner sign-off"},
            {"kind": "implement", "description": "write greeting.txt",
             "path": "greeting.txt", "content": "hello zylcode\n"},
            {"kind": "run", "description": "announce", "command": ["echo", "executed"]},
            {"kind": "test", "description": "verify", "command": ["echo", "verified"], "verify": true}
        ]
    }"#;
    let intent = NatlasEngineeringIntent::parse(json).unwrap();
    let e = env();
    let job = job_from_intent(&intent, &e.workdir);

    let mut runner = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job,
        actor: "tester".to_string(),
        max_steps: 16,
        runtime: permissive_runtime(&e.root),
    })
    .unwrap();

    // Gate first (the approval node parks the run), then approve, then run.
    let first = runner.run().await.unwrap();
    assert_eq!(first.status, RunStatus::AwaitingApproval);
    runner.approve("step-01", "owner").unwrap();
    let report = runner.run().await.unwrap();

    assert_eq!(report.status, RunStatus::Completed);
    assert!(e.workdir.join("greeting.txt").exists());
    // A real exit-0 command must yield a verified test claim.
    let claims = runner.claim_store().list().unwrap();
    let verified: Vec<_> = claims
        .iter()
        .filter(|c| c.status == ClaimStatus::Verified)
        .collect();
    assert!(
        verified
            .iter()
            .any(|c| c.statement.contains("exited 0")),
        "a real exit-0 command must yield a verified claim: {:?}",
        claims.iter().map(|c| &c.statement).collect::<Vec<_>>()
    );
    assert!(report.graph_integrity, "evidence graph must verify");
    // Provenance: the graph carries the originating INTENT node and the last
    // node traces back to it.
    let doc = runner.graph_store().document().unwrap();
    assert!(doc
        .nodes
        .iter()
        .any(|n| n.kind == zylcode_core::evidence_graph::NodeKind::Intent));
    let last = doc.nodes.last().unwrap();
    let ancestry = runner.graph_store().ancestry(&last.id).unwrap();
    assert_eq!(
        ancestry.last().unwrap().kind,
        zylcode_core::evidence_graph::NodeKind::Intent
    );
}

#[tokio::test]
async fn a_command_that_misses_its_expected_exit_is_failed_not_silent() {
    // A `RunCommand` that expects exit 7 but `echo` exits 0 must be Failed and
    // its unmet expectation recorded as a contradicted claim — never dropped.
    let e = env();
    let mut job = FactoryJob::new(
        "force a failing command",
        "p",
        "r",
        "main",
        e.workdir.to_string_lossy().to_string(),
        "tester",
    );
    job.graph
        .add_node(
            zylcode_core::factory::graph::TaskNode::new(
                "approve",
                TaskKind::HumanApproval,
                "owner sign-off",
                TaskAction::Approval {
                    reason: "sign-off".to_string(),
                },
            ),
        )
        .unwrap();
    job.graph
        .add_node(
            zylcode_core::factory::graph::TaskNode::new(
                "bad",
                TaskKind::Implementation,
                "command that will not meet its expectation",
                TaskAction::RunCommand {
                    command: "echo".to_string(),
                    args: vec!["x".to_string()],
                    expect_exit: 7,
                },
            )
            .depends_on(&["approve"]),
        )
        .unwrap();

    let mut runner = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job,
        actor: "tester".to_string(),
        max_steps: 8,
        runtime: permissive_runtime(&e.root),
    })
    .unwrap();

    // Gate first (the approval node parks the run), then approve, then run.
    let first = runner.run().await.unwrap();
    assert_eq!(first.status, RunStatus::AwaitingApproval);
    runner.approve("approve", "owner").unwrap();
    let report = runner.run().await.unwrap();
    assert_eq!(report.status, RunStatus::Blocked);
    assert_eq!(
        runner.job.graph.get("bad").unwrap().state,
        TaskState::Failed
    );
    assert_eq!(report.failures.len(), 1);
    assert!(report.failures[0].cause.contains("expected exit 7"));
    // The unmet expectation is recorded as a contradicted claim, not dropped.
    let claims = runner.claim_store().list().unwrap();
    assert!(claims
        .iter()
        .any(|c| c.status == ClaimStatus::Contradicted));
}

// ---------------------------------------------------------------------------
// Endpoint resilience: unavailable and auth-failure are stated, never faked
// ---------------------------------------------------------------------------

struct MockErrorTransport {
    status: u16,
    body: String,
}

#[async_trait::async_trait]
impl NatlasTransport for MockErrorTransport {
    async fn send(
        &self,
        _config: &NatlasConfig,
        _request: &NatlasRequest,
    ) -> Result<NatlasRawResponse, NatlasError> {
        Ok(NatlasRawResponse {
            status: self.status,
            body: self.body.clone(),
        })
    }
}

#[tokio::test]
async fn endpoint_unavailable_is_blocked_with_no_synthetic_response() {
    let t = MockErrorTransport {
        status: 503,
        body: "Service Unavailable".to_string(),
    };
    let client = NatlasClient::new(config(), t, "tester");
    let inv = client
        .invoke(&NatlasRequest::new("do a thing", "system"))
        .await;
    assert!(!inv.succeeded());
    // No synthetic model text is ever produced.
    assert!(inv.response.is_none());
    // The resilience state is stated truthfully.
    let state = NatlasResilienceState::from_error(
        inv.error.as_ref().unwrap(),
    );
    assert!(!state.is_operational());
    assert_eq!(state, NatlasResilienceState::Warming);
}

#[tokio::test]
async fn auth_failure_is_stated_not_substituted() {
    let t = MockErrorTransport {
        status: 401,
        body: "unauthorized".to_string(),
    };
    let client = NatlasClient::new(config(), t, "tester");
    let inv = client
        .invoke(&NatlasRequest::new("do a thing", "system"))
        .await;
    assert!(!inv.succeeded());
    assert!(inv.response.is_none());
    let state = NatlasResilienceState::from_error(inv.error.as_ref().unwrap());
    assert_eq!(state, NatlasResilienceState::AuthFailure);
    assert!(inv.evidence.error_code.as_deref() == Some("HTTP_STATUS"));
}

// ---------------------------------------------------------------------------
// Evidence redaction: secrets never land in the evidence record
// ---------------------------------------------------------------------------

#[test]
fn evidence_redacts_secrets_from_error_detail() {
    let err = NatlasError::Transport {
        message: "connect failed with key sk-the-real-secret".to_string(),
    };
    let ev = NatlasEvidence::for_failure(
        "NCAIR1/N-ATLaS",
        "general",
        &err,
        12,
        100,
        &["sk-the-real-secret"],
    );
    let detail = ev.error_detail.unwrap();
    assert!(!detail.contains("sk-the-real-secret"));
    assert_eq!(ev.secret_redactions, 1);
    assert!(!ev.success);
}

#[test]
fn redaction_helper_counts_and_scrubs() {
    let (out, n) = redact_secrets(
        "token=abcDEFsecret and abcDEFsecret again",
        &["abcDEFsecret"],
    );
    assert_eq!(n, 2);
    assert!(!out.contains("abcDEFsecret"));
}

// ---------------------------------------------------------------------------
// Full seam: a canonical response carrying the structured intent flows all the
// way to a real factory execution — without a live model.
// ---------------------------------------------------------------------------

struct MockCanonicalTransport {
    canonical_body: String,
}

#[async_trait::async_trait]
impl NatlasTransport for MockCanonicalTransport {
    async fn send(
        &self,
        _config: &NatlasConfig,
        _request: &NatlasRequest,
    ) -> Result<NatlasRawResponse, NatlasError> {
        Ok(NatlasRawResponse {
            status: 200,
            body: self.canonical_body.clone(),
        })
    }
}

#[tokio::test]
async fn canonical_response_with_intent_drives_real_factory_execution() {
    let structured = r#"{"summary":"write a file and verify","steps":[{"kind":"approve","description":"owner sign-off"},{"kind":"implement","description":"write f.txt","path":"f.txt","content":"data\n"},{"kind":"test","description":"verify","command":["echo","ok"],"verify":true}]}"#;
    let canonical =
        format!("{{\"text\":{},\"model\":\"NCAIR1/N-ATLaS\",\"request_id\":\"req-1\",\"finish_reason\":\"stop\",\"usage\":{{\"input_tokens\":10,\"output_tokens\":20}}}}",
            serde_json::to_string(structured).unwrap());

    let t = MockCanonicalTransport {
        canonical_body: canonical,
    };
    let client = NatlasClient::new(config(), t, "tester");
    let inv = client
        .invoke(&NatlasRequest::new("create f.txt and verify it", "system"))
        .await;
    assert!(inv.succeeded(), "canonical success expected: {inv:?}");
    let response = inv.response.as_ref().unwrap();

    // The model identity must come from the response, not be assumed.
    assert_eq!(response.model, "NCAIR1/N-ATLaS");

    // Parse the structured intent out of the model text and run the factory.
    let intent = NatlasEngineeringIntent::parse(&response.text).unwrap();
    let e = env();
    let job = job_from_intent(&intent, &e.workdir);
    let mut runner = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job,
        actor: "tester".to_string(),
        max_steps: 16,
        runtime: permissive_runtime(&e.root),
    })
    .unwrap();

    // Gate first.
    let first = runner.run().await.unwrap();
    assert_eq!(first.status, RunStatus::AwaitingApproval);
    runner.approve("step-01", "owner").unwrap();
    let second = runner.run().await.unwrap();

    assert_eq!(second.status, RunStatus::Completed);
    assert!(e.workdir.join("f.txt").exists(), "real file write happened");
    assert!(second.summary.succeeded >= 3);
}
