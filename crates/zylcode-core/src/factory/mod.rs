//! ZylCode Software Factory — the durable, dependency-aware factory job.
//!
//! # What this module is
//!
//! A **factory job** is a unit of engineering work that outlives the process
//! that started it. It carries:
//!
//! * an explicit [`JobStage`] lifecycle (`INTAKE → … → DELIVERY`),
//! * a [`TaskGraph`] — a dependency-aware DAG, not a linear queue,
//! * durable state on disk ([`JobStore`]), so a restart resumes rather than
//!   restarts,
//! * a [`FactoryRunner`] that executes each ready task through the **real**
//!   tool runtime (`zylcode_mcp::dispatch`) and records what actually happened
//!   into the existing evidence stores — the [`EvidenceGraph`], the
//!   [`ClaimStore`], and the [`Failure`] taxonomy.
//!
//! # What this module is deliberately not
//!
//! * It is **not** a second mission queue. `missions.rs` is a linear FIFO with
//!   approval states; it has no notion of a dependency between two tasks. A
//!   factory job does. The two coexist: a mission is *what the operator asked
//!   for*, a factory job is *how a non-trivial piece of work is decomposed and
//!   proven*.
//! * It is **not** a set of fake agents. The roles in the directive are modelled
//!   as [`TaskKind`]s with explicit responsibilities — a prompt, a deterministic
//!   service, a tool pipeline, or a gate — and the runner executes them. Creating
//!   sixteen "agents" that share one code path would be theatre, and the
//!   directive forbids exactly that.
//! * It **never fabricates model success**. A task whose work requires a model
//!   is [`TaskState::Blocked`] with reason `BLOCKED_PROVIDER` while AGENT-01
//!   remains blocked. The deterministic orchestration, tool execution and
//!   evidence are proven independently of any provider.
//!
//! # Reused substrate (nothing is re-implemented)
//!
//! | Concern | Existing module |
//! |---|---|
//! | Epistemic claim + fail-closed promotion | [`crate::claim`] |
//! | Traversable provenance graph | [`crate::evidence_graph`] |
//! | Failure taxonomy + recovery | [`crate::failure`] |
//! | Gated tool dispatch + JSONL evidence | `zylcode_mcp::{dispatch, ToolRuntime}` |
//!
//! [`EvidenceGraph`]: crate::evidence_graph::EvidenceGraph
//! [`ClaimStore`]: crate::claim::ClaimStore
//! [`Failure`]: crate::failure::Failure

pub mod graph;
pub mod job;
pub mod runner;
pub mod store;

pub use graph::{TaskAction, TaskGraph, TaskKind, TaskNode, TaskState};
pub use job::{FactoryJob, JobStage};
pub use runner::{
    FactoryRunner, FactoryRunnerConfig, RunReport, RunStatus, BLOCKED_PROVIDER,
    DEFAULT_TASK_TIMEOUT_SECS, RECOMMENDED_RISK_CEILING,
};
pub use store::{jobs_dir, JobStore};
