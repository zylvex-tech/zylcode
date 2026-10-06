//! End-to-end test for the software-factory vertical slice.
//!
//! Proves the directive's first implementation target with real execution:
//!
//! ```text
//! FACTORY JOB
//!   → DURABLE TASK GRAPH
//!   → REPOSITORY CONTEXT (a real file the graph reads back)
//!   → ONE BOUNDED IMPLEMENTATION TASK (a real file write)
//!   → REAL TOOL EXECUTION (through the gated dispatch path)
//!   → TEST (a real command whose exit code is checked)
//!   → EVIDENCE RECORD (evidence graph + claims)
//!   → HUMAN REVIEW (a real approval gate)
//!   → DURABLE COMPLETION STATE
//! ```
//!
//! Every assertion below is about something that actually happened on disk or in
//! a persisted store. Nothing asserts that a model said something, because no
//! model is reachable in this environment — and the slice is designed so that
//! fact does not weaken it.

use std::path::Path;
use std::sync::Arc;

use zylcode_core::factory::{
    FactoryJob, FactoryRunner, FactoryRunnerConfig, JobStage, JobStore, RunStatus, TaskAction,
    TaskNode, TaskState, BLOCKED_PROVIDER,
};
use zylcode_core::factory::graph::TaskKind;
use zylcode_mcp::{JsonlEvidenceSink, PermissionGate, PermissionPolicy, RiskLevel, ToolRuntime};

/// A runtime that permits read/write/execute and records evidence to disk.
fn permissive_runtime(dir: &Path) -> ToolRuntime {
    let gate = Arc::new(PermissionGate::new(PermissionPolicy::allow_up_to(
        RiskLevel::Execute,
    )));
    let sink = Arc::new(JsonlEvidenceSink::new(dir.join("evidence.jsonl")));
    ToolRuntime::new(gate, sink)
}

/// A runtime whose policy denies anything above a read.
fn read_only_runtime(dir: &Path) -> ToolRuntime {
    let gate = Arc::new(PermissionGate::new(PermissionPolicy::allow_up_to(
        RiskLevel::Read,
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

fn job(nodes: Vec<TaskNode>) -> FactoryJob {
    let mut job = FactoryJob::new(
        "establish the factory vertical slice",
        "proj-zylcode",
        "https://github.com/zylvex-tech/zylcode",
        "main",
        "/worktree",
        "test-harness",
    );
    for n in nodes {
        job.graph.add_node(n).unwrap();
    }
    job
}

fn note(id: &str, kind: TaskKind) -> TaskNode {
    TaskNode::new(
        id,
        kind,
        format!("task {id}"),
        TaskAction::Note {
            text: format!("{id} complete"),
        },
    )
}

/// The full slice: requirements → implementation (real write) → read back →
/// verify (real command) → documentation. Ends COMPLETED with evidence.
#[tokio::test]
async fn the_slice_executes_in_dependency_order_with_real_tools_and_evidence() {
    let e = env();
    let job = job(vec![
        note("req", TaskKind::Requirements),
        TaskNode::new(
            "impl",
            TaskKind::Implementation,
            "write the module",
            TaskAction::WriteFile {
                path: "lib.rs".to_string(),
                content: "pub fn value() -> u32 { 42 }\n".to_string(),
            },
        )
        .depends_on(&["req"]),
        TaskNode::new(
            "read_back",
            TaskKind::RepositoryContext,
            "read the module back",
            TaskAction::ReadFile {
                path: "lib.rs".to_string(),
            },
        )
        .depends_on(&["impl"]),
        TaskNode::new(
            "verify",
            TaskKind::Test,
            "run the verification command",
            TaskAction::Verify {
                command: "echo".to_string(),
                args: vec!["ok".to_string()],
            },
        )
        .depends_on(&["read_back"]),
        note("doc", TaskKind::Documentation).depends_on(&["verify"]),
    ]);

    let mut runner = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job: job.clone(),
        actor: "factory-test".to_string(),
        max_steps: 64,
        runtime: permissive_runtime(&e.root),
    })
    .unwrap();

    let report = runner.run().await.unwrap();

    // 1. The job completed, and every task reached a good terminal state.
    assert_eq!(report.status, RunStatus::Completed, "{:?}", report.blocked);
    assert!(runner.job.graph.is_complete());
    assert_eq!(report.summary.succeeded, 5);
    assert_eq!(report.summary.failed, 0);
    assert_eq!(report.summary.blocked, 0);

    // 2. Execution order respected the dependency chain.
    assert_eq!(
        report.executed,
        vec!["req", "impl", "read_back", "verify", "doc"]
    );

    // 3. The implementation task really wrote the file.
    let written = std::fs::read_to_string(e.workdir.join("lib.rs")).unwrap();
    assert!(written.contains("fn value()"), "{written}");

    // 4. The verification task produced a VERIFIED claim, because a real command
    //    exited as expected.
    let claims = runner.claim_store().list().unwrap();
    let verified: Vec<_> = claims
        .iter()
        .filter(|c| c.status == zylcode_core::claim::ClaimStatus::Verified)
        .collect();
    assert!(
        verified
            .iter()
            .any(|c| c.statement.contains("exited 0")),
        "a real exit-0 command must yield a verified test claim: {:?}",
        claims.iter().map(|c| &c.statement).collect::<Vec<_>>()
    );

    // 5. The evidence graph is intact and traces back to the intent.
    assert!(report.graph_integrity, "evidence graph must verify");
    assert!(report.claims_integrity, "claim chain must verify");
    let doc = runner.graph_store().document().unwrap();
    assert!(
        doc.nodes
            .iter()
            .any(|n| n.kind == zylcode_core::evidence_graph::NodeKind::Intent),
        "the graph must carry the originating INTENT node"
    );
    let last = doc.nodes.last().unwrap();
    let ancestry = runner.graph_store().ancestry(&last.id).unwrap();
    assert_eq!(
        ancestry.last().unwrap().kind,
        zylcode_core::evidence_graph::NodeKind::Intent,
        "the last node must trace back to the intent"
    );

    // 6. The stage advanced to DELIVERY because the graph is complete.
    assert_eq!(report.stage, JobStage::Delivery);

    // 7. The durable document on disk reflects the completed job.
    let store = JobStore::open(&e.root).unwrap();
    let persisted = store.load(&job.id).unwrap();
    assert!(persisted.graph.is_complete());
    assert_eq!(persisted.stage, JobStage::Delivery);
}

/// A task that requires a model is BLOCKED, with a machine-readable reason. It is
/// never simulated, and it produces no claim.
#[tokio::test]
async fn a_model_task_is_blocked_provider_and_never_faked() {
    let e = env();
    let job = job(vec![TaskNode::new(
        "design",
        TaskKind::Architecture,
        "design the auth module",
        TaskAction::ModelTask {
            prompt: "Design an authentication module.".to_string(),
        },
    )]);

    let mut runner = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job,
        actor: "factory-test".to_string(),
        max_steps: 8,
        runtime: permissive_runtime(&e.root),
    })
    .unwrap();

    let report = runner.run().await.unwrap();

    assert_eq!(report.status, RunStatus::Blocked);
    let blocked = runner.job.graph.get("design").unwrap();
    assert_eq!(blocked.state, TaskState::Blocked);
    assert_eq!(blocked.blocked_reason.as_deref(), Some(BLOCKED_PROVIDER));
    // No claim was invented for work that did not happen.
    assert!(
        runner.claim_store().list().unwrap().is_empty(),
        "a blocked task must not produce a claim"
    );
    assert_eq!(report.summary.succeeded, 0);
}

/// A human gate parks the job; approving resumes it and it completes.
#[tokio::test]
async fn a_human_approval_gate_parks_and_then_resumes() {
    let e = env();
    let job = job(vec![
        note("impl", TaskKind::Implementation),
        TaskNode::new(
            "approve",
            TaskKind::HumanApproval,
            "supervisor sign-off",
            TaskAction::Approval {
                reason: "release requires supervisor approval".to_string(),
            },
        )
        .depends_on(&["impl"]),
    ]);

    let mut runner = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job: job.clone(),
        actor: "factory-test".to_string(),
        max_steps: 8,
        runtime: permissive_runtime(&e.root),
    })
    .unwrap();

    let first = runner.run().await.unwrap();
    assert_eq!(first.status, RunStatus::AwaitingApproval);
    assert_eq!(
        runner.job.graph.get("approve").unwrap().state,
        TaskState::AwaitingApproval
    );
    assert!(!runner.job.graph.is_complete());

    // Approving the wrong task is refused.
    assert!(runner.approve("impl", "supervisor").is_err());

    runner.approve("approve", "supervisor").unwrap();
    let second = runner.run().await.unwrap();
    assert_eq!(second.status, RunStatus::Completed);
    assert_eq!(
        runner.job.graph.get("approve").unwrap().approved_by.as_deref(),
        Some("supervisor")
    );
}

/// A task whose dependency failed is cancelled explicitly — never left pending.
#[tokio::test]
async fn a_failed_dependency_cancels_its_dependents() {
    let e = env();
    let job = job(vec![
        TaskNode::new(
            "bad",
            TaskKind::Test,
            "command that will not meet its expectation",
            TaskAction::RunCommand {
                command: "echo".to_string(),
                args: vec!["x".to_string()],
                // `echo` exits 0; we demand 7, so this must fail.
                expect_exit: 7,
            },
        ),
        note("after", TaskKind::Documentation).depends_on(&["bad"]),
    ]);

    let mut runner = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job,
        actor: "factory-test".to_string(),
        max_steps: 8,
        runtime: permissive_runtime(&e.root),
    })
    .unwrap();

    let report = runner.run().await.unwrap();

    assert_eq!(report.status, RunStatus::Blocked);
    assert_eq!(runner.job.graph.get("bad").unwrap().state, TaskState::Failed);
    assert_eq!(
        runner.job.graph.get("after").unwrap().state,
        TaskState::Cancelled,
        "a dependent of a failed task must be cancelled, not left pending"
    );
    assert_eq!(report.cancelled, vec!["after".to_string()]);

    // The failure was recorded with a real cause.
    assert_eq!(report.failures.len(), 1);
    assert!(report.failures[0].cause.contains("expected exit 7"));

    // The unmet expectation produced a CONTRADICTED claim, not a silent drop.
    let claims = runner.claim_store().list().unwrap();
    assert!(
        claims
            .iter()
            .any(|c| c.status == zylcode_core::claim::ClaimStatus::Contradicted),
        "the failed expectation must be recorded as a contradicted claim"
    );
}

/// A write the policy does not permit parks for approval; approving re-dispatches
/// with approval set and the write lands.
#[tokio::test]
async fn a_policy_refusal_parks_for_approval_then_executes_after_approval() {
    let e = env();
    let job = job(vec![TaskNode::new(
        "write",
        TaskKind::Implementation,
        "write a file the policy does not permit by default",
        TaskAction::WriteFile {
            path: "guarded.txt".to_string(),
            content: "approved content\n".to_string(),
        },
    )]);

    let mut runner = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job,
        actor: "factory-test".to_string(),
        max_steps: 8,
        runtime: read_only_runtime(&e.root),
    })
    .unwrap();

    let first = runner.run().await.unwrap();
    assert_eq!(first.status, RunStatus::AwaitingApproval);
    assert!(
        !e.workdir.join("guarded.txt").exists(),
        "the file must not exist before approval"
    );

    runner.approve("write", "operator").unwrap();
    let second = runner.run().await.unwrap();
    assert_eq!(second.status, RunStatus::Completed);
    assert_eq!(
        std::fs::read_to_string(e.workdir.join("guarded.txt")).unwrap(),
        "approved content\n"
    );
}

/// A job stopped by its step budget resumes from the durable document, and the
/// work already done is not repeated.
#[tokio::test]
async fn a_job_resumes_from_the_store_and_does_not_repeat_completed_work() {
    let e = env();
    let job = job(vec![
        note("a", TaskKind::Requirements),
        note("b", TaskKind::Architecture).depends_on(&["a"]),
        note("c", TaskKind::Documentation).depends_on(&["b"]),
    ]);

    // First run: one step only, so the graph is deliberately left incomplete.
    let mut first_runner = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job: job.clone(),
        actor: "factory-test".to_string(),
        max_steps: 1,
        runtime: permissive_runtime(&e.root),
    })
    .unwrap();
    let first = first_runner.run().await.unwrap();
    assert_eq!(first.status, RunStatus::StepLimit);
    assert_eq!(first.executed, vec!["a".to_string()]);
    assert!(!first_runner.job.graph.is_complete());
    drop(first_runner);

    // Second run: a *new* runner, loaded from the durable store, exactly as a
    // restarted process would.
    let store = JobStore::open(&e.root).unwrap();
    let reloaded = store.load(&job.id).unwrap();
    assert_eq!(reloaded.graph.get("a").unwrap().state, TaskState::Succeeded);

    let mut second_runner = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job: reloaded,
        actor: "factory-test".to_string(),
        max_steps: 8,
        runtime: permissive_runtime(&e.root),
    })
    .unwrap();
    let second = second_runner.run().await.unwrap();

    assert_eq!(second.status, RunStatus::Completed);
    // `a` was not re-executed; only the remaining work ran.
    assert_eq!(second.executed, vec!["b".to_string(), "c".to_string()]);
    assert_eq!(
        second_runner.job.graph.get("a").unwrap().attempts,
        1,
        "a completed task must not be attempted again on resume"
    );
}

/// The graph refuses to run when it is structurally unsound.
#[tokio::test]
async fn an_invalid_graph_is_refused_before_anything_runs() {
    let e = env();
    let mut job = job(vec![note("a", TaskKind::Requirements)]);
    // Bypass the builder to plant a dangling dependency directly.
    job.graph.get_mut("a").unwrap().depends_on = vec!["ghost".to_string()];

    let result = FactoryRunner::open(FactoryRunnerConfig {
        root: e.root.clone(),
        workdir: e.workdir.clone(),
        job,
        actor: "factory-test".to_string(),
        max_steps: 8,
        runtime: permissive_runtime(&e.root),
    });
    assert!(result.is_err(), "an invalid graph must not open");
}
