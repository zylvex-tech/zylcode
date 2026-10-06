//! The factory runner — directive Phase 3 / Phase 4 / Phase 12 / Phase 20.
//!
//! [`FactoryRunner`] walks a [`TaskGraph`] in dependency order and executes each
//! ready task through the **real** tool runtime. It is the piece that turns a
//! durable plan into executed work with evidence.
//!
//! # The five rules this runner obeys
//!
//! 1. **A task is executed through a real tool or not at all.** Every tool-backed
//!    action goes through [`zylcode_mcp::dispatch`], which consults the
//!    permission gate *before* the executor and records the refusal when it
//!    refuses. There is no bypass.
//! 2. **Success is established by exit code and captured output, never by
//!    assertion.** A `RunCommand` that expects exit 0 but sees exit 1 is
//!    [`TaskState::Failed`]; the claim it produced is `CONTRADICTED`, not
//!    quietly dropped.
//! 3. **Model work is blocked, not faked.** A [`TaskAction::ModelTask`] is
//!    [`TaskState::Blocked`] with reason [`BLOCKED_PROVIDER`] while no approved
//!    provider exists. The deterministic orchestration is proven on its own.
//! 4. **Progress is durable.** The job is saved after every state transition, so
//!    a process that dies mid-graph resumes from the last recorded truth rather
//!    than re-deriving it.
//! 5. **Failure propagates.** A task whose dependency reached a terminal bad
//!    state is cancelled explicitly. It is never left `Pending` forever, and it
//!    is never silently promoted to `Ready`.
//!
//! # Execution order and the parallel question
//!
//! The graph records real dependencies, so *parallel-safe* tasks are
//! distinguishable from serial ones. This runner executes ready tasks
//! **sequentially in declaration order** and says so. It does not claim
//! concurrency it does not perform. Parallel execution over isolated worktrees is
//! a designed extension (see `ZYLCODE_FACTORY_EXECUTION_MODEL_2026-10-06.md`),
//! not a property of this slice.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::claim::{
    Claim, ClaimKind, ClaimSource, ClaimStore, EvidenceKind, VerificationLevel,
};
use crate::evidence_graph::{EdgeKind, EvidenceGraph, EvidenceNode, NodeKind};
use crate::failure::{Failure, FailureStatus};
use zylcode_mcp::{dispatch, PermissionDecision, RiskLevel, ToolContext, ToolRuntime};

use super::graph::{GraphSummary, TaskAction, TaskNode, TaskState};
use super::job::{FactoryJob, JobStage};
use super::store::JobStore;

/// Reason recorded when a task requires a model and no approved provider exists.
///
/// A constant, not a formatted string, so a report can grep for it and a test can
/// assert it. This is the directive's `BLOCKED_PROVIDER` transition.
pub const BLOCKED_PROVIDER: &str = "BLOCKED_PROVIDER";

/// Default tool timeout for one factory task.
pub const DEFAULT_TASK_TIMEOUT_SECS: u64 = 600;

/// The factory's per-job state directory: `<root>/.zylcode/factory`.
pub fn factory_dir(root: &Path) -> PathBuf {
    root.join(".zylcode").join("factory")
}

/// Per-job evidence graph path.
pub fn job_graph_path(root: &Path, job_id: &str) -> PathBuf {
    factory_dir(root).join(format!("{job_id}.graph.json"))
}

/// Per-job claim-store path.
pub fn job_claims_path(root: &Path, job_id: &str) -> PathBuf {
    factory_dir(root).join(format!("{job_id}.claims.json"))
}

/// Per-job failure-record path.
pub fn job_failures_path(root: &Path, job_id: &str) -> PathBuf {
    factory_dir(root).join(format!("{job_id}.failures.json"))
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// Every task reached a terminal good state.
    Completed,
    /// At least one task is blocked (including `BLOCKED_PROVIDER`).
    Blocked,
    /// At least one task is parked at a human gate.
    AwaitingApproval,
    /// Nothing can progress and nothing is complete.
    Stalled,
    /// The step budget was exhausted before the graph settled.
    StepLimit,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RunStatus::Completed => "COMPLETED",
            RunStatus::Blocked => "BLOCKED",
            RunStatus::AwaitingApproval => "AWAITING_APPROVAL",
            RunStatus::Stalled => "STALLED",
            RunStatus::StepLimit => "STEP_LIMIT",
        }
    }
}

/// What one `run()` produced. Serialisable so it can be attached to a report.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunReport {
    pub job_id: String,
    pub status: RunStatus,
    pub stage: JobStage,
    pub summary: GraphSummary,
    /// Task ids executed, in order.
    pub executed: Vec<String>,
    /// Task ids cancelled by dependency propagation.
    pub cancelled: Vec<String>,
    /// Blocked tasks and their reasons.
    pub blocked: Vec<(String, String)>,
    /// Tasks parked at a human gate.
    pub awaiting_approval: Vec<String>,
    /// Claim ids recorded, with their status, for the truth surface.
    pub claims: Vec<(String, String, String)>,
    /// Failure records produced.
    pub failures: Vec<Failure>,
    /// Integrity of the job's evidence graph and claim chain.
    pub graph_integrity: bool,
    pub claims_integrity: bool,
}

/// Everything a runner needs to open a job.
pub struct FactoryRunnerConfig {
    /// Workspace root: where job documents and per-job evidence live.
    pub root: PathBuf,
    /// Directory the tools execute in. Usually the mission workspace.
    pub workdir: PathBuf,
    /// The job to run. Its graph must validate.
    pub job: FactoryJob,
    /// Actor attributed to every tool invocation.
    pub actor: String,
    /// Upper bound on task executions in one `run()` call.
    pub max_steps: usize,
    /// The gated tool runtime. The caller chooses the policy deliberately.
    pub runtime: ToolRuntime,
}

/// The durable, evidence-producing factory runner.
pub struct FactoryRunner {
    pub job: FactoryJob,
    root: PathBuf,
    workdir: PathBuf,
    store: JobStore,
    graph: EvidenceGraph,
    claims: ClaimStore,
    runtime: ToolRuntime,
    actor: String,
    max_steps: usize,
    failures: Vec<Failure>,
    /// Last evidence-graph node id, so the spine survives a resume.
    spine: Option<String>,
    session_id: String,
}

/// Internal per-node outcome.
enum NodeOutcome {
    Succeeded(String),
    Failed(String),
    Blocked(String, String),
    AwaitingApproval(String),
}

impl FactoryRunner {
    /// Open (or resume) a job.
    ///
    /// On a fresh job this seeds the `INTENT` node of the evidence graph. On a
    /// resume it re-reads the persisted graph and continues the spine, so the
    /// provenance chain is unbroken across a restart.
    pub fn open(cfg: FactoryRunnerConfig) -> Result<Self> {
        cfg.job
            .graph
            .validate()
            .context("factory job graph is invalid; refusing to run it")?;

        let store = JobStore::open(&cfg.root)?;
        let graph = EvidenceGraph::with_path(job_graph_path(&cfg.root, &cfg.job.id));
        let claims = ClaimStore::with_path(job_claims_path(&cfg.root, &cfg.job.id));

        // Failures are persisted so a resume does not lose the record of what
        // already went wrong.
        let failures_path = job_failures_path(&cfg.root, &cfg.job.id);
        let failures: Vec<Failure> = if failures_path.exists() {
            let raw = std::fs::read_to_string(&failures_path)?;
            if raw.trim().is_empty() {
                Vec::new()
            } else {
                serde_json::from_str(&raw).unwrap_or_default()
            }
        } else {
            Vec::new()
        };

        // Continue the spine from whatever is already recorded.
        let spine = graph
            .document()
            .ok()
            .and_then(|doc| doc.nodes.last().map(|n| n.id.clone()));

        let mut runner = FactoryRunner {
            job: cfg.job,
            root: cfg.root,
            workdir: cfg.workdir,
            store,
            graph,
            claims,
            runtime: cfg.runtime,
            actor: cfg.actor,
            max_steps: cfg.max_steps.max(1),
            failures,
            spine,
            session_id: uuid::Uuid::new_v4().to_string(),
        };

        // Seed the INTENT node exactly once, so every later node can be traced
        // back to the originating intent (Phase 8).
        if runner.spine.is_none() {
            let intent_node = EvidenceNode::new(
                NodeKind::Intent,
                format!("factory job {}: {}", &runner.job.id[..8.min(runner.job.id.len())], runner.job.intent),
                runner.actor.clone(),
            )
            .with_commit(runner.job.branch.clone());
            let stored = runner.graph.add_node(intent_node)?;
            runner.spine = Some(stored.id);
        }

        runner.store.save(&runner.job)?;
        Ok(runner)
    }

    pub fn graph_store(&self) -> &EvidenceGraph {
        &self.graph
    }

    pub fn claim_store(&self) -> &ClaimStore {
        &self.claims
    }

    pub fn failures(&self) -> &[Failure] {
        &self.failures
    }

    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    /// Approve a task parked at a human gate. Returns an error if the task is not
    /// actually awaiting approval — an approval that silently applies to the
    /// wrong task is worse than a refusal.
    pub fn approve(&mut self, task_id: &str, approver: &str) -> Result<()> {
        let node = self
            .job
            .graph
            .get_mut(task_id)
            .with_context(|| format!("unknown task {task_id}"))?;
        anyhow::ensure!(
            node.state == TaskState::AwaitingApproval,
            "task {task_id} is {}, not awaiting approval",
            node.state.as_str()
        );
        node.approved_by = Some(approver.to_string());
        node.state = TaskState::Pending;
        node.outcome = Some(format!("approved by {approver}"));
        self.store.save(&self.job)?;
        Ok(())
    }

    /// Explicitly cancel a task.
    pub fn cancel(&mut self, task_id: &str, reason: &str) -> Result<()> {
        let node = self
            .job
            .graph
            .get_mut(task_id)
            .with_context(|| format!("unknown task {task_id}"))?;
        node.state = TaskState::Cancelled;
        node.outcome = Some(format!("cancelled: {reason}"));
        node.finished_at = Some(chrono::Utc::now().to_rfc3339());
        self.store.save(&self.job)?;
        Ok(())
    }

    /// Mark a task blocked from outside the runner (e.g. a provider outage
    /// discovered by a caller).
    pub fn block(&mut self, task_id: &str, reason: &str) -> Result<()> {
        let node = self
            .job
            .graph
            .get_mut(task_id)
            .with_context(|| format!("unknown task {task_id}"))?;
        node.state = TaskState::Blocked;
        node.blocked_reason = Some(reason.to_string());
        node.finished_at = Some(chrono::Utc::now().to_rfc3339());
        self.store.save(&self.job)?;
        Ok(())
    }

    /// Execute the graph until it settles, a human gate is reached, or the step
    /// budget is exhausted. Returns a report of what actually happened.
    pub async fn run(&mut self) -> Result<RunReport> {
        let mut executed = Vec::new();
        let mut cancelled = Vec::new();
        let mut steps = 0usize;

        let status = loop {
            if self.job.graph.is_complete() {
                break RunStatus::Completed;
            }

            // 1. Propagate failure: cancel anything whose dependency can never
            //    succeed. Explicit, so it is never left Pending forever.
            for id in self.job.graph.orphaned() {
                self.cancel(&id, "a dependency reached a terminal failure state")?;
                cancelled.push(id);
            }

            // 2. Find the next task that may run.
            let ready = self.job.graph.ready();
            if ready.is_empty() {
                // Nothing runnable: either a human gate, a block, or a stall.
                break if !self.job.graph.awaiting_approval().is_empty() {
                    RunStatus::AwaitingApproval
                } else if !self.job.graph.blocked().is_empty()
                    || self
                        .job
                        .graph
                        .nodes()
                        .iter()
                        .any(|n| n.state == TaskState::Failed)
                {
                    RunStatus::Blocked
                } else {
                    RunStatus::Stalled
                };
            }

            if steps >= self.max_steps {
                break RunStatus::StepLimit;
            }

            let id = ready[0].clone();
            self.execute_node(&id).await?;
            executed.push(id);
            self.job.recompute_stage();
            self.store.save(&self.job)?;
            steps += 1;
        };

        self.job.recompute_stage();
        self.store.save(&self.job)?;
        self.persist_failures()?;

        Ok(RunReport {
            job_id: self.job.id.clone(),
            status,
            stage: self.job.stage,
            summary: self.job.graph.summary(),
            executed,
            cancelled,
            blocked: self
                .job
                .graph
                .blocked()
                .iter()
                .map(|n| (n.id.clone(), n.blocked_reason.clone().unwrap_or_default()))
                .collect(),
            awaiting_approval: self
                .job
                .graph
                .awaiting_approval()
                .iter()
                .map(|n| n.id.clone())
                .collect(),
            claims: self
                .claims
                .list()
                .unwrap_or_default()
                .into_iter()
                .map(|c| (c.id, c.statement, format!("{:?}", c.status)))
                .collect(),
            failures: self.failures.clone(),
            graph_integrity: self.graph.verify_integrity().unwrap_or(false),
            claims_integrity: self.claims.verify_chain().unwrap_or(false),
        })
    }

    // -----------------------------------------------------------------------
    // Node execution
    // -----------------------------------------------------------------------

    async fn execute_node(&mut self, id: &str) -> Result<()> {
        let node = self
            .job
            .graph
            .get(id)
            .cloned()
            .with_context(|| format!("unknown task {id}"))?;

        // Record that execution started, before it does.
        {
            let n = self.job.graph.get_mut(id).unwrap();
            n.state = TaskState::Running;
            n.attempts += 1;
        }
        self.store.save(&self.job)?;

        let outcome = match &node.action {
            TaskAction::Note { text } => NodeOutcome::Succeeded(format!("note: {text}")),
            TaskAction::ModelTask { prompt } => NodeOutcome::Blocked(
                BLOCKED_PROVIDER.to_string(),
                format!(
                    "task requires a model ({}) but no approved provider is available; \
                     AGENT-01 remains blocked and this transition is not simulated",
                    truncate(prompt, 80)
                ),
            ),
            TaskAction::Approval { reason } => {
                if node.approved_by.is_some() {
                    NodeOutcome::Succeeded(format!(
                        "approved by {}",
                        node.approved_by.clone().unwrap_or_default()
                    ))
                } else {
                    NodeOutcome::AwaitingApproval(reason.clone())
                }
            }
            TaskAction::ReadFile { path } => {
                self.dispatch_tool(&node, "fs.read", serde_json::json!({ "path": path }))
                    .await
            }
            TaskAction::WriteFile { path, content } => {
                self.dispatch_tool(
                    &node,
                    "fs.write",
                    serde_json::json!({ "path": path, "content": content }),
                )
                .await
            }
            TaskAction::RunCommand {
                command,
                args,
                expect_exit,
            } => {
                self.dispatch_command(&node, command, args, Some(*expect_exit))
                    .await
            }
            TaskAction::Verify { command, args } => {
                self.dispatch_command(&node, command, args, Some(0)).await
            }
        };

        let finished = chrono::Utc::now().to_rfc3339();
        let n = self.job.graph.get_mut(id).unwrap();
        match outcome {
            NodeOutcome::Succeeded(msg) => {
                n.state = TaskState::Succeeded;
                n.outcome = Some(msg);
                n.blocked_reason = None;
                n.finished_at = Some(finished);
            }
            NodeOutcome::Failed(msg) => {
                n.state = TaskState::Failed;
                n.outcome = Some(msg);
                n.finished_at = Some(finished);
            }
            NodeOutcome::Blocked(reason, msg) => {
                n.state = TaskState::Blocked;
                n.blocked_reason = Some(reason);
                n.outcome = Some(msg);
                n.finished_at = Some(finished);
            }
            NodeOutcome::AwaitingApproval(reason) => {
                n.state = TaskState::AwaitingApproval;
                n.outcome = Some(format!("awaiting human approval: {reason}"));
            }
        }
        self.store.save(&self.job)?;
        Ok(())
    }

    /// Dispatch a tool-backed action through the gated runtime and record the
    /// outcome as evidence.
    async fn dispatch_tool(
        &mut self,
        node: &TaskNode,
        tool_id: &str,
        params: serde_json::Value,
    ) -> NodeOutcome {
        let ctx = self.tool_context(node.approved_by.is_some());
        let outcome = dispatch(tool_id, params.clone(), &ctx, &self.runtime).await;

        // The gate's decision is recorded either way. A refusal is evidence.
        let decision_str = outcome.decision.evidence_record();
        match &outcome.decision {
            PermissionDecision::Deny(reason) => {
                self.record_evidence(
                    node,
                    NodeKind::ToolCall,
                    format!("{tool_id} denied by permission gate"),
                    tool_id,
                    Some(params.clone()),
                    None,
                    Some(decision_str.clone()),
                );
                return NodeOutcome::Blocked(
                    "PERMISSION_DENIED".to_string(),
                    format!("permission gate denied {tool_id}: {reason}"),
                );
            }
            PermissionDecision::RequireApproval(reason) => {
                self.record_evidence(
                    node,
                    NodeKind::ToolCall,
                    format!("{tool_id} requires human approval"),
                    tool_id,
                    Some(params.clone()),
                    None,
                    Some(decision_str.clone()),
                );
                return NodeOutcome::AwaitingApproval(format!(
                    "{tool_id} requires human approval: {reason}"
                ));
            }
            PermissionDecision::Allow(_) => {}
        }

        match outcome.result {
            Ok(result) => {
                let invocation = outcome.evidence.invocation_id.clone();
                let summary = summarize_tool_output(&result.output);
                let evidence_id = self.record_evidence(
                    node,
                    NodeKind::Result,
                    format!("{tool_id} executed: {}", truncate(&summary, 160)),
                    tool_id,
                    Some(params.clone()),
                    Some(result.output.clone()),
                    Some(decision_str),
                );
                // A tool that reports success=false is not a success, whatever
                // the process exit code said.
                if !result.success {
                    self.record_failure(
                        node,
                        format!("{tool_id} reported failure"),
                        FailureStatus::Failed,
                        &summary,
                    );
                    return NodeOutcome::Failed(format!("{tool_id} reported failure: {summary}"));
                }
                self.record_claim(
                    node,
                    format!("{} completed: {}", tool_id, truncate(&summary, 120)),
                    ClaimKind::Process,
                    VerificationLevel::R1StructuralValidity,
                    &invocation,
                    false,
                );
                let _ = evidence_id;
                NodeOutcome::Succeeded(summary)
            }
            Err(e) => {
                let msg = e.to_string();
                self.record_failure(node, format!("{tool_id} failed"), FailureStatus::Failed, &msg);
                NodeOutcome::Failed(format!("{tool_id} failed: {msg}"))
            }
        }
    }

    /// Run a command through `shell.execute` and check its exit code against the
    /// expectation. This is where a test claim is earned or contradicted.
    async fn dispatch_command(
        &mut self,
        node: &TaskNode,
        command: &str,
        args: &[String],
        expect_exit: Option<i32>,
    ) -> NodeOutcome {
        let params = serde_json::json!({ "command": command, "args": args });
        let ctx = self.tool_context(node.approved_by.is_some());
        let outcome = dispatch("shell.execute", params.clone(), &ctx, &self.runtime).await;
        let decision_str = outcome.decision.evidence_record();

        match &outcome.decision {
            PermissionDecision::Deny(reason) => {
                self.record_evidence(
                    node,
                    NodeKind::ToolCall,
                    format!("shell.execute denied: {command}"),
                    "shell.execute",
                    Some(params.clone()),
                    None,
                    Some(decision_str),
                );
                return NodeOutcome::Blocked(
                    "PERMISSION_DENIED".to_string(),
                    format!("permission gate denied shell.execute: {reason}"),
                );
            }
            PermissionDecision::RequireApproval(reason) => {
                self.record_evidence(
                    node,
                    NodeKind::ToolCall,
                    format!("shell.execute requires approval: {command}"),
                    "shell.execute",
                    Some(params.clone()),
                    None,
                    Some(decision_str),
                );
                return NodeOutcome::AwaitingApproval(format!(
                    "shell.execute requires human approval: {reason}"
                ));
            }
            PermissionDecision::Allow(_) => {}
        }

        let result = match outcome.result {
            Ok(r) => r,
            Err(e) => {
                let msg = e.to_string();
                self.record_failure(
                    node,
                    format!("{command} failed to execute"),
                    FailureStatus::Failed,
                    &msg,
                );
                return NodeOutcome::Failed(format!("{command} failed to execute: {msg}"));
            }
        };

        let invocation = outcome.evidence.invocation_id.clone();
        let exit_code = result
            .output
            .get("exit_code")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);
        let summary = summarize_tool_output(&result.output);

        self.record_evidence(
            node,
            NodeKind::Verification,
            format!(
                "{command} {} -> exit {:?}",
                args.join(" "),
                exit_code.unwrap_or(-1)
            ),
            "shell.execute",
            Some(params),
            Some(result.output.clone()),
            Some(decision_str),
        );

        match (expect_exit, exit_code) {
            (Some(expected), Some(actual)) if expected == actual => {
                // A deterministic tool observed the command succeed. That is
                // R3 (test verified) evidence, so the claim may be promoted.
                self.record_claim(
                    node,
                    format!("`{command} {}` exited {actual}", args.join(" ")),
                    ClaimKind::Test,
                    VerificationLevel::R3TestVerified,
                    &invocation,
                    true,
                );
                NodeOutcome::Succeeded(format!("exit {actual} as expected"))
            }
            (Some(expected), Some(actual)) => {
                // The expectation was not met. The claim is contradicted, not
                // dropped — a silent failure is how a factory lies.
                let statement = format!("`{command} {}` exits {expected}", args.join(" "));
                self.record_claim_contradicted(
                    node,
                    statement,
                    ClaimKind::Test,
                    &invocation,
                    format!("observed exit {actual}, expected {expected}"),
                );
                self.record_failure(
                    node,
                    format!("{command} exit mismatch"),
                    FailureStatus::Failed,
                    &format!("expected exit {expected}, observed {actual}"),
                );
                NodeOutcome::Failed(format!("expected exit {expected}, observed {actual}"))
            }
            (_, None) => {
                self.record_failure(
                    node,
                    format!("{command} produced no exit code"),
                    FailureStatus::Unknown,
                    &summary,
                );
                NodeOutcome::Failed(format!("{command} produced no exit code"))
            }
            (None, Some(actual)) => NodeOutcome::Succeeded(format!("exit {actual}")),
        }
    }

    fn tool_context(&self, approved: bool) -> ToolContext {
        ToolContext {
            working_directory: self.workdir.clone(),
            environment: Default::default(),
            timeout: Duration::from_secs(DEFAULT_TASK_TIMEOUT_SECS),
            session_id: Some(self.session_id.clone()),
            actor: Some(self.actor.clone()),
            // A previously approved task re-dispatches with approval set, which
            // is what the permission policy's `approval_required` field is for.
            approval_required: approved,
        }
    }

    // -----------------------------------------------------------------------
    // Evidence recording
    // -----------------------------------------------------------------------

    /// Append a node to the per-job evidence graph, linked to the spine.
    #[allow(clippy::too_many_arguments)]
    fn record_evidence(
        &mut self,
        node: &TaskNode,
        kind: NodeKind,
        summary: String,
        tool: &str,
        inputs: Option<serde_json::Value>,
        outputs: Option<serde_json::Value>,
        verification: Option<String>,
    ) -> Option<String> {
        let mut ev = EvidenceNode::new(kind, summary, self.actor.clone())
            .with_tool(tool)
            .with_commit(self.job.branch.clone());
        ev.inputs = inputs;
        ev.outputs = outputs;
        ev.verification = verification;
        ev.conclusion = Some(format!("task {} ({})", node.id, node.kind.as_str()));
        let stored = match self.graph.add_node(ev) {
            Ok(s) => s,
            Err(_) => return None,
        };
        if let Some(prev) = self.spine.clone() {
            let relation = match kind {
                NodeKind::Result | NodeKind::Artifact => EdgeKind::Produces,
                NodeKind::Verification => EdgeKind::Verifies,
                _ => EdgeKind::DerivesFrom,
            };
            let _ = self.graph.add_edge(&prev, &stored.id, relation, None);
        }
        self.spine = Some(stored.id.clone());
        // Attach the evidence node to the task so a report can traverse it.
        if let Some(n) = self.job.graph.get_mut(&node.id) {
            n.evidence_node_id = Some(stored.id.clone());
        }
        Some(stored.id)
    }

    /// Record a claim. When `promote` is true and the level permits, the claim is
    /// promoted to VERIFIED — which only succeeds because the caller supplied
    /// real evidence.
    fn record_claim(
        &mut self,
        node: &TaskNode,
        statement: String,
        kind: ClaimKind,
        level: VerificationLevel,
        evidence_id: &str,
        promote: bool,
    ) {
        let mut claim = Claim::new(statement, kind, ClaimSource::DeterministicTool, level);
        claim.add_evidence(EvidenceKind::ToolEvidence, evidence_id);
        claim.mission_id = Some(self.job.id.clone());
        if promote {
            if claim.promote_verified().is_err() {
                // Fail-closed: if promotion is refused, the claim stays
                // unverified rather than being forced.
                let _ = claim.observe();
            }
        } else {
            let _ = claim.observe();
        }
        if let Ok(stored) = self.claims.record(claim) {
            if let Some(n) = self.job.graph.get_mut(&node.id) {
                n.claim_ids.push(stored.id);
            }
        }
    }

    fn record_claim_contradicted(
        &mut self,
        node: &TaskNode,
        statement: String,
        kind: ClaimKind,
        evidence_id: &str,
        note: String,
    ) {
        let mut claim = Claim::new(
            statement,
            kind,
            ClaimSource::DeterministicTool,
            VerificationLevel::R3TestVerified,
        );
        claim.add_evidence(EvidenceKind::ToolEvidence, evidence_id);
        claim.mission_id = Some(self.job.id.clone());
        if claim.contradict(note).is_ok() {
            if let Ok(stored) = self.claims.record(claim) {
                if let Some(n) = self.job.graph.get_mut(&node.id) {
                    n.claim_ids.push(stored.id);
                }
            }
        }
    }

    fn record_failure(
        &mut self,
        node: &TaskNode,
        operation: String,
        status: FailureStatus,
        cause: &str,
    ) {
        let mut f = match Failure::new(operation, cause, status) {
            Ok(f) => f,
            Err(_) => return,
        };
        f = f.with_mission(self.job.id.clone());
        if let Some(ev) = self.job.graph.get(&node.id).and_then(|n| n.evidence_node_id.clone()) {
            f.add_evidence(EvidenceKind::Other, ev);
        }
        f.affect_artifact(node.action.describe());
        if let Some(n) = self.job.graph.get_mut(&node.id) {
            n.failure_id = Some(f.id.clone());
        }
        self.failures.push(f);
    }

    fn persist_failures(&self) -> Result<()> {
        let path = job_failures_path(&self.root, &self.job.id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&self.failures)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

/// A short, honest summary of a tool's output. Never invents content.
fn summarize_tool_output(output: &serde_json::Value) -> String {
    if let Some(code) = output.get("exit_code").and_then(|v| v.as_i64()) {
        let stderr = output
            .get("stderr")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let tail = if stderr.is_empty() {
            output
                .get("stdout")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .lines()
                .last()
                .unwrap_or("")
                .to_string()
        } else {
            stderr.lines().last().unwrap_or("").to_string()
        };
        return if tail.is_empty() {
            format!("exit {code}")
        } else {
            format!("exit {code}: {tail}")
        };
    }
    if let Some(size) = output.get("size").and_then(|v| v.as_u64()) {
        return format!(
            "{} {} bytes",
            output.get("action").and_then(|v| v.as_str()).unwrap_or("ok"),
            size
        );
    }
    output.to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

/// The risk ceiling a factory job runs under by default.
///
/// `Write` — a factory job may read and write files, and may not execute
/// arbitrary processes or touch git history without an explicit policy decision.
/// The caller of [`FactoryRunnerConfig`] chooses the runtime and therefore the
/// real ceiling; this constant documents the recommended default so the choice
/// is deliberate rather than incidental.
pub const RECOMMENDED_RISK_CEILING: RiskLevel = RiskLevel::Write;
