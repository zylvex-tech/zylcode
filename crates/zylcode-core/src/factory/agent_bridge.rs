//! The agent → factory seam — Factory × Agent unification (Phase C).
//!
//! For a while two execution models coexisted without touching: the agent loop
//! (plan → tool → verify, in-process, in-memory plan) and the software factory
//! (durable [`TaskGraph`] DAG, approval gates, persisted evidence). The audit
//! called this a semantic collision. This module is the seam that resolves it:
//!
//! * **The agent plans.** A structured engineering intent — the same
//!   [`NatlasEngineeringIntent`] contract the N-ATLAS round trip produces — is
//!   converted into a validated [`TaskGraph`] by the already-tested
//!   [`NatlasEngineeringIntent::to_task_graph`] and wrapped in a durable
//!   [`FactoryJob`].
//! * **The factory executes.** The job runs through [`FactoryRunner`]: real
//!   gated tool dispatch, persisted evidence graph + claim chain, dependency
//!   resolution, and a hard stop at any human approval gate.
//! * **Approval stays human.** Every `approve`/`verify` step maps to a real
//!   [`TaskAction::Approval`] gate (per the intent contract) — the runner parks
//!   in `AwaitingApproval` and only an explicit [`FactoryRunner::approve`]
//!   lets it continue.
//! * **Everything persists.** The job document, evidence graph, claims and
//!   failures live under `factory/jobs/<id>/` in the workspace root, so a
//!   restart resumes instead of restarting.
//!
//! No second executor exists behind this surface: an HTTP/CLI caller gets
//! exactly what [`FactoryRunner`] produced, including its honest failure
//! states (`Blocked`, `Stalled`, `StepLimit`).

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use zylcode_mcp::{JsonlEvidenceSink, PermissionGate, PermissionPolicy, ToolRuntime};

use crate::competition::natlas::intent::NatlasEngineeringIntent;
use crate::factory::job::FactoryJob;

/// Wrap a structured engineering intent in a durable factory job.
///
/// The intent is validated by [`NatlasEngineeringIntent::to_task_graph`]
/// (dependency-chained, approval-gated); invalid intents are errors here, not
/// half-constructed jobs. The job is **not** persisted by this function —
/// [`FactoryRunner::open`] does that at run time, which keeps "plan" and
/// "execute" separated for callers that want to inspect the decomposition
/// first.
pub fn agent_intent_to_job(
    intent: &NatlasEngineeringIntent,
    project_id: &str,
    repository: &str,
    branch: &str,
    worktree: &str,
    created_by: &str,
) -> Result<FactoryJob> {
    let graph = intent
        .to_task_graph()
        .context("agent intent does not decompose into a valid task graph")?;
    let mut job = FactoryJob::new(
        intent.summary.clone(),
        project_id,
        repository,
        branch,
        worktree,
        created_by,
    );
    job.graph = graph;
    Ok(job)
}

/// The tool runtime serve-intel / CLI callers use for factory execution.
///
/// Restrictive by design: actions up to `Execute` (commands) are allowed under
/// the permission gate, `GitWrite`/`Destructive`/`Release` are not — a caller
/// that wants more must construct its own [`ToolRuntime`] deliberately, which
/// keeps the HTTP surface from becoming a second, more permissive runtime.
/// Evidence lines land in the workspace's `.zylcode/factory` directory.
pub fn default_tool_runtime(root: &Path) -> ToolRuntime {
    let evidence_dir = root.join(".zylcode").join("factory");
    let _ = std::fs::create_dir_all(&evidence_dir);
    let gate = Arc::new(PermissionGate::new(PermissionPolicy::allow_up_to(
        zylcode_mcp::RiskLevel::Execute,
    )));
    let sink = Arc::new(JsonlEvidenceSink::new(
        evidence_dir.join("factory-evidence.jsonl"),
    ));
    ToolRuntime::new(gate, sink)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_intent() -> NatlasEngineeringIntent {
        let value = json!({
            "summary": "add a health check",
            "steps": [
                { "kind": "analyse", "description": "read the module" },
                {
                    "kind": "implement",
                    "description": "write the module",
                    "target_path": "src/health.rs",
                    "content": "pub fn healthy() -> bool { true }\n"
                },
                {
                    "kind": "test",
                    "description": "verify the module",
                    "command": ["echo", "ok"],
                    "verify": true
                },
                { "kind": "approve", "description": "human sign-off" }
            ]
        });
        NatlasEngineeringIntent::parse(&serde_json::to_string(&value).unwrap()).unwrap()
    }

    #[test]
    fn an_intent_becomes_a_durable_factory_job() {
        let intent = sample_intent();
        let mut job = agent_intent_to_job(&intent, "proj", "repo", "main", "/wt", "agent:journey")
            .expect("valid intent decomposes");
        assert_eq!(job.intent, "add a health check");
        assert!(
            job.graph.validate().is_ok(),
            "the seeded graph must be a valid DAG"
        );
        // The approval step of the contract must survive as a real gate, and
        // write steps must be real writes — not notes.
        let approvals: Vec<_> = job
            .graph
            .nodes()
            .iter()
            .filter(|n| matches!(n.action, crate::factory::graph::TaskAction::Approval { .. }))
            .collect();
        assert_eq!(approvals.len(), 1, "the approve step is a human gate");
        // The stage is a projection of what has *succeeded* — a freshly
        // seeded job has done nothing yet, so INTAKE is the honest stage.
        job.recompute_stage();
        assert_eq!(
            job.stage,
            crate::factory::job::JobStage::Intake,
            "a fresh job must be INTAKE — the stage is earned, not declared"
        );
    }

    #[test]
    fn an_invalid_intent_never_reaches_a_job() {
        // Unknown step kinds are rejected at parse — the bridge only ever
        // sees a validated intent, so a malformed graph cannot be smuggled in.
        let unknown_kind = serde_json::json!({ "summary": "x", "steps": [{ "kind": "unknown" }] });
        assert!(NatlasEngineeringIntent::parse(&unknown_kind.to_string()).is_err());
        let empty_summary = serde_json::json!({ "summary": "", "steps": [{ "kind": "analyse", "description": "x" }] });
        assert!(NatlasEngineeringIntent::parse(&empty_summary.to_string()).is_err());
    }

    #[test]
    fn the_default_runtime_depends_only_on_the_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let rt1 = default_tool_runtime(dir.path());
        let rt2 = default_tool_runtime(dir.path());
        // Two calls create the evidence directory idempotently and both
        // runtimes are constructible — no state machine hidden in the helper.
        assert!(dir.path().join(".zylcode").join("factory").is_dir());
        drop((rt1, rt2));
    }
}
