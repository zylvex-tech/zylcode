//! The dependency-aware task graph — Master Transformation Prompt §4 / directive
//! Phase 4.
//!
//! A [`TaskGraph`] is a DAG of [`TaskNode`]s. It answers three questions the
//! linear mission queue cannot:
//!
//! 1. **What may run now?** — [`TaskGraph::ready`] returns only the pending
//!    nodes whose dependencies have all succeeded. A node whose dependency
//!    failed is never silently promoted.
//! 2. **What is blocked, and why?** — a node is [`TaskState::Blocked`] with a
//!    stated reason. Blocked is neither "pending" nor "failed"; collapsing the
//!    three is how a system lies about progress.
//! 3. **Is the job finished?** — [`TaskGraph::is_complete`] is true only when
//!    every node is in a terminal *good* state. One failed node keeps the job
//!    incomplete.
//!
//! The graph is a pure data structure: it holds no handles, no clock, no I/O.
//! That keeps it trivially serialisable into the durable job document and
//! trivially testable.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The logical role a task plays (directive Phase 2).
///
/// These are **roles, not agents**. A role names a responsibility; whether it is
/// discharged by a model prompt, a deterministic service, a tool pipeline or a
/// gate is decided by the task's [`TaskAction`]. Naming sixteen agents that all
/// run the same code would satisfy the letter of the directive and none of its
/// intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Product / requirements: turn an intent into checkable requirements.
    Requirements,
    /// Research: gather and classify sources. (See RESEARCH_MODE_ARCHITECTURE.)
    Research,
    /// System architect: decide structure before code exists.
    Architecture,
    /// Repository intelligence: build the context the change will land in.
    RepositoryContext,
    /// Implementation: produce or modify the artifact.
    Implementation,
    /// Test engineer: run the suite and record what it returned.
    Test,
    /// Debugging: diagnose a real failure from captured output.
    Debug,
    /// Security reviewer: examine the change for risk.
    SecurityReview,
    /// Documentation: describe what exists.
    Documentation,
    /// Release engineer: confirm the artifact is ready to ship.
    ReleaseReadiness,
    /// A human decision the machine may not make alone.
    HumanApproval,
}

impl TaskKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskKind::Requirements => "requirements",
            TaskKind::Research => "research",
            TaskKind::Architecture => "architecture",
            TaskKind::RepositoryContext => "repository_context",
            TaskKind::Implementation => "implementation",
            TaskKind::Test => "test",
            TaskKind::Debug => "debug",
            TaskKind::SecurityReview => "security_review",
            TaskKind::Documentation => "documentation",
            TaskKind::ReleaseReadiness => "release_readiness",
            TaskKind::HumanApproval => "human_approval",
        }
    }

    /// The role name a report should print for this task.
    pub fn role(self) -> &'static str {
        match self {
            TaskKind::Requirements => "PRODUCT/REQUIREMENTS",
            TaskKind::Research => "RESEARCH",
            TaskKind::Architecture => "SYSTEM_ARCHITECT",
            TaskKind::RepositoryContext => "REPOSITORY_INTELLIGENCE",
            TaskKind::Implementation => "IMPLEMENTATION",
            TaskKind::Test => "TEST_ENGINEER",
            TaskKind::Debug => "DEBUGGING",
            TaskKind::SecurityReview => "SECURITY_REVIEWER",
            TaskKind::Documentation => "DOCUMENTATION",
            TaskKind::ReleaseReadiness => "RELEASE_ENGINEER",
            TaskKind::HumanApproval => "HUMAN_CHECKPOINT",
        }
    }
}

/// Lifecycle state of one task.
///
/// The distinction between [`TaskState::Pending`] (dependencies unmet),
/// [`TaskState::Ready`] (dependencies met, not started), [`TaskState::Blocked`]
/// (cannot proceed, reason recorded) and [`TaskState::Failed`] (ran and did not
/// meet its contract) is load-bearing. A system that reports one of these as
/// another cannot be trusted about its own progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Dependencies are not all satisfied yet.
    Pending,
    /// Dependencies satisfied; eligible to run.
    Ready,
    /// Currently executing.
    Running,
    /// Ran and met its contract.
    Succeeded,
    /// Ran and did not meet its contract.
    Failed,
    /// Cannot proceed. The reason is recorded; the task never ran.
    Blocked,
    /// Parked at a human decision gate.
    AwaitingApproval,
    /// Explicitly cancelled (by a human or a dependency's terminal failure).
    Cancelled,
    /// Deliberately not executed (e.g. a dependency branch was not taken).
    Skipped,
}

impl TaskState {
    /// Terminal *good* states — the only states a completed job may contain.
    pub fn is_success(self) -> bool {
        matches!(self, TaskState::Succeeded | TaskState::Skipped)
    }

    /// No further transition will happen without external input.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            TaskState::Succeeded
                | TaskState::Failed
                | TaskState::Blocked
                | TaskState::Cancelled
                | TaskState::Skipped
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TaskState::Pending => "pending",
            TaskState::Ready => "ready",
            TaskState::Running => "running",
            TaskState::Succeeded => "succeeded",
            TaskState::Failed => "failed",
            TaskState::Blocked => "blocked",
            TaskState::AwaitingApproval => "awaiting_approval",
            TaskState::Cancelled => "cancelled",
            TaskState::Skipped => "skipped",
        }
    }
}

/// What a task actually does.
///
/// Every variant either dispatches through a real tool, requires a human, or is
/// explicitly blocked. There is no variant that "pretends".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum TaskAction {
    /// Read a workspace-relative file through the `fs.read` tool.
    ReadFile { path: String },
    /// Write a workspace-relative file through the `fs.write` tool.
    WriteFile { path: String, content: String },
    /// Run a program through the `shell.execute` tool; the exit code is checked.
    RunCommand {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        expect_exit: i32,
    },
    /// Run a command and record the outcome as a test claim at level R3.
    Verify {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// Work that requires a model. BLOCKED while no approved provider exists.
    ModelTask { prompt: String },
    /// A human decision gate. The task parks until approved.
    Approval { reason: String },
    /// A recorded annotation with no side effect.
    Note { text: String },
}

impl TaskAction {
    /// The tool id this action dispatches to, or `None` for non-tool actions.
    pub fn tool_id(&self) -> Option<&'static str> {
        match self {
            TaskAction::ReadFile { .. } => Some("fs.read"),
            TaskAction::WriteFile { .. } => Some("fs.write"),
            TaskAction::RunCommand { .. } | TaskAction::Verify { .. } => Some("shell.execute"),
            TaskAction::ModelTask { .. } | TaskAction::Approval { .. } | TaskAction::Note { .. } => {
                None
            }
        }
    }

    /// One-line description for reports.
    pub fn describe(&self) -> String {
        match self {
            TaskAction::ReadFile { path } => format!("read {path}"),
            TaskAction::WriteFile { path, .. } => format!("write {path}"),
            TaskAction::RunCommand {
                command,
                args,
                expect_exit,
            } => format!("run `{command} {}` (expect exit {expect_exit})", args.join(" ")),
            TaskAction::Verify { command, args } => {
                format!("verify `{command} {}`", args.join(" "))
            }
            TaskAction::ModelTask { .. } => "model task (provider-gated)".to_string(),
            TaskAction::Approval { reason } => format!("human approval: {reason}"),
            TaskAction::Note { .. } => "note".to_string(),
        }
    }
}

/// One node of the task graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: String,
    pub kind: TaskKind,
    pub title: String,
    pub action: TaskAction,
    /// Ids of tasks that must reach a success state before this one may run.
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub state: TaskState,
    /// How many times execution has been attempted.
    #[serde(default)]
    pub attempts: u32,
    /// Maximum attempts before the node is failed rather than retried forever.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// Why the node is blocked. Present only in [`TaskState::Blocked`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
    /// Set when a human approved an [`TaskAction::Approval`] gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    /// The evidence-graph node this task produced, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_node_id: Option<String>,
    /// Claims recorded by this task.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claim_ids: Vec<String>,
    /// The failure record id, when the task failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_id: Option<String>,
    /// Short human-readable outcome.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
}

fn default_max_attempts() -> u32 {
    1
}

impl TaskNode {
    pub fn new(
        id: impl Into<String>,
        kind: TaskKind,
        title: impl Into<String>,
        action: TaskAction,
    ) -> Self {
        TaskNode {
            id: id.into(),
            kind,
            title: title.into(),
            action,
            depends_on: Vec::new(),
            state: TaskState::Pending,
            attempts: 0,
            max_attempts: default_max_attempts(),
            blocked_reason: None,
            approved_by: None,
            evidence_node_id: None,
            claim_ids: Vec::new(),
            failure_id: None,
            outcome: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
        }
    }

    pub fn depends_on(mut self, ids: &[&str]) -> Self {
        self.depends_on = ids.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n.max(1);
        self
    }
}

/// Aggregate counts for a report or a dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub blocked: usize,
    pub awaiting_approval: usize,
    pub pending: usize,
    pub ready: usize,
    pub running: usize,
    pub cancelled: usize,
    pub skipped: usize,
}

/// A dependency-aware task graph.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskGraph {
    nodes: Vec<TaskNode>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn nodes(&self) -> &[TaskNode] {
        &self.nodes
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Append a node. Duplicate ids are refused — an ambiguous graph cannot be
    /// reasoned about.
    pub fn add_node(&mut self, node: TaskNode) -> Result<()> {
        if node.id.trim().is_empty() {
            bail!("task id must not be empty");
        }
        if self.nodes.iter().any(|n| n.id == node.id) {
            bail!("duplicate task id: {}", node.id);
        }
        self.nodes.push(node);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&TaskNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut TaskNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Structural validation: unique ids, every dependency exists, and the
    /// dependency relation is acyclic. Called before a job is executed.
    pub fn validate(&self) -> Result<()> {
        let mut ids = BTreeSet::new();
        for n in &self.nodes {
            if n.id.trim().is_empty() {
                bail!("task with empty id");
            }
            if !ids.insert(n.id.as_str()) {
                bail!("duplicate task id: {}", n.id);
            }
        }
        for n in &self.nodes {
            for dep in &n.depends_on {
                if dep == &n.id {
                    bail!("task {} depends on itself", n.id);
                }
                if !ids.contains(dep.as_str()) {
                    bail!("task {} depends on unknown task {}", n.id, dep);
                }
            }
        }
        // Cycle detection via Kahn's algorithm.
        let mut indegree: BTreeMap<&str, usize> =
            self.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
        let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for n in &self.nodes {
            for dep in &n.depends_on {
                *indegree.entry(n.id.as_str()).or_insert(0) += 1;
                adjacency.entry(dep.as_str()).or_default().push(n.id.as_str());
            }
        }
        let mut queue: Vec<&str> = indegree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut visited = 0usize;
        while let Some(id) = queue.pop() {
            visited += 1;
            if let Some(nexts) = adjacency.get(id) {
                for next in nexts {
                    if let Some(d) = indegree.get_mut(next) {
                        *d -= 1;
                        if *d == 0 {
                            queue.push(next);
                        }
                    }
                }
            }
        }
        if visited != self.nodes.len() {
            bail!("task graph contains a dependency cycle");
        }
        Ok(())
    }

    /// Ids of tasks eligible to run: [`TaskState::Pending`] whose every
    /// dependency is in a success state. Order is stable (declaration order).
    pub fn ready(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|n| n.state == TaskState::Pending)
            .filter(|n| {
                n.depends_on.iter().all(|dep| {
                    self.get(dep)
                        .map(|d| d.state.is_success())
                        .unwrap_or(false)
                })
            })
            .map(|n| n.id.clone())
            .collect()
    }

    /// True only when every node reached a terminal *good* state.
    pub fn is_complete(&self) -> bool {
        !self.nodes.is_empty() && self.nodes.iter().all(|n| n.state.is_success())
    }

    /// True when at least one node could still make progress. A node parked
    /// awaiting a human is *not* live work — the graph cannot advance it without
    /// external input — and neither is a pending node whose dependency already
    /// reached a terminal failure (see [`TaskGraph::orphaned`]).
    ///
    /// `false` together with `is_complete() == false` means the job is **stalled
    /// or blocked** — a state the runner reports rather than hides.
    pub fn has_live_work(&self) -> bool {
        let orphaned = self.orphaned();
        self.nodes.iter().any(|n| match n.state {
            TaskState::Ready | TaskState::Running => true,
            TaskState::Pending => !orphaned.contains(&n.id),
            _ => false,
        })
    }

    /// Nodes that ended blocked, with their reasons.
    pub fn blocked(&self) -> Vec<&TaskNode> {
        self.nodes
            .iter()
            .filter(|n| n.state == TaskState::Blocked)
            .collect()
    }

    /// Nodes parked at a human gate.
    pub fn awaiting_approval(&self) -> Vec<&TaskNode> {
        self.nodes
            .iter()
            .filter(|n| n.state == TaskState::AwaitingApproval)
            .collect()
    }

    /// A node whose dependency reached a terminal *bad* state can never run.
    /// Returns those ids so the runner can cancel them explicitly instead of
    /// leaving them pending forever.
    pub fn orphaned(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|n| n.state == TaskState::Pending)
            .filter(|n| {
                n.depends_on.iter().any(|dep| {
                    self.get(dep)
                        .map(|d| {
                            matches!(
                                d.state,
                                TaskState::Failed
                                    | TaskState::Blocked
                                    | TaskState::Cancelled
                            )
                        })
                        .unwrap_or(true)
                })
            })
            .map(|n| n.id.clone())
            .collect()
    }

    pub fn summary(&self) -> GraphSummary {
        let count = |s: TaskState| self.nodes.iter().filter(|n| n.state == s).count();
        GraphSummary {
            total: self.nodes.len(),
            succeeded: count(TaskState::Succeeded),
            failed: count(TaskState::Failed),
            blocked: count(TaskState::Blocked),
            awaiting_approval: count(TaskState::AwaitingApproval),
            pending: count(TaskState::Pending),
            ready: count(TaskState::Ready),
            running: count(TaskState::Running),
            cancelled: count(TaskState::Cancelled),
            skipped: count(TaskState::Skipped),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str) -> TaskNode {
        TaskNode::new(
            id,
            TaskKind::Documentation,
            format!("note {id}"),
            TaskAction::Note {
                text: format!("{id} done"),
            },
        )
    }

    #[test]
    fn duplicate_id_refused() {
        let mut g = TaskGraph::new();
        g.add_node(note("a")).unwrap();
        assert!(g.add_node(note("a")).is_err());
        assert!(g
            .add_node(TaskNode::new(
                "",
                TaskKind::Documentation,
                "empty",
                TaskAction::Note { text: "x".into() }
            ))
            .is_err());
    }

    #[test]
    fn validate_rejects_unknown_dependency_and_self_loop() {
        let mut g = TaskGraph::new();
        g.add_node(note("a").depends_on(&["ghost"])).unwrap();
        assert!(g.validate().is_err());

        let mut g2 = TaskGraph::new();
        g2.add_node(note("a").depends_on(&["a"])).unwrap();
        assert!(g2.validate().is_err());
    }

    #[test]
    fn validate_rejects_cycles() {
        let mut g = TaskGraph::new();
        g.add_node(note("a").depends_on(&["c"])).unwrap();
        g.add_node(note("b").depends_on(&["a"])).unwrap();
        g.add_node(note("c").depends_on(&["b"])).unwrap();
        let err = g.validate().unwrap_err().to_string();
        assert!(err.contains("cycle"), "{err}");
    }

    #[test]
    fn diamond_graph_is_valid_and_ready_is_dependency_aware() {
        let mut g = TaskGraph::new();
        g.add_node(note("root")).unwrap();
        g.add_node(note("left").depends_on(&["root"])).unwrap();
        g.add_node(note("right").depends_on(&["root"])).unwrap();
        g.add_node(note("join").depends_on(&["left", "right"])).unwrap();
        g.validate().unwrap();

        // Only the root is ready initially.
        assert_eq!(g.ready(), vec!["root".to_string()]);

        g.get_mut("root").unwrap().state = TaskState::Succeeded;
        // Now both branches are ready; the join is not.
        let ready = g.ready();
        assert_eq!(ready, vec!["left".to_string(), "right".to_string()]);

        g.get_mut("left").unwrap().state = TaskState::Succeeded;
        assert_eq!(g.ready(), vec!["right".to_string()]);

        g.get_mut("right").unwrap().state = TaskState::Succeeded;
        assert_eq!(g.ready(), vec!["join".to_string()]);

        g.get_mut("join").unwrap().state = TaskState::Succeeded;
        assert!(g.ready().is_empty());
        assert!(g.is_complete());
    }

    #[test]
    fn a_failed_dependency_orphans_its_dependents() {
        let mut g = TaskGraph::new();
        g.add_node(note("a")).unwrap();
        g.add_node(note("b").depends_on(&["a"])).unwrap();
        g.get_mut("a").unwrap().state = TaskState::Failed;

        assert!(!g.is_complete());
        assert!(g.ready().is_empty(), "b must not become ready");
        assert_eq!(g.orphaned(), vec!["b".to_string()]);
        assert!(!g.has_live_work(), "nothing can make progress");
    }

    #[test]
    fn blocked_dependency_orphans_dependents_too() {
        let mut g = TaskGraph::new();
        g.add_node(note("a")).unwrap();
        g.add_node(note("b").depends_on(&["a"])).unwrap();
        g.get_mut("a").unwrap().state = TaskState::Blocked;
        assert_eq!(g.orphaned(), vec!["b".to_string()]);
        assert_eq!(g.blocked().len(), 1);
    }

    #[test]
    fn an_empty_graph_is_not_complete() {
        let g = TaskGraph::new();
        assert!(!g.is_complete());
        assert!(!g.has_live_work());
    }

    #[test]
    fn summary_counts_every_state() {
        let mut g = TaskGraph::new();
        g.add_node(note("a")).unwrap();
        g.add_node(note("b")).unwrap();
        g.add_node(note("c")).unwrap();
        g.add_node(note("d")).unwrap();
        g.get_mut("a").unwrap().state = TaskState::Succeeded;
        g.get_mut("b").unwrap().state = TaskState::Failed;
        g.get_mut("c").unwrap().state = TaskState::Blocked;
        let s = g.summary();
        assert_eq!(s.total, 4);
        assert_eq!(s.succeeded, 1);
        assert_eq!(s.failed, 1);
        assert_eq!(s.blocked, 1);
        assert_eq!(s.pending, 1);
    }

    #[test]
    fn action_reports_its_real_tool_binding() {
        assert_eq!(
            TaskAction::ReadFile {
                path: "a.rs".into()
            }
            .tool_id(),
            Some("fs.read")
        );
        assert_eq!(
            TaskAction::WriteFile {
                path: "a.rs".into(),
                content: "x".into()
            }
            .tool_id(),
            Some("fs.write")
        );
        assert_eq!(
            TaskAction::Verify {
                command: "cargo".into(),
                args: vec!["test".into()]
            }
            .tool_id(),
            Some("shell.execute")
        );
        // Model work has no tool — it is provider-gated, not silently routed.
        assert_eq!(TaskAction::ModelTask { prompt: "x".into() }.tool_id(), None);
    }
}
