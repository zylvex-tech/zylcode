//! The factory job — directive Phase 3.
//!
//! A [`FactoryJob`] is the durable header of a unit of engineering work: who
//! asked for it, against which repository and revision, what stage it reached,
//! and the [`TaskGraph`] that decomposes it.
//!
//! # The lifecycle is a projection, not a claim
//!
//! [`JobStage`] is derived from what the graph has actually achieved
//! ([`FactoryJob::derive_stage`]). It moves **forward only**: a job that reached
//! `TEST` does not fall back to `PLAN` because a later task is still pending.
//! The stage is therefore a conservative lower bound on progress — it can lag
//! reality, and it can never lead it. A stage that led reality would be a
//! fabricated status, which is the one thing this product may not do.

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::graph::{TaskGraph, TaskKind};

/// The factory job lifecycle (directive Phase 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStage {
    Intake,
    Discovery,
    Requirements,
    Research,
    Plan,
    Architecture,
    TaskGraph,
    Implementation,
    Test,
    Review,
    Security,
    Documentation,
    ReleaseReadiness,
    OwnerApproval,
    Delivery,
}

impl JobStage {
    /// Ordinal position, used to enforce forward-only movement.
    pub fn order(self) -> u8 {
        match self {
            JobStage::Intake => 0,
            JobStage::Discovery => 1,
            JobStage::Requirements => 2,
            JobStage::Research => 3,
            JobStage::Plan => 4,
            JobStage::Architecture => 5,
            JobStage::TaskGraph => 6,
            JobStage::Implementation => 7,
            JobStage::Test => 8,
            JobStage::Review => 9,
            JobStage::Security => 10,
            JobStage::Documentation => 11,
            JobStage::ReleaseReadiness => 12,
            JobStage::OwnerApproval => 13,
            JobStage::Delivery => 14,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            JobStage::Intake => "INTAKE",
            JobStage::Discovery => "DISCOVERY",
            JobStage::Requirements => "REQUIREMENTS",
            JobStage::Research => "RESEARCH",
            JobStage::Plan => "PLAN",
            JobStage::Architecture => "ARCHITECTURE",
            JobStage::TaskGraph => "TASK_GRAPH",
            JobStage::Implementation => "IMPLEMENTATION",
            JobStage::Test => "TEST",
            JobStage::Review => "REVIEW",
            JobStage::Security => "SECURITY",
            JobStage::Documentation => "DOCUMENTATION",
            JobStage::ReleaseReadiness => "RELEASE_READINESS",
            JobStage::OwnerApproval => "OWNER_APPROVAL",
            JobStage::Delivery => "DELIVERY",
        }
    }

    /// The stage a task kind advances the job to when it succeeds.
    pub fn for_kind(kind: TaskKind) -> JobStage {
        match kind {
            TaskKind::Requirements => JobStage::Requirements,
            TaskKind::Research => JobStage::Research,
            TaskKind::Architecture => JobStage::Architecture,
            TaskKind::RepositoryContext => JobStage::Discovery,
            TaskKind::Implementation => JobStage::Implementation,
            TaskKind::Test => JobStage::Test,
            TaskKind::Debug => JobStage::Test,
            TaskKind::SecurityReview => JobStage::Security,
            TaskKind::Documentation => JobStage::Documentation,
            TaskKind::ReleaseReadiness => JobStage::ReleaseReadiness,
            TaskKind::HumanApproval => JobStage::OwnerApproval,
        }
    }
}

/// The durable header of a factory job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactoryJob {
    pub schema_version: u32,
    pub id: String,
    /// The project this job belongs to (Constitution §4: a Project is not a
    /// directory).
    pub project_id: String,
    /// Repository URL or path the job operates on.
    pub repository: String,
    pub branch: String,
    /// Worktree the tools execute in. Recorded so a resume lands in the same
    /// place and a report can say where the work happened.
    pub worktree: String,
    /// The originating intent, verbatim.
    pub intent: String,
    pub stage: JobStage,
    pub graph: TaskGraph,
    /// Who or what created the job.
    pub created_by: String,
    /// Free-form recorded notes (never a substitute for evidence).
    #[serde(default)]
    pub notes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl FactoryJob {
    pub fn new(
        intent: impl Into<String>,
        project_id: impl Into<String>,
        repository: impl Into<String>,
        branch: impl Into<String>,
        worktree: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        FactoryJob {
            schema_version: 1,
            id: Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            repository: repository.into(),
            branch: branch.into(),
            worktree: worktree.into(),
            intent: intent.into(),
            stage: JobStage::Intake,
            graph: TaskGraph::new(),
            created_by: created_by.into(),
            notes: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn note(&mut self, text: impl Into<String>) {
        self.notes.push(text.into());
        self.updated_at = Utc::now();
    }

    /// Advance the stage. Forward-only: a request to move backwards is refused
    /// rather than silently applied, because a stage that can regress is not a
    /// record of what happened.
    pub fn advance_stage(&mut self, stage: JobStage) -> Result<()> {
        if stage.order() < self.stage.order() {
            bail!(
                "job {} is at {} and cannot move back to {}",
                self.id,
                self.stage.as_str(),
                stage.as_str()
            );
        }
        self.stage = stage;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// The stage implied by the graph's own state: the furthest stage any
    /// *succeeded* task kind has reached. If nothing has succeeded yet but work
    /// has begun, the job has at least reached `TASK_GRAPH`. A complete graph is
    /// `DELIVERY`. Pure — no mutation, so it can be used in a report.
    ///
    /// Note the deliberate omission of a blanket "has a graph ⇒ at least
    /// TASK_GRAPH" floor: a job whose graph was just built has *achieved*
    /// nothing yet, and saying otherwise would be a stage that leads reality.
    pub fn derive_stage(&self) -> JobStage {
        if self.graph.is_complete() {
            return JobStage::Delivery;
        }
        let mut best = JobStage::Intake;
        for node in self.graph.nodes() {
            if node.state.is_success() {
                let s = JobStage::for_kind(node.kind);
                if s.order() > best.order() {
                    best = s;
                }
            }
        }
        if best == JobStage::Intake && self.graph.nodes().iter().any(|n| n.attempts > 0) {
            return JobStage::TaskGraph;
        }
        best
    }

    /// Recompute the stage from the graph, moving forward only. Returns the
    /// stage after the update.
    pub fn recompute_stage(&mut self) -> JobStage {
        let derived = self.derive_stage();
        if derived.order() > self.stage.order() {
            self.stage = derived;
        }
        self.updated_at = Utc::now();
        self.stage
    }

    /// A job is terminal when its graph is complete, or when nothing in it can
    /// make further progress without external input.
    pub fn is_terminal(&self) -> bool {
        self.graph.is_complete()
            || (!self.graph.has_live_work()
                && self.graph.awaiting_approval().is_empty()
                && !self.graph.is_empty())
    }

    pub fn summary_line(&self) -> String {
        let s = self.graph.summary();
        format!(
            "job {} [{}] {}/{} succeeded, {} failed, {} blocked, {} awaiting approval",
            &self.id[..8.min(self.id.len())],
            self.stage.as_str(),
            s.succeeded,
            s.total,
            s.failed,
            s.blocked,
            s.awaiting_approval
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::graph::{TaskAction, TaskNode, TaskState};

    fn job_with(nodes: Vec<TaskNode>) -> FactoryJob {
        let mut job = FactoryJob::new("intent", "proj", "repo", "main", "/wt", "tester");
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
                text: id.to_string(),
            },
        )
    }

    #[test]
    fn a_new_job_starts_at_intake() {
        let job = FactoryJob::new("i", "p", "r", "main", "/wt", "t");
        assert_eq!(job.stage, JobStage::Intake);
        assert!(!job.is_terminal());
    }

    #[test]
    fn stage_is_forward_only() {
        let mut job = FactoryJob::new("i", "p", "r", "main", "/wt", "t");
        job.advance_stage(JobStage::Implementation).unwrap();
        assert!(job.advance_stage(JobStage::Plan).is_err());
        assert_eq!(job.stage, JobStage::Implementation);
    }

    #[test]
    fn derive_stage_follows_succeeded_task_kinds() {
        let mut job = job_with(vec![
            note("req", TaskKind::Requirements),
            note("impl", TaskKind::Implementation),
            note("test", TaskKind::Test),
            // A fourth, deliberately unfinished task keeps the graph incomplete,
            // so the stage can be observed *below* DELIVERY.
            note("doc", TaskKind::Documentation).depends_on(&["test"]),
        ]);
        // Nothing has been attempted: the job has achieved nothing yet, so the
        // stage must not claim the graph's existence as progress.
        assert_eq!(job.derive_stage(), JobStage::Intake);

        // Once work begins but nothing has succeeded, the job has at least
        // reached the task-graph stage.
        job.graph.get_mut("req").unwrap().attempts = 1;
        assert_eq!(job.derive_stage(), JobStage::TaskGraph);

        job.graph.get_mut("req").unwrap().state = TaskState::Succeeded;
        assert_eq!(job.derive_stage(), JobStage::Requirements);

        job.graph.get_mut("impl").unwrap().state = TaskState::Succeeded;
        assert_eq!(job.derive_stage(), JobStage::Implementation);

        // A *failed* test task does not advance the stage — only success does.
        job.graph.get_mut("test").unwrap().state = TaskState::Failed;
        assert_eq!(job.derive_stage(), JobStage::Implementation);

        // Fixing it to success reaches TEST — and stops there, because `doc` is
        // still outstanding.
        job.graph.get_mut("test").unwrap().state = TaskState::Succeeded;
        assert_eq!(job.derive_stage(), JobStage::Test);
        assert!(!job.graph.is_complete());

        // Only when the last task succeeds does the job reach DELIVERY.
        job.graph.get_mut("doc").unwrap().state = TaskState::Succeeded;
        assert_eq!(job.derive_stage(), JobStage::Delivery);
    }

    #[test]
    fn derive_stage_is_delivery_when_graph_is_complete() {
        let mut job = job_with(vec![note("a", TaskKind::Documentation)]);
        job.graph.get_mut("a").unwrap().state = TaskState::Succeeded;
        assert_eq!(job.derive_stage(), JobStage::Delivery);
    }

    #[test]
    fn recompute_never_moves_backwards() {
        let mut job = job_with(vec![
            note("a", TaskKind::Requirements),
            note("b", TaskKind::Implementation),
        ]);
        job.graph.get_mut("b").unwrap().state = TaskState::Succeeded;
        job.recompute_stage();
        assert_eq!(job.stage, JobStage::Implementation);

        // Even if the graph were later reduced, the recorded stage holds.
        job.graph.get_mut("b").unwrap().state = TaskState::Pending;
        job.recompute_stage();
        assert_eq!(job.stage, JobStage::Implementation);
    }

    #[test]
    fn a_blocked_graph_with_no_live_work_is_terminal() {
        let mut job = job_with(vec![note("a", TaskKind::Test)]);
        job.graph.get_mut("a").unwrap().state = TaskState::Blocked;
        assert!(job.is_terminal());
    }

    #[test]
    fn a_graph_awaiting_approval_is_not_terminal() {
        let mut job = job_with(vec![note("a", TaskKind::HumanApproval)]);
        job.graph.get_mut("a").unwrap().state = TaskState::AwaitingApproval;
        assert!(!job.is_terminal());
    }
}
