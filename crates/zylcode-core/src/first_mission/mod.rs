//! The First Mission — Master Transformation Prompt §36.
//!
//! One complete end-to-end autonomous engineering mission against a real
//! fixture project, with real tools, a real recoverable failure, real repair,
//! interruption/resume, and evidence for every mandated step:
//!
//! 1. human specification  2. structured plan  3. real tool execution
//! 4. real project modification  5. real test run  6. recoverable failure
//! 7. diagnosis from captured output  8. repair  9. re-verification
//! 10. persisted execution ledger  11. evidence graph  12. engineering record
//! 13. verified vs hypothesis/unknown classification  14. survives kill + resume
//! 15. reproducibility package.
//!
//! Design notes (honesty constraints):
//!
//! * The specification is human-authored prose plus a machine-readable JSON
//!   block; the repair edit is **specification-directed**, not model-invented.
//!   Whether model-driven repair would also succeed is recorded as a
//!   HYPOTHESIS, never as a verified fact.
//! * Interruption is injected by a harness (`ZYLCODE_MISSION_CRASH_AT`, the
//!   CLI reads the env var) via `std::process::abort()` *after* a step's real
//!   side effects and *before* the state checkpoint. The death is real; only
//!   its location is chosen for determinism. Recovery is real: the resumed
//!   run reconciles the filesystem against expected effect hashes and never
//!   blindly repeats completed work.
//! * Pass/fail of every verification is established by exit codes and output
//!   signatures of real commands — never by the model's say-so.

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use uuid::Uuid;

use crate::claim::{
    Claim, ClaimKind, ClaimSource, ClaimStatus, ClaimStore, EvidenceKind, VerificationLevel,
};
use crate::evidence_graph::{EdgeKind, EvidenceGraph, EvidenceNode, NodeKind};
use crate::failure::{Failure, FailureStatus};
use crate::ledger::{ExecutionState, LedgerEntry, LedgerStore, SessionCheckpoint};
use crate::missions::{MissionMode, MissionQueue};
use crate::sqlite_ledger::SqliteLedgerStore;

// Specification parsing and plan construction live in a child module so this
// file is the *runner* rather than also being the grammar. The public surface
// is unchanged: `zylcode_core::first_mission::MissionSpec` and friends are
// re-exported below.
pub mod spec;

use self::spec::build_plan;
pub use self::spec::{parse_spec, Expect, MissionSpec, Operation, Plan, PlanStep};

// ---------------------------------------------------------------------------
// Persistent mission state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    Blocked,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Completed => "completed",
            RunStatus::Blocked => "blocked",
        }
    }
}

/// Checkpoint of the mission itself (mirrors `SessionCheckpoint` discipline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionState {
    pub schema_version: u32,
    pub mission_id: String,
    pub session_id: Uuid,
    pub spec_hash: String,
    pub spec_path: String,
    pub fixture_path: String,
    /// Index into `Plan.steps` of the next step to execute.
    pub next_step: usize,
    pub resumed_count: u32,
    /// Steps skipped on resume because their evidence was already on disk
    /// (crash landed after evidence, before this checkpoint).
    #[serde(default)]
    pub reconciled_steps: Vec<String>,
    pub status: RunStatus,
    pub blocked_reason: Option<String>,
    /// Last evidence-graph node id (spine continuity across resume).
    pub last_node: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionEnvironment {
    pub python_version: String,
    pub git_version: String,
    pub os: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntegrityReport {
    pub ledger_chain: bool,
    pub graph: bool,
    pub claims_chain: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionOutcomeStatus {
    Completed,
    Blocked,
}

/// What the runner reports (feeds the §35 closing statement).
#[derive(Debug, Clone, Serialize)]
pub struct MissionOutcome {
    pub mission_id: String,
    pub status: MissionOutcomeStatus,
    pub blocked_reason: Option<String>,
    pub workdir: PathBuf,
    pub mission_dir: PathBuf,
    pub record_path: PathBuf,
    pub package_path: PathBuf,
    pub verified_claims: Vec<String>,
    pub observed_claims: Vec<String>,
    pub derived_claims: Vec<String>,
    pub hypotheses: Vec<String>,
    pub unknowns: Vec<String>,
    pub failure_count: usize,
    pub resumed_count: u32,
    pub reconciled_steps: usize,
    pub integrity: IntegrityReport,
}

pub struct MissionRunConfig {
    pub spec_path: PathBuf,
    pub fixture_path: PathBuf,
    pub workdir: PathBuf,
    pub fresh: bool,
    /// Step id at which the harness injects an interruption (test only).
    pub crash_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(sha256_hex(&bytes))
}

fn tail(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let start = s.len() - max;
        // do not split a char boundary
        let mut i = start;
        while i < s.len() && !s.is_char_boundary(i) {
            i += 1;
        }
        s[i..].to_string()
    }
}

fn first_line_containing(hay: &str, needle: &str) -> Option<String> {
    hay.lines()
        .find(|l| l.contains(needle))
        .map(|l| l.trim().to_string())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to)
                .with_context(|| format!("copy {} -> {}", entry.path().display(), to.display()))?;
        }
    }
    Ok(())
}

fn git(args: &[&str], cwd: &Path) -> Result<std::process::Output> {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .context("git is required for the First Mission (not found on PATH)")
}

fn git_stdout(args: &[&str], cwd: &Path) -> Result<String> {
    let out = git(args, cwd)?;
    if !out.status.success() {
        bail!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[derive(Debug, Clone)]
pub struct CmdOutcome {
    pub exit_code: i32,
    pub spawn_ok: bool,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

impl CmdOutcome {
    pub fn haystack(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

fn run_cmd(argv: &[String], cwd: &Path) -> Result<CmdOutcome> {
    if argv.is_empty() {
        bail!("empty command");
    }
    let started = Instant::now();
    let out = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(cwd)
        .output();
    let duration_ms = started.elapsed().as_millis() as u64;
    Ok(match out {
        Ok(o) => CmdOutcome {
            exit_code: o.status.code().unwrap_or(-1),
            spawn_ok: true,
            stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
            duration_ms,
        },
        Err(e) => CmdOutcome {
            exit_code: -1,
            spawn_ok: false,
            stdout: String::new(),
            stderr: format!("spawn failed: {e}"),
            duration_ms,
        },
    })
}

/// Ledger chain material: hash of an entry's identity + outcome.
pub fn entry_material_hash(e: &LedgerEntry) -> String {
    let state = serde_json::to_string(&e.state).unwrap_or_default();
    let payload = e
        .payload
        .as_ref()
        .map(|p| serde_json::to_string(p).unwrap_or_default())
        .unwrap_or_default();
    sha256_hex(format!("{}|{}|{}|{}", e.id, e.action_id, state, payload).as_bytes())
}

/// Verify the prev-hash chain of ledger entries (step 10/§5 integrity).
pub fn verify_ledger_chain(entries: &[LedgerEntry]) -> bool {
    let mut expected_prev = "0".repeat(64);
    for e in entries {
        if e.prev_hash != expected_prev {
            return false;
        }
        expected_prev = entry_material_hash(e);
    }
    true
}

// ---------------------------------------------------------------------------
// The mission runner
// ---------------------------------------------------------------------------

/// A fully materialised First Mission: workspace layout, evidence stores, and
/// the checkpointed run loop (§36).
///
/// Layout under `workdir`:
/// ```text
/// workdir/
///   project/                  <- fixture copy; real git repo; real tools run here
///   .zylcode/missions.json    <- MissionQueue (existing product surface)
///   .zylcode/first_mission/
///     state.json  plan.json  ledger.sqlite
///     evidence_graph.json  claims.json  failures.json
///     engineering_record.md  repro/
/// ```
pub struct FirstMission {
    pub spec: MissionSpec,
    pub spec_hash: String,
    pub plan: Plan,
    pub state: MissionState,
    pub env: MissionEnvironment,
    pub workdir: PathBuf,
    pub project_dir: PathBuf,
    pub mission_dir: PathBuf,
    pub crash_at: Option<String>,
    ledger: SqliteLedgerStore,
    graph: EvidenceGraph,
    claims: ClaimStore,
    queue: MissionQueue,
    failures: Vec<Failure>,
    /// Material hash of the last ledger entry (chain continuation on resume).
    last_material: String,
    /// Last ledger entry id written (checkpoint context).
    last_entry_id: Option<Uuid>,
}

impl FirstMission {
    fn state_path(&self) -> PathBuf {
        self.mission_dir.join("state.json")
    }

    fn plan_path(&self) -> PathBuf {
        self.mission_dir.join("plan.json")
    }

    fn failures_path(&self) -> PathBuf {
        self.mission_dir.join("failures.json")
    }

    fn record_path(&self) -> PathBuf {
        self.mission_dir.join("engineering_record.md")
    }

    fn repro_dir(&self) -> PathBuf {
        self.mission_dir.join("repro")
    }

    /// Action id namespace for ledger entries of this runner.
    fn action_id(step_id: &str) -> String {
        format!("first_mission:{step_id}")
    }

    /// Open (or resume) a mission.
    ///
    /// * `fresh == true`: workdir must not contain mission state (refuses to
    ///   overwrite anything — §10).
    /// * `fresh == false`: resumes from `state.json`; refuses if the
    ///   specification changed since the mission started.
    pub async fn open(cfg: MissionRunConfig) -> Result<Self> {
        let crash_at = cfg.crash_at.clone();
        let spec_text = std::fs::read_to_string(&cfg.spec_path)
            .with_context(|| format!("reading spec {}", cfg.spec_path.display()))?;
        let (spec, spec_hash) = parse_spec(&spec_text)?;

        let workdir = cfg.workdir.clone();
        let project_dir = workdir.join("project");
        let mission_dir = workdir.join(".zylcode").join("first_mission");
        let state_path = mission_dir.join("state.json");
        let resuming = state_path.exists();

        if cfg.fresh && resuming {
            bail!(
                "mission state already exists at {}; refusing to start fresh over existing \
                 evidence (delete it explicitly if you really want to discard it)",
                state_path.display()
            );
        }
        if !resuming {
            // Unknown non-empty workdir without our state: refuse (§10),
            // whether or not --fresh was passed — `--fresh` only authorises
            // starting over *our own* mission state, never adopting foreign
            // content.
            if workdir.exists() {
                let has_content = std::fs::read_dir(&workdir)?.next().is_some();
                if has_content && !mission_dir.exists() {
                    bail!(
                        "workdir {} is non-empty but carries no ZylCode mission state; \
                         refusing to adopt it",
                        workdir.display()
                    );
                }
            }
        }

        std::fs::create_dir_all(&mission_dir)?;
        std::fs::create_dir_all(&project_dir)?;

        let ledger = SqliteLedgerStore::new(
            mission_dir
                .join("ledger.sqlite")
                .to_str()
                .context("utf-8 path")?,
        )?;
        let graph = EvidenceGraph::with_path(mission_dir.join("evidence_graph.json"));
        let claims = ClaimStore::with_path(mission_dir.join("claims.json"));
        let queue = MissionQueue::new(workdir.clone());

        let (mut state, plan, failures, last_material) = if resuming {
            let state: MissionState = serde_json::from_str(
                &std::fs::read_to_string(&state_path)
                    .with_context(|| format!("reading {}", state_path.display()))?,
            )
            .with_context(|| format!("corrupt mission state {}", state_path.display()))?;

            if state.spec_hash != spec_hash {
                bail!(
                    "specification changed since the mission started (recorded {}, now {}); \
                     refusing to resume against a different spec",
                    &state.spec_hash[..12.min(state.spec_hash.len())],
                    &spec_hash[..12]
                );
            }

            let plan: Plan =
                serde_json::from_str(&std::fs::read_to_string(mission_dir.join("plan.json"))?)?;
            let rebuilt = build_plan(&spec, &fixture_name(&cfg.fixture_path));
            if plan.steps.iter().map(|s| s.id.clone()).collect::<Vec<_>>()
                != rebuilt
                    .steps
                    .iter()
                    .map(|s| s.id.clone())
                    .collect::<Vec<_>>()
            {
                bail!("plan.json on disk does not match the plan rebuilt from the specification");
            }

            let failures: Vec<Failure> =
                match std::fs::read_to_string(mission_dir.join("failures.json")) {
                    Ok(s) if !s.trim().is_empty() => serde_json::from_str(&s)?,
                    _ => Vec::new(),
                };
            let entries = ledger.get_entries(state.session_id).await?;
            let last = entries
                .last()
                .map(entry_material_hash)
                .unwrap_or_else(|| "0".repeat(64));

            (state, plan, failures, last)
        } else {
            let plan = build_plan(&spec, &fixture_name(&cfg.fixture_path));
            let mission = queue.enqueue(&spec.intent, MissionMode::Build)?;
            let state = MissionState {
                schema_version: 1,
                mission_id: mission.id,
                session_id: Uuid::new_v4(),
                spec_hash: spec_hash.clone(),
                spec_path: cfg.spec_path.display().to_string(),
                fixture_path: cfg.fixture_path.display().to_string(),
                next_step: 0,
                resumed_count: 0,
                reconciled_steps: Vec::new(),
                status: RunStatus::Running,
                blocked_reason: None,
                last_node: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            let failures = Vec::new();
            let last = "0".repeat(64);
            (state, plan, failures, last)
        };

        if resuming {
            state.resumed_count += 1;
            state.status = RunStatus::Running;
            state.blocked_reason = None;
            state.updated_at = Utc::now();
        }

        let env = capture_environment()?;

        let mut m = FirstMission {
            spec,
            spec_hash,
            plan,
            state,
            env,
            workdir,
            project_dir,
            mission_dir,
            crash_at,
            ledger,
            graph,
            claims,
            queue,
            failures,
            last_material,
            last_entry_id: None,
        };

        if !resuming {
            // Genesis evidence: intent + specification + plan (§5 spine),
            // recorded before anything executes.
            let intent_node = m.graph.add_node(
                EvidenceNode::new(NodeKind::Intent, m.spec.intent.clone(), "human")
                    .with_evidence(EvidenceKind::Other, format!("spec-sha256:{}", m.spec_hash)),
            )?;
            let spec_node = m.graph.add_node(
                EvidenceNode::new(
                    NodeKind::Specification,
                    format!(
                        "specification document {} ({} requirements, {} operations)",
                        m.state.spec_path,
                        m.spec.requirements.len(),
                        m.spec.operations.len()
                    ),
                    "human",
                )
                .with_evidence(EvidenceKind::Other, format!("spec-sha256:{}", m.spec_hash)),
            )?;
            m.graph.add_edge(
                intent_node.id.clone(),
                spec_node.id.clone(),
                EdgeKind::DerivesFrom,
                None,
            )?;
            let plan_node = m.graph.add_node(EvidenceNode::new(
                NodeKind::Plan,
                format!("{}-step execution plan", m.plan.steps.len()),
                "zylcode:first_mission",
            ))?;
            m.graph.add_edge(
                spec_node.id.clone(),
                plan_node.id.clone(),
                EdgeKind::DerivesFrom,
                None,
            )?;
            m.state.last_node = Some(plan_node.id);

            // Intent as a claim: the human said it; the spec file is the
            // evidence. Observed, never Verified.
            let mut intent_claim = Claim::new(
                m.spec.intent.clone(),
                ClaimKind::Requirement,
                ClaimSource::Human,
                VerificationLevel::R1StructuralValidity,
            )
            .with_mission(m.state.mission_id.clone());
            intent_claim.add_evidence(EvidenceKind::Other, format!("spec-sha256:{}", m.spec_hash));
            intent_claim.observe()?;
            m.claims.record(intent_claim)?;

            m.save_state()?;
            m.write_plan_file()?;
        }

        Ok(m)
    }

    fn write_plan_file(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.plan)?;
        std::fs::write(self.plan_path(), json)?;
        Ok(())
    }

    fn save_state(&mut self) -> Result<()> {
        self.state.updated_at = Utc::now();
        let json = serde_json::to_string_pretty(&self.state)?;
        std::fs::write(self.state_path(), json)?;
        Ok(())
    }

    fn save_failures(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.failures)?;
        std::fs::write(self.failures_path(), json)?;
        Ok(())
    }

    // --- ledger -------------------------------------------------------------

    /// Append one evidence entry for a completed step (single entry per step:
    /// payload carries started/finished timestamps, so the lifecycle stays
    /// honest without post-append mutation, which would break the chain).
    async fn append_evidence(
        &mut self,
        step_id: &str,
        entry_id: Uuid,
        mut payload: serde_json::Value,
        error: Option<String>,
    ) -> Result<Uuid> {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "spec_hash".into(),
                serde_json::Value::String(self.spec_hash.clone()),
            );
            obj.insert("step_id".into(), serde_json::Value::String(step_id.into()));
        }
        let state = if error.is_some() {
            ExecutionState::Failed
        } else {
            ExecutionState::Executed
        };
        let entry = LedgerEntry {
            id: entry_id,
            session_id: self.state.session_id,
            action_id: Self::action_id(step_id),
            arguments: serde_json::json!({ "plan_step": step_id, "mission": self.state.mission_id }),
            state,
            prev_hash: self.last_material.clone(),
            timestamp: Utc::now(),
            payload: Some(payload),
            error,
        };
        self.last_material = entry_material_hash(&entry);
        self.ledger.append(entry).await?;
        self.last_entry_id = Some(entry_id);
        Ok(entry_id)
    }

    /// True if this step already has an Executed/Failed evidence entry on disk.
    async fn step_entry(&self, step_id: &str) -> Result<Option<LedgerEntry>> {
        let entries = self.ledger.get_entries(self.state.session_id).await?;
        let action = Self::action_id(step_id);
        Ok(entries.into_iter().rfind(|e| e.action_id == action))
    }

    // --- graph --------------------------------------------------------------

    /// Append a node to the §5 spine: linked from the current tip, carrying
    /// environment provenance. The tip lives in `state.last_node`, so the
    /// chain survives crash/resume without trusting stale checkpoints (the
    /// tip is refreshed from the document at the start of every run).
    fn push_spine(&mut self, mut node: EvidenceNode, relation: EdgeKind) -> Result<String> {
        node.environment
            .insert("os".into(), std::env::consts::OS.into());
        node.environment
            .insert("arch".into(), std::env::consts::ARCH.into());
        node.environment
            .insert("python".into(), self.env.python_version.clone());
        let stored = self.graph.add_node(node)?;
        if let Some(prev) = self.state.last_node.take() {
            self.graph
                .add_edge(prev, stored.id.clone(), relation, None)?;
        }
        self.state.last_node = Some(stored.id.clone());
        Ok(stored.id)
    }

    /// Build a spine node pre-wired to a ledger entry (§5: every node cites
    /// the evidence it summarises).
    fn step_node(kind: NodeKind, summary: String, entry: Uuid) -> EvidenceNode {
        EvidenceNode::new(kind, summary, "zylcode:first_mission")
            .with_evidence(EvidenceKind::LedgerEntry, entry.to_string())
    }

    /// Artifact node helper (also used by reconciliation paths).
    fn add_artifact_node(
        &mut self,
        step_id: &str,
        path: &str,
        sha: Option<&str>,
        label: &str,
    ) -> Result<String> {
        let mut node = EvidenceNode::new(
            NodeKind::Artifact,
            format!("{label}: {path}"),
            "zylcode:first_mission",
        );
        node.files_changed.push(path.to_string());
        if let Some(s) = sha {
            node.conclusion = Some(format!("sha256 {s}"));
        }
        node.unknowns = Some(format!("step {step_id}"));
        self.push_spine(node, EdgeKind::DerivesFrom)
    }

    // --- claims -------------------------------------------------------------

    /// Record a claim whose VERIFIED status rests on a ledger entry.
    fn record_verified_claim(
        &self,
        statement: &str,
        kind: ClaimKind,
        level: VerificationLevel,
        entry_id: Uuid,
    ) -> Result<String> {
        let mut c = Claim::new(statement, kind, ClaimSource::DeterministicTool, level)
            .with_mission(self.state.mission_id.clone());
        c.add_evidence(EvidenceKind::LedgerEntry, entry_id.to_string());
        c.promote_verified()?;
        let stored = self.claims.record(c)?;
        Ok(stored.id)
    }

    /// Record an evidence-backed observation (pass/fail nuance lives in the
    /// statement and in ClaimStatus, never in a fabricated Verified).
    fn record_observed_claim(
        &self,
        statement: &str,
        kind: ClaimKind,
        level: VerificationLevel,
        entry_id: Uuid,
    ) -> Result<String> {
        let mut c = Claim::new(statement, kind, ClaimSource::DeterministicTool, level)
            .with_mission(self.state.mission_id.clone());
        c.add_evidence(EvidenceKind::LedgerEntry, entry_id.to_string());
        c.observe()?;
        let stored = self.claims.record(c)?;
        Ok(stored.id)
    }

    // --- failure lifecycle helpers ------------------------------------------

    /// First open (non-terminal) failure. An empty `operation` matches any
    /// open failure — repair steps use it to pick up whatever the previous
    /// step diagnosed, without hard-coding its key.
    fn open_failure_for(&self, operation: &str) -> Option<usize> {
        self.failures.iter().position(|f| {
            (operation.is_empty() || f.operation == operation)
                && !matches!(f.status, FailureStatus::Recovered | FailureStatus::Blocked)
        })
    }

    fn persist_failure(&mut self, f: Failure, operation: &str) -> Result<String> {
        let id = f.id.clone();
        if let Some(pos) = self.failures.iter().position(|x| x.operation == operation) {
            self.failures[pos] = f;
        } else {
            self.failures.push(f);
        }
        self.save_failures()?;
        Ok(id)
    }

    // --- crash injection (harness only) --------------------------------------

    /// Die like a real process death: no destructors, no flush, no checkpoint.
    /// Only ever triggered when the caller asked for `crash_at` (the CLI reads
    /// `ZYLCODE_MISSION_CRASH_AT`; tests spawn real child processes).
    fn maybe_crash(&self, step_id: &str, phase: &str) {
        let want = self.crash_at.as_deref();
        let hit = match want {
            Some(w) if w == step_id && phase == "post_evidence" => true,
            Some(w) if w == format!("{step_id}:pre-evidence") && phase == "side_effects" => true,
            _ => false,
        };
        if hit {
            eprintln!(
                "[zylcode] harness interruption injected at step '{}' ({phase}); \
                 process aborting before checkpoint",
                step_id
            );
            std::process::abort();
        }
    }
}

fn fixture_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "fixture".into())
}

fn capture_environment() -> Result<MissionEnvironment> {
    let python = run_cmd(&["python".into(), "--version".into()], Path::new("."))
        .map(|o| {
            let s = format!("{}{}", o.stdout.trim(), o.stderr.trim());
            if s.is_empty() {
                "unknown".into()
            } else {
                s
            }
        })
        .unwrap_or_else(|_| "unavailable".into());
    let git_v = run_cmd(&["git".into(), "--version".into()], Path::new("."))
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_else(|_| "unavailable".into());
    Ok(MissionEnvironment {
        python_version: python,
        git_version: git_v,
        os: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        arch: std::env::consts::ARCH.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Run loop: execute, checkpoint, reconcile, resume (§10, §36 steps 3–9)
// ---------------------------------------------------------------------------

/// What reconciliation decided for one plan step.
enum Reconcile {
    /// No evidence and no side effects (or safe to re-run): execute normally.
    Execute,
    /// Evidence already on disk: skip execution, advance the checkpoint.
    Skip {
        entry_id: Option<Uuid>,
        note: String,
    },
}

impl FirstMission {
    /// Execute the mission from its checkpoint to completion (or block).
    pub async fn run(&mut self) -> Result<MissionOutcome> {
        if self.state.status == RunStatus::Completed && self.record_path().exists() {
            return self.build_outcome().await;
        }

        // Refresh the graph spine from the document itself: a crash may have
        // appended nodes after the last checkpoint, and `state.last_node` is
        // therefore stale (never trust a checkpoint over the evidence).
        if let Some(last) = self.graph.document()?.nodes.last().map(|n| n.id.clone()) {
            self.state.last_node = Some(last);
        }

        let mut cursor = self.state.next_step;
        while cursor < self.plan.steps.len() {
            let step = self.plan.steps[cursor].clone();

            match self.reconcile(&step).await? {
                Reconcile::Skip { entry_id, note } => {
                    tracing::info!(step = %step.id, %note, "first mission: skipped after reconciliation");
                    if !self.state.reconciled_steps.contains(&step.id) {
                        self.state.reconciled_steps.push(step.id.clone());
                    }
                    if entry_id.is_some() {
                        self.last_entry_id = entry_id;
                    }
                }
                Reconcile::Execute => {
                    if let Some(blocked) = self.exec_step(&step).await? {
                        self.handle_blocked(&step, &blocked).await?;
                        return self.build_outcome().await;
                    }
                }
            }

            cursor += 1;
            self.state.next_step = cursor;
            self.save_state()?;
            self.save_checkpoint().await;
        }

        self.state.status = RunStatus::Completed;
        self.save_state()?;
        self.build_outcome().await
    }

    /// Checkpoint discipline: state.json (advanced first) + the existing
    /// `SessionCheckpoint` ledger surface (§10 reuse, not a parallel system).
    async fn save_checkpoint(&self) {
        let context = serde_json::json!({
            "mission_id": self.state.mission_id,
            "next_step": self.state.next_step,
            "plan_steps": self.plan.steps.len(),
            "spec_hash": self.spec_hash,
        });
        let checkpoint = SessionCheckpoint {
            session_id: self.state.session_id,
            last_entry_id: self.last_entry_id.unwrap_or_else(Uuid::nil),
            agent_state: format!("first_mission:{}", self.state.status.as_str()),
            context,
            timestamp: Utc::now(),
        };
        if let Err(e) = self.ledger.save_checkpoint(checkpoint).await {
            tracing::warn!(error = %e, "first mission: checkpoint save failed (state.json is authoritative)");
        }
    }

    /// Decide whether a plan step still needs execution (§10: never blindly
    /// repeat; verify the filesystem against recorded evidence first).
    async fn reconcile(&mut self, step: &PlanStep) -> Result<Reconcile> {
        let entry = self.step_entry(&step.id).await?;

        // Map `op:N` to its operation, when the step is a spec operation.
        let op = if let Some(idx) = step.id.strip_prefix("op:") {
            idx.parse::<usize>()
                .ok()
                .and_then(|i| self.spec.operations.get(i).cloned())
        } else {
            None
        };

        if let Some(e) = entry {
            if e.state == ExecutionState::Failed {
                // A failed entry means the previous attempt blocked; re-run it
                // so the failure is re-evaluated against current reality.
                return Ok(Reconcile::Execute);
            }
            // Finalize's durable side effects are the record and the package:
            // evidence without them means the process died mid-step (between
            // the evidence append and the export). Re-run — it is idempotent.
            if step.id == "finalize"
                && !(self.record_path().exists() && self.repro_dir().join("manifest.json").exists())
            {
                return Ok(Reconcile::Execute);
            }
            // Evidence exists → side effects happened. Verify they still hold
            // for file-mutating steps (tamper / external edit detection).
            if let Some(Operation::Replace { path, old, new }) = &op {
                let file = self.project_dir.join(path);
                match std::fs::read_to_string(&file) {
                    Ok(s) if s.contains(new.as_str()) && !s.contains(old.as_str()) => {}
                    _ => {
                        return Ok(Reconcile::Execute);
                    }
                }
            }
            if let Some(Operation::Write { path, content }) = &op {
                let file = self.project_dir.join(path);
                let ok = std::fs::read(&file)
                    .map(|b| sha256_hex(&b) == sha256_hex(content.as_bytes()))
                    .unwrap_or(false);
                if !ok {
                    return Ok(Reconcile::Execute);
                }
            }
            let note = format!(
                "evidence entry {} on disk; side effects verified — not re-executed",
                e.id
            );
            return Ok(Reconcile::Skip {
                entry_id: Some(e.id),
                note,
            });
        }

        // No evidence entry. Did side effects land anyway (crash between
        // side effects and evidence)?
        match op.as_ref() {
            Some(Operation::Write { path, content }) => {
                let file = self.project_dir.join(path);
                let landed = std::fs::read(&file)
                    .map(|b| sha256_hex(&b) == sha256_hex(content.as_bytes()))
                    .unwrap_or(false);
                if landed {
                    // Reconstruct evidence from the durable artifact itself.
                    let eid = Uuid::new_v4();
                    let sha = sha256_hex(content.as_bytes());
                    self.append_evidence(
                        &step.id,
                        eid,
                        serde_json::json!({
                            "kind": "write",
                            "path": path,
                            "after_sha256": sha,
                            "bytes": content.len(),
                            "reconciled": true,
                            "note": "execution evidence was lost with an interrupted process; \
                                     reconstructed from the durable artifact on disk",
                        }),
                        None,
                    )
                    .await?;
                    self.add_artifact_node(
                        &step.id,
                        path,
                        Some(sha.as_str()),
                        "write (reconciled)",
                    )?;
                    return Ok(Reconcile::Skip {
                        entry_id: Some(eid),
                        note: "side effect present on disk; evidence reconstructed (§10)".into(),
                    });
                }
            }
            Some(Operation::Replace { path, old, new }) => {
                let file = self.project_dir.join(path);
                let landed = std::fs::read_to_string(&file)
                    .map(|s| s.contains(new.as_str()) && !s.contains(old.as_str()))
                    .unwrap_or(false);
                if landed {
                    let eid = Uuid::new_v4();
                    let sha = sha256_hex(std::fs::read(&file)?.as_ref());
                    self.append_evidence(
                        &step.id,
                        eid,
                        serde_json::json!({
                            "kind": "replace",
                            "path": path,
                            "after_sha256": sha,
                            "reconciled": true,
                            "note": "repair already present on disk; evidence reconstructed \
                                     from the durable artifact (§10)",
                        }),
                        None,
                    )
                    .await?;
                    self.add_artifact_node(
                        &step.id,
                        path,
                        Some(sha.as_str()),
                        "replace (reconciled)",
                    )?;
                    // The recovery attempt must be on record even when the
                    // process that made it died before recording it.
                    if let Some(fi) = self.failures.iter().position(|f| {
                        f.operation.starts_with("run:")
                            && !matches!(
                                f.status,
                                FailureStatus::Recovered | FailureStatus::Blocked
                            )
                    }) {
                        if self.failures[fi].retry_count == 0 {
                            self.failures[fi].attempt_recovery(format!(
                                "spec-directed replace in {path} (observed after interruption)"
                            ));
                            self.save_failures()?;
                        }
                    }
                    return Ok(Reconcile::Skip {
                        entry_id: Some(eid),
                        note: "repair present on disk; evidence reconstructed (§10)".into(),
                    });
                }
            }
            Some(Operation::Commit { message }) => {
                // git history is durable evidence of the commit.
                if let Ok(hash) = git_stdout(&["log", "-1", "--format=%H %s"], &self.project_dir) {
                    if hash.contains(message.as_str()) {
                        let eid = Uuid::new_v4();
                        let commit = hash.split_whitespace().next().unwrap_or("").to_string();
                        self.append_evidence(
                            &step.id,
                            eid,
                            serde_json::json!({
                                "kind": "commit",
                                "message": message,
                                "commit": commit,
                                "reconciled": true,
                                "note": "commit found in git history; evidence reconstructed (§10)",
                            }),
                            None,
                        )
                        .await?;
                        return Ok(Reconcile::Skip {
                            entry_id: Some(eid),
                            note: "commit already in git history (§10)".into(),
                        });
                    }
                }
            }
            Some(Operation::Run { .. }) | None => {
                // Verify-commands are read-only and idempotent: re-running is
                // always safe (and yields fresh, honest evidence).
            }
        }

        Ok(Reconcile::Execute)
    }

    /// Dispatch one plan step. Returns `Some(reason)` when the mission is
    /// blocked (never silently converted into success — §9/§18).
    async fn exec_step(&mut self, step: &PlanStep) -> Result<Option<String>> {
        match step.id.as_str() {
            "prepare" => self.exec_prepare(step).await,
            "baseline" => self.exec_baseline(step).await,
            "finalize" => self.exec_finalize(step).await,
            _ => {
                let idx: usize = step
                    .id
                    .strip_prefix("op:")
                    .context("malformed step id")?
                    .parse()
                    .context("malformed op index")?;
                let op = self
                    .spec
                    .operations
                    .get(idx)
                    .cloned()
                    .with_context(|| format!("operation {idx} missing from spec"))?;
                self.exec_op(step, &op).await
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Step executors (§36 steps 3–9: real tools, real failure, real repair)
// ---------------------------------------------------------------------------

/// Parse a structured diagnosis out of *captured* output (§36 step 7: the
/// cause comes from what the tool actually printed, never assumed).
fn diagnose_from_output(hay: &str) -> String {
    let mut facts: Vec<String> = Vec::new();
    for needle in ["AssertionError", "FAIL:", "ERROR:", "Traceback"] {
        if let Some(l) = first_line_containing(hay, needle) {
            if !facts.contains(&l) {
                facts.push(l);
            }
        }
    }
    if facts.is_empty() {
        facts.push(tail(hay.trim(), 300));
    }
    facts.join(" | ")
}

/// Deepest source frame in a Python traceback → the artifact to repair.
fn implicated_file(hay: &str) -> Option<String> {
    let mut found = None;
    for line in hay.lines() {
        if let Some(rest) = line.split("File \"").nth(1) {
            if let Some(path) = rest.split('\"').next() {
                if path.ends_with(".py") {
                    found = Some(path.replace('\\', "/"));
                }
            }
        }
    }
    found
}

impl FirstMission {
    /// Step `prepare`: copy the fixture, hygiene gitignore, baseline revision.
    async fn exec_prepare(&mut self, step: &PlanStep) -> Result<Option<String>> {
        let fixture = PathBuf::from(&self.state.fixture_path);
        if !fixture.is_dir() {
            let reason = format!("fixture directory {} does not exist", fixture.display());
            self.append_evidence(
                &step.id,
                Uuid::new_v4(),
                serde_json::json!({"kind": "prepare"}),
                Some(reason.clone()),
            )
            .await?;
            return Ok(Some(reason));
        }

        copy_dir_all(&fixture, &self.project_dir)?;

        // Keep runtime junk out of the repository so later commits reflect
        // the real change set.
        let gi = self.project_dir.join(".gitignore");
        let existing = std::fs::read_to_string(&gi).unwrap_or_default();
        if !existing.contains("__pycache__") {
            let mut next = existing;
            if !next.ends_with('\n') && !next.is_empty() {
                next.push('\n');
            }
            next.push_str("__pycache__/\n*.pyc\n");
            std::fs::write(&gi, next)?;
        }

        if !self.project_dir.join(".git").exists() {
            git(&["init", "-q"], &self.project_dir)?;
        }
        git(&["add", "-A"], &self.project_dir)?;

        let head_before = git_stdout(&["rev-parse", "HEAD"], &self.project_dir).ok();
        let status = git(&["status", "--porcelain"], &self.project_dir)?;
        let dirty = !String::from_utf8_lossy(&status.stdout).trim().is_empty();
        let mut committed_new = false;
        if head_before.is_none() || dirty {
            let out = Command::new("git")
                .args([
                    "-c",
                    "user.name=ZylCode First Mission",
                    "-c",
                    "user.email=mission@zylcode.local",
                    "commit",
                    "-q",
                    "-m",
                    "baseline: fixture as provided",
                ])
                .current_dir(&self.project_dir)
                .output()?;
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                // A clean tree with an existing HEAD means the baseline
                // revision already exists (crash between commit and
                // evidence): that is success, not failure.
                if head_before.is_none() {
                    let reason = format!("baseline commit failed: {err}");
                    self.append_evidence(
                        &step.id,
                        Uuid::new_v4(),
                        serde_json::json!({"kind": "prepare"}),
                        Some(reason.clone()),
                    )
                    .await?;
                    return Ok(Some(reason));
                }
            } else {
                committed_new = true;
            }
        }
        let head = git_stdout(&["rev-parse", "HEAD"], &self.project_dir)?;
        let file_count = walkdir::WalkDir::new(&self.project_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();

        self.maybe_crash(&step.id, "side_effects");

        let payload = serde_json::json!({
            "kind": "prepare",
            "fixture": self.state.fixture_path,
            "files_copied": file_count,
            "baseline_commit": head,
            "committed_new_revision": committed_new,
            "gitignore_extended": true,
        });
        let entry = self
            .append_evidence(&step.id, Uuid::new_v4(), payload, None)
            .await?;

        let mut action = Self::step_node(
            NodeKind::Action,
            format!(
                "prepare workspace from fixture '{}' ({} files)",
                fixture_name(&fixture),
                file_count
            ),
            entry,
        );
        action.commit = Some(head.clone());
        self.push_spine(action, EdgeKind::DerivesFrom)?;
        let mut artifact = EvidenceNode::new(
            NodeKind::Artifact,
            format!("baseline revision {head}"),
            "zylcode:first_mission",
        )
        .with_evidence(EvidenceKind::LedgerEntry, entry.to_string());
        artifact.commit = Some(head);
        self.push_spine(artifact, EdgeKind::Produces)?;

        self.maybe_crash(&step.id, "post_evidence");
        Ok(None)
    }

    /// Step `baseline`: the suite must pass *before* any modification (§36
    /// step 5 establishes the green starting point; a red baseline blocks).
    async fn exec_baseline(&mut self, step: &PlanStep) -> Result<Option<String>> {
        let out = run_cmd(&self.spec.verify_command, &self.project_dir)?;
        let haystack = out.haystack();

        self.maybe_crash(&step.id, "side_effects");

        let matched = out.spawn_ok && out.exit_code == 0;
        let payload = serde_json::json!({
            "kind": "baseline",
            "command": self.spec.verify_command,
            "exit_code": out.exit_code,
            "spawn_ok": out.spawn_ok,
            "duration_ms": out.duration_ms,
            "stdout_tail": tail(&out.stdout, 4000),
            "stderr_tail": tail(&out.stderr, 4000),
            "output_sha256": sha256_hex(haystack.as_bytes()),
        });
        let error = if matched {
            None
        } else {
            Some(format!(
                "baseline verification failed (exit {}): {}",
                out.exit_code,
                tail(haystack.trim(), 400)
            ))
        };
        let entry = self
            .append_evidence(&step.id, Uuid::new_v4(), payload, error.clone())
            .await?;

        let mut tool = Self::step_node(
            NodeKind::ToolCall,
            format!(
                "run baseline verification: {}",
                self.spec.verify_command.join(" ")
            ),
            entry,
        );
        tool.tool = Some(self.spec.verify_command[0].clone());
        tool.inputs = Some(serde_json::json!(&self.spec.verify_command));
        tool.outputs = Some(serde_json::json!({
            "exit_code": out.exit_code,
            "stdout_tail": tail(&out.stdout, 800),
            "stderr_tail": tail(&out.stderr, 800),
        }));
        self.push_spine(tool, EdgeKind::DerivesFrom)?;
        let mut result = EvidenceNode::new(
            NodeKind::Result,
            if matched {
                "baseline suite passed".to_string()
            } else {
                format!("baseline suite failed (exit {})", out.exit_code)
            },
            "zylcode:first_mission",
        )
        .with_evidence(EvidenceKind::LedgerEntry, entry.to_string());
        result.outputs = Some(serde_json::json!({"exit_code": out.exit_code}));
        self.push_spine(result, EdgeKind::Produces)?;

        if !matched {
            let reason = error.unwrap_or_else(|| "baseline verification failed".into());
            return Ok(Some(reason));
        }

        let mut verify = EvidenceNode::new(
            NodeKind::Verification,
            "baseline suite green before any modification",
            "zylcode:first_mission",
        )
        .with_evidence(EvidenceKind::LedgerEntry, entry.to_string());
        verify.verification = Some(self.spec.verify_command.join(" "));
        verify.conclusion = Some("passed (exit 0)".into());
        self.push_spine(verify, EdgeKind::DerivesFrom)?;

        self.record_verified_claim(
            "Baseline suite passes in the freshly prepared workspace before any modification.",
            ClaimKind::Test,
            VerificationLevel::R3TestVerified,
            entry,
        )?;

        self.maybe_crash(&step.id, "post_evidence");
        Ok(None)
    }

    /// Spec operation dispatch.
    async fn exec_op(&mut self, step: &PlanStep, op: &Operation) -> Result<Option<String>> {
        match op {
            Operation::Write { path, content } => self.exec_write(step, path, content).await,
            Operation::Run {
                id,
                expect,
                signature,
            } => self.exec_run(step, id, *expect, signature).await,
            Operation::Replace { path, old, new } => self.exec_replace(step, path, old, new).await,
            Operation::Commit { message } => self.exec_commit(step, message).await,
        }
    }

    async fn exec_write(
        &mut self,
        step: &PlanStep,
        path: &str,
        content: &str,
    ) -> Result<Option<String>> {
        let target = self.project_dir.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let before = std::fs::read(&target).ok().map(|b| sha256_hex(&b));
        std::fs::write(&target, content.as_bytes())?;
        // Post-condition: the bytes on disk are the bytes the spec demanded.
        let on_disk = std::fs::read(&target)?;
        if sha256_hex(&on_disk) != sha256_hex(content.as_bytes()) {
            let reason = format!(
                "write verification failed for {path}: disk content differs from specification"
            );
            self.append_evidence(
                &step.id,
                Uuid::new_v4(),
                serde_json::json!({"kind": "write", "path": path}),
                Some(reason.clone()),
            )
            .await?;
            return Ok(Some(reason));
        }

        self.maybe_crash(&step.id, "side_effects");

        let after = sha256_hex(&on_disk);
        let payload = serde_json::json!({
            "kind": "write",
            "path": path,
            "before_sha256": before,
            "after_sha256": after,
            "bytes": on_disk.len(),
        });
        let entry = self
            .append_evidence(&step.id, Uuid::new_v4(), payload, None)
            .await?;

        let mut action = Self::step_node(
            NodeKind::Action,
            format!("write {path} ({} bytes)", on_disk.len()),
            entry,
        );
        action.files_changed.push(path.to_string());
        self.push_spine(action, EdgeKind::DerivesFrom)?;
        let mut artifact = EvidenceNode::new(
            NodeKind::Artifact,
            format!("{path} written"),
            "zylcode:first_mission",
        )
        .with_evidence(EvidenceKind::LedgerEntry, entry.to_string());
        artifact.files_changed.push(path.to_string());
        artifact.conclusion = Some(format!("sha256 {after}"));
        self.push_spine(artifact, EdgeKind::Produces)?;

        self.maybe_crash(&step.id, "post_evidence");
        Ok(None)
    }

    async fn exec_run(
        &mut self,
        step: &PlanStep,
        run_id: &str,
        expect: Expect,
        signature: &[String],
    ) -> Result<Option<String>> {
        let out = run_cmd(&self.spec.verify_command, &self.project_dir)?;
        let haystack = out.haystack();

        self.maybe_crash(&step.id, "side_effects");

        let sigs_present: Vec<&String> = signature
            .iter()
            .filter(|s| haystack.contains(s.as_str()))
            .collect();
        let sigs_ok = sigs_present.len() == signature.len();
        let matched = match expect {
            Expect::Pass => out.spawn_ok && out.exit_code == 0 && sigs_ok,
            Expect::Fail => out.spawn_ok && out.exit_code != 0 && sigs_ok,
        };

        let payload = serde_json::json!({
            "kind": "run",
            "run_id": run_id,
            "command": self.spec.verify_command,
            "expect": expect,
            "exit_code": out.exit_code,
            "spawn_ok": out.spawn_ok,
            "duration_ms": out.duration_ms,
            "signatures": signature,
            "signatures_present": sigs_present.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            "matched_expectation": matched,
            "stdout_tail": tail(&out.stdout, 4000),
            "stderr_tail": tail(&out.stderr, 4000),
            "output_sha256": sha256_hex(haystack.as_bytes()),
        });
        let mismatch_reason = if matched {
            None
        } else {
            Some(format!(
                "run '{run_id}': expected {expect:?} but got exit {} with {}/{} required signatures \
                 (output: {})",
                out.exit_code,
                sigs_present.len(),
                signature.len(),
                tail(haystack.trim(), 400)
            ))
        };
        let entry = self
            .append_evidence(&step.id, Uuid::new_v4(), payload, mismatch_reason.clone())
            .await?;

        let mut tool = Self::step_node(
            NodeKind::ToolCall,
            format!("run '{run_id}': {}", self.spec.verify_command.join(" ")),
            entry,
        );
        tool.tool = Some(self.spec.verify_command[0].clone());
        tool.inputs = Some(serde_json::json!({
            "command": &self.spec.verify_command,
            "expect": expect,
            "signatures": signature,
        }));
        tool.outputs = Some(serde_json::json!({
            "exit_code": out.exit_code,
            "stdout_tail": tail(&out.stdout, 800),
            "stderr_tail": tail(&out.stderr, 800),
        }));
        self.push_spine(tool, EdgeKind::DerivesFrom)?;
        let mut result = EvidenceNode::new(
            NodeKind::Result,
            format!(
                "run '{run_id}' exited {} (expected {expect:?})",
                out.exit_code
            ),
            "zylcode:first_mission",
        )
        .with_evidence(EvidenceKind::LedgerEntry, entry.to_string());
        result.outputs = Some(serde_json::json!({"exit_code": out.exit_code}));
        result.unknowns = mismatch_reason.clone();
        self.push_spine(result, EdgeKind::Produces)?;

        if let Some(reason) = mismatch_reason {
            // Spec and reality disagree → the mission stops honestly (§18).
            return Ok(Some(reason));
        }

        // Mandated recoverable failure (§36 steps 6–7): an expected-failing
        // run is a real, structured, diagnosed failure — never dressed up.
        if expect == Expect::Fail {
            let cause = first_line_containing(&haystack, "AssertionError")
                .or_else(|| first_line_containing(&haystack, "FAIL:"))
                .unwrap_or_else(|| tail(haystack.trim(), 300));
            let diagnosis = diagnose_from_output(&haystack);
            let mut f = Failure::new(format!("run:{run_id}"), cause, FailureStatus::Failed)?
                .with_mission(self.state.mission_id.clone());
            if let Some(file) = implicated_file(&haystack) {
                f.affect_artifact(file);
            }
            f.diagnose(diagnosis);
            f.add_evidence(EvidenceKind::LedgerEntry, entry.to_string());
            self.persist_failure(f, &format!("run:{run_id}"))?;

            self.record_observed_claim(
                &format!(
                    "Run '{run_id}' fails as the specification requires: exit {} with signatures {:?} present.",
                    out.exit_code, signature
                ),
                ClaimKind::Test,
                VerificationLevel::R3TestVerified,
                entry,
            )?;
        } else {
            self.record_verified_claim(
                &format!(
                    "Run '{run_id}' passes: exit code 0 with signatures {:?} present (same verification command).",
                    signature
                ),
                ClaimKind::Test,
                VerificationLevel::R3TestVerified,
                entry,
            )?;

            // §36 step 9: a green re-run is what may upgrade an open failure
            // to RECOVERED — fail-closed, evidence-backed (§9).
            if let Some(fi) = self.open_failure_for("") {
                let ledger_ok = out.exit_code == 0 && sigs_ok;
                if ledger_ok {
                    let f = &mut self.failures[fi];
                    f.add_evidence(EvidenceKind::LedgerEntry, entry.to_string());
                    let op_key = f.operation.clone();
                    let op_name = f.operation.clone();
                    let eid = entry.to_string();
                    let resolution = format!(
                        "re-verification '{run_id}' passed on the same command (exit 0, evidence {eid}); \
                         diagnosed cause was repaired and re-checked"
                    );
                    match f.resolve_recovered(resolution) {
                        Ok(()) => {
                            self.persist_failure(
                                self.failures
                                    .iter()
                                    .find(|x| x.operation == op_key)
                                    .cloned()
                                    .context("failure vanished")?,
                                &op_key,
                            )?;
                            tracing::info!(operation = %op_name, "failure recovered with evidence");
                        }
                        Err(e) => {
                            // Refusal is itself the honest outcome: leave the
                            // failure open and say so in the record.
                            tracing::warn!(error = %e, operation = %op_name, "recovery refused by fail-closed guard");
                            self.save_failures()?;
                        }
                    }
                }
            }
        }

        let mut verify = EvidenceNode::new(
            NodeKind::Verification,
            format!("run '{run_id}' matched expectation {expect:?}"),
            "zylcode:first_mission",
        )
        .with_evidence(EvidenceKind::LedgerEntry, entry.to_string());
        verify.verification = Some(self.spec.verify_command.join(" "));
        verify.conclusion = Some(format!("exit {} matched {expect:?}", out.exit_code));
        self.push_spine(verify, EdgeKind::DerivesFrom)?;

        self.maybe_crash(&step.id, "post_evidence");
        Ok(None)
    }

    async fn exec_replace(
        &mut self,
        step: &PlanStep,
        path: &str,
        old: &str,
        new: &str,
    ) -> Result<Option<String>> {
        let target = self.project_dir.join(path);
        let content = match std::fs::read_to_string(&target) {
            Ok(s) => s,
            Err(e) => {
                let reason = format!("replace target {path} unreadable: {e}");
                self.append_evidence(
                    &step.id,
                    Uuid::new_v4(),
                    serde_json::json!({"kind": "replace", "path": path}),
                    Some(reason.clone()),
                )
                .await?;
                return Ok(Some(reason));
            }
        };
        let occurrences = content.matches(old).count();
        if occurrences != 1 {
            let reason = if occurrences == 0 && content.contains(new) {
                format!(
                    "replace target {path} already contains the replacement but has no mission \
                     evidence for it — an unknown actor changed the file; refusing to claim the repair"
                )
            } else if occurrences == 0 {
                format!("replace target {path} does not contain the expected source text")
            } else {
                format!(
                    "replace target {path} matches the source text {occurrences} times — \
                     ambiguous, refusing to guess which to replace"
                )
            };
            self.append_evidence(
                &step.id,
                Uuid::new_v4(),
                serde_json::json!({"kind": "replace", "path": path, "occurrences": occurrences}),
                Some(reason.clone()),
            )
            .await?;
            return Ok(Some(reason));
        }
        let before_sha = sha256_hex(content.as_bytes());
        let replaced = content.replacen(old, new, 1);
        std::fs::write(&target, replaced.as_bytes())?;

        // Post-condition before any claim is made.
        let check = std::fs::read_to_string(&target)?;
        if !check.contains(new) || check.contains(old) {
            let reason = format!("post-replace verification failed for {path}");
            self.append_evidence(
                &step.id,
                Uuid::new_v4(),
                serde_json::json!({"kind": "replace", "path": path}),
                Some(reason.clone()),
            )
            .await?;
            return Ok(Some(reason));
        }

        self.maybe_crash(&step.id, "side_effects");

        let after_sha = sha256_hex(check.as_bytes());
        let payload = serde_json::json!({
            "kind": "replace",
            "path": path,
            "before_sha256": before_sha,
            "after_sha256": after_sha,
            "replacements": 1,
        });
        let entry = self
            .append_evidence(&step.id, Uuid::new_v4(), payload, None)
            .await?;

        // §36 step 8: record the recovery attempt on any open failure.
        if let Some(fi) = self.open_failure_for("") {
            self.failures[fi]
                .attempt_recovery(format!("spec-directed replace in {path}: {old} -> {new}"));
            self.failures[fi].affect_artifact(path.to_string());
            self.failures[fi].add_evidence(EvidenceKind::LedgerEntry, entry.to_string());
            self.save_failures()?;
        }

        let mut action = Self::step_node(
            NodeKind::Action,
            format!("repair {path}: replace source expression"),
            entry,
        );
        action.files_changed.push(path.to_string());
        self.push_spine(action, EdgeKind::DerivesFrom)?;
        let mut artifact = EvidenceNode::new(
            NodeKind::Artifact,
            format!("{path} repaired"),
            "zylcode:first_mission",
        )
        .with_evidence(EvidenceKind::LedgerEntry, entry.to_string());
        artifact.files_changed.push(path.to_string());
        artifact.conclusion = Some(format!("sha256 {after_sha}"));
        self.push_spine(artifact, EdgeKind::Produces)?;

        self.maybe_crash(&step.id, "post_evidence");
        Ok(None)
    }

    async fn exec_commit(&mut self, step: &PlanStep, message: &str) -> Result<Option<String>> {
        let status = git(&["status", "--porcelain"], &self.project_dir)?;
        let clean = String::from_utf8_lossy(&status.stdout).trim().is_empty();
        if clean {
            let reason = format!(
                "commit '{message}' requested but the working tree is clean and no matching \
                 revision exists — nothing to record"
            );
            self.append_evidence(
                &step.id,
                Uuid::new_v4(),
                serde_json::json!({"kind": "commit", "message": message}),
                Some(reason.clone()),
            )
            .await?;
            return Ok(Some(reason));
        }
        git(&["add", "-A"], &self.project_dir)?;
        let out = Command::new("git")
            .args([
                "-c",
                "user.name=ZylCode First Mission",
                "-c",
                "user.email=mission@zylcode.local",
                "commit",
                "-q",
                "-m",
                message,
            ])
            .current_dir(&self.project_dir)
            .output()?;
        if !out.status.success() {
            let reason = format!(
                "commit failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
            self.append_evidence(
                &step.id,
                Uuid::new_v4(),
                serde_json::json!({"kind": "commit", "message": message}),
                Some(reason.clone()),
            )
            .await?;
            return Ok(Some(reason));
        }

        self.maybe_crash(&step.id, "side_effects");

        let rev = git_stdout(&["rev-parse", "HEAD"], &self.project_dir)?;
        let changed = git_stdout(&["show", "--stat", "--format=", "-1"], &self.project_dir)
            .unwrap_or_default();
        let payload = serde_json::json!({
            "kind": "commit",
            "message": message,
            "revision": rev,
            "stat": changed,
        });
        let entry = self
            .append_evidence(&step.id, Uuid::new_v4(), payload, None)
            .await?;

        let mut action = Self::step_node(NodeKind::Action, format!("commit: {message}"), entry);
        action.commit = Some(rev.clone());
        self.push_spine(action, EdgeKind::DerivesFrom)?;
        let mut artifact = EvidenceNode::new(
            NodeKind::Artifact,
            format!("revision {rev}"),
            "zylcode:first_mission",
        )
        .with_evidence(EvidenceKind::LedgerEntry, entry.to_string());
        artifact.commit = Some(rev);
        self.push_spine(artifact, EdgeKind::Produces)?;

        self.maybe_crash(&step.id, "post_evidence");
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Finalize, blocking, and the reported outcome (§36 steps 11–15, §9, §12)
// ---------------------------------------------------------------------------

fn failure_status_str(s: FailureStatus) -> &'static str {
    match s {
        FailureStatus::Failed => "failed",
        FailureStatus::Blocked => "blocked",
        FailureStatus::TimedOut => "timed_out",
        FailureStatus::Cancelled => "cancelled",
        FailureStatus::Unknown => "unknown",
        FailureStatus::PartiallyCompleted => "partially_completed",
        FailureStatus::Recovered => "recovered",
    }
}

/// One markdown table cell (no raw pipes or newlines).
fn md_cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

fn verdict(ok: bool) -> &'static str {
    if ok {
        "✓ OK"
    } else {
        "✗ BROKEN"
    }
}

impl FirstMission {
    /// Evidence integrity across all three chains (§36 step 10, §5). A chain
    /// that cannot be read is reported as broken — never as a silent pass.
    async fn compute_integrity(&self) -> Result<IntegrityReport> {
        let entries = self.ledger.get_entries(self.state.session_id).await?;
        let ledger_chain = verify_ledger_chain(&entries);
        let graph = self.graph.verify_integrity().unwrap_or_else(|e| {
            tracing::error!(error = %e, "evidence graph unreadable during integrity check");
            false
        });
        let claims_chain = self.claims.verify_chain().unwrap_or_else(|e| {
            tracing::error!(error = %e, "claim chain unreadable during integrity check");
            false
        });
        Ok(IntegrityReport {
            ledger_chain,
            graph,
            claims_chain,
        })
    }

    /// §36 step 13: classify what the mission actually established —
    /// verified facts vs derived requirements vs hypothesis vs unknown.
    /// Idempotent: a crash between recording and checkpointing must never
    /// duplicate claims on resume.
    fn classify_claims(&self) -> Result<Vec<Claim>> {
        let claims = self.claims.list()?;
        let mut seen: Vec<String> = claims.iter().map(|c| c.statement.clone()).collect();
        let premises: Vec<String> = claims
            .iter()
            .filter(|c| matches!(c.status, ClaimStatus::Verified | ClaimStatus::Observed))
            .map(|c| c.id.clone())
            .collect();
        let mut added = Vec::new();

        for req in &self.spec.requirements {
            if premises.is_empty() {
                // Nothing established at all — say so instead of deriving.
                let statement = format!(
                    "Requirement satisfaction NOT established: {req} (no verified or observed evidence recorded)"
                );
                if seen.contains(&statement) {
                    continue;
                }
                let mut c = Claim::new(
                    statement.as_str(),
                    ClaimKind::Requirement,
                    ClaimSource::DerivedComputation,
                    VerificationLevel::R0ModelOutput,
                )
                .with_mission(self.state.mission_id.clone());
                c.mark_unknown();
                let stored = self.claims.record(c)?;
                seen.push(statement);
                added.push(stored);
                continue;
            }

            let statement = format!("Requirement appears satisfied: {req}");
            if seen.contains(&statement) {
                continue;
            }
            let mut c = Claim::new(
                statement.as_str(),
                ClaimKind::Requirement,
                ClaimSource::DerivedComputation,
                VerificationLevel::R1StructuralValidity,
            )
            .with_mission(self.state.mission_id.clone());
            for p in &premises {
                c.add_evidence(EvidenceKind::Claim, p);
            }
            c.derive(
                "derived by inspection from the mission's verified/observed claims \
                 (premises cited as evidence refs); no automated requirement tracing \
                 exists yet — this stays DERIVED, never VERIFIED",
            )?;
            let stored = self.claims.record(c)?;
            seen.push(statement);
            added.push(stored);
        }

        // Hypothesis (§4): the honest status of model-driven repair.
        let hypothesis =
            "Model-driven repair (without the specification-directed edit) would also fix this defect";
        if !seen.iter().any(|s| s == hypothesis) {
            let mut c = Claim::new(
                hypothesis,
                ClaimKind::Behavior,
                ClaimSource::Model,
                VerificationLevel::R0ModelOutput,
            )
            .with_mission(self.state.mission_id.clone());
            c.mark_hypothesis();
            added.push(self.claims.record(c)?);
        }

        // Unknown (§18): never presented as satisfied.
        let unknown =
            "Production readiness of the fixture project (real workloads, other environments) is unknown";
        if !seen.iter().any(|s| s == unknown) {
            let mut c = Claim::new(
                unknown,
                ClaimKind::Other,
                ClaimSource::Human,
                VerificationLevel::R0ModelOutput,
            )
            .with_mission(self.state.mission_id.clone());
            c.mark_unknown();
            added.push(self.claims.record(c)?);
        }

        Ok(added)
    }

    /// §36 step 12: the Engineering Record — a human-auditable rendering of
    /// exactly what the mission established (the §12 truth surface on disk).
    async fn write_record(&self, status_line: &str, finalize_pending: bool) -> Result<PathBuf> {
        let entries = self.ledger.get_entries(self.state.session_id).await?;
        let integrity = self.compute_integrity().await?;
        let claims = self.claims.list()?;
        let node_count = self.graph.document()?.nodes.len();

        let mut md = String::new();
        md.push_str("# Engineering Record — ZylCode First Mission\n\n");
        md.push_str("| field | value |\n|---|---|\n");
        md.push_str(&format!(
            "| mission | `{}` |\n",
            md_cell(&self.state.mission_id)
        ));
        md.push_str(&format!("| status | {status_line} |\n"));
        md.push_str(&format!(
            "| specification | `{}` (sha256 `{}`) |\n",
            md_cell(&self.state.spec_path),
            self.spec_hash
        ));
        md.push_str(&format!(
            "| fixture | `{}` |\n",
            md_cell(&self.state.fixture_path)
        ));
        md.push_str(&format!("| intent | {} |\n", md_cell(&self.spec.intent)));
        md.push_str(&format!(
            "| verify command | `{}` |\n",
            md_cell(&self.spec.verify_command.join(" "))
        ));
        md.push_str(&format!(
            "| created | {} |\n",
            self.state.created_at.to_rfc3339()
        ));
        md.push_str(&format!(
            "| updated | {} |\n",
            self.state.updated_at.to_rfc3339()
        ));
        md.push_str(&format!(
            "| interruptions survived | {} |\n",
            self.state.resumed_count
        ));
        md.push_str(&format!(
            "| reconciled on resume | {} |\n",
            if self.state.reconciled_steps.is_empty() {
                "none".to_string()
            } else {
                self.state.reconciled_steps.join(", ")
            }
        ));
        md.push_str(&format!(
            "| environment | {} · {} · {} |\n\n",
            md_cell(&self.env.python_version),
            md_cell(&self.env.git_version),
            md_cell(&self.env.os),
        ));

        // --- per-step evidence ------------------------------------------------
        let mut step_status: BTreeMap<String, (String, String)> = BTreeMap::new();
        for e in &entries {
            let kind = e
                .payload
                .as_ref()
                .and_then(|p| p.get("kind"))
                .and_then(|k| k.as_str())
                .unwrap_or("-")
                .to_string();
            let (label, detail) = match e.state {
                ExecutionState::Failed => (
                    "FAILED".to_string(),
                    e.error
                        .clone()
                        .unwrap_or_else(|| "no reason recorded".into()),
                ),
                _ => ("EXECUTED".to_string(), kind),
            };
            step_status.insert(e.action_id.clone(), (label, detail));
        }
        md.push_str("## Plan and per-step evidence\n\n");
        md.push_str("| step | description | evidence | detail |\n|---|---|---|---|\n");
        for step in &self.plan.steps {
            let (label, detail) = match step_status.get(&Self::action_id(&step.id)) {
                Some(v) => v.clone(),
                None if finalize_pending && step.id == "finalize" => (
                    "WRITTEN HERE".to_string(),
                    "this record precedes its own finalize evidence entry (no \
                     self-reference); that entry cites this record's sha256"
                        .to_string(),
                ),
                None => ("NO EVIDENCE".to_string(), "not executed".to_string()),
            };
            md.push_str(&format!(
                "| `{}` | {} | {label} | {} |\n",
                step.id,
                md_cell(&step.description),
                md_cell(&detail),
            ));
        }
        md.push('\n');

        // --- claims by epistemic state ---------------------------------------
        md.push_str("## Claims by epistemic state (§4)\n\n");
        let groups: [(&str, ClaimStatus); 7] = [
            (
                "PROVEN — verified with cited evidence at R2+ (fail-closed gate)",
                ClaimStatus::Verified,
            ),
            (
                "OBSERVED — a deterministic tool captured it",
                ClaimStatus::Observed,
            ),
            (
                "DERIVED — follows from cited premises by stated reasoning",
                ClaimStatus::Derived,
            ),
            (
                "HYPOTHESIS — reasoned prediction, no confirming evidence",
                ClaimStatus::Hypothesis,
            ),
            ("UNKNOWN — nothing established", ClaimStatus::Unknown),
            (
                "UNVERIFIED — asserted without evidence",
                ClaimStatus::Unverified,
            ),
            (
                "CONTRADICTED — cited evidence conflicts",
                ClaimStatus::Contradicted,
            ),
        ];
        for (label, status) in groups {
            let items: Vec<&Claim> = claims.iter().filter(|c| c.status == status).collect();
            if items.is_empty() {
                continue;
            }
            md.push_str(&format!("### {label}\n\n"));
            for c in items {
                let st = format!("{:?}", c.status);
                let mut refs: Vec<String> = c
                    .evidence_refs
                    .iter()
                    .map(|r| format!("{:?}:{}", r.kind, r.id))
                    .collect();
                let total = refs.len();
                refs.truncate(4);
                let ref_txt = if total > refs.len() {
                    format!("{} (+{} more)", refs.join(", "), total - refs.len())
                } else {
                    refs.join(", ")
                };
                md.push_str(&format!(
                    "- **{}** — {st} @ level {} · evidence: `{}`\n",
                    md_cell(&c.statement),
                    c.verification_level.as_str(),
                    if ref_txt.is_empty() {
                        "none".to_string()
                    } else {
                        ref_txt
                    },
                ));
                if let Some(note) = &c.note {
                    md.push_str(&format!("  - reasoning: {}\n", md_cell(note)));
                }
            }
            md.push('\n');
        }

        // --- failures ----------------------------------------------------------
        md.push_str("## Failures (§9)\n\n");
        if self.failures.is_empty() {
            md.push_str("_No failures recorded._\n\n");
        } else {
            for f in &self.failures {
                md.push_str(&format!(
                    "### `{}` — {}\n\n",
                    md_cell(&f.operation),
                    failure_status_str(f.status)
                ));
                md.push_str(&format!("- **cause:** {}\n", md_cell(&f.cause)));
                if let Some(d) = &f.diagnosis {
                    md.push_str(&format!("- **diagnosis:** {}\n", md_cell(d)));
                }
                md.push_str(&format!(
                    "- **recovery:** attempt #{} — {}\n",
                    f.retry_count,
                    md_cell(&f.recovery_strategy)
                ));
                if let Some(r) = &f.resolution {
                    md.push_str(&format!("- **resolution:** {}\n", md_cell(r)));
                }
                if !f.affected_artifacts.is_empty() {
                    md.push_str(&format!(
                        "- **affected artifacts:** {}\n",
                        md_cell(&f.affected_artifacts.join(", "))
                    ));
                }
                md.push_str(&format!("- **evidence refs:** {}\n", f.evidence_refs.len()));
                md.push('\n');
            }
        }

        // --- integrity ---------------------------------------------------------
        md.push_str("## Evidence integrity\n\n");
        md.push_str("| chain | result |\n|---|---|\n");
        md.push_str(&format!(
            "| ledger prev-hash chain ({} entries) | {} |\n",
            entries.len(),
            verdict(integrity.ledger_chain)
        ));
        md.push_str(&format!(
            "| evidence graph ({} nodes) | {} |\n",
            node_count,
            verdict(integrity.graph)
        ));
        md.push_str(&format!(
            "| claim chain ({} claims) | {} |\n\n",
            claims.len(),
            verdict(integrity.claims_chain)
        ));

        // --- honest gaps -------------------------------------------------------
        md.push_str("## What this record does NOT establish (§18)\n\n");
        md.push_str("- **Model-driven repair:** not attempted — the edit was specification-directed. Whether an autonomous model reaches the same fix is a HYPOTHESIS above, not a verified fact.\n");
        md.push_str("- **Other environments:** one OS/arch/Python version exercised; portability is UNKNOWN.\n");
        md.push_str("- **Production readiness:** UNKNOWN — no deployment, load, or adversarial exercise is evidenced here.\n");
        md.push_str("- **ZylCode as a product:** this record certifies this mission's fixture only; product-wide status lives in `docs/governance/ENGINEERING_TRUTH.md`.\n");
        if self.state.status == RunStatus::Blocked {
            md.push_str("- **Remaining plan steps:** did not execute — their rows above read NO EVIDENCE, and no claim may rest on them.\n");
        }
        md.push('\n');

        // --- reproduce ---------------------------------------------------------
        md.push_str("## Reproduce and inspect\n\n");
        md.push_str(&format!(
            "- **Reproducibility package:** `{}` (manifest.json, checksums.sha256, ledger.jsonl, claims.json, evidence_graph.json, failures.json, plan.json, state.json, specification.md, engineering_record.md)\n",
            self.repro_dir().display()
        ));
        md.push_str(
            "- **Verify the package:** `cd <package dir> && sha256sum -c checksums.sha256`\n",
        );
        md.push_str(&format!(
            "- **Re-run verification:** `cd {} && {}`\n",
            self.project_dir.display(),
            self.spec.verify_command.join(" ")
        ));
        md.push_str("- **Verify the evidence chains:** `verify_ledger_chain()` over ledger.jsonl, `EvidenceGraph::verify_integrity()`, `ClaimStore::verify_chain()` — or resume the mission and watch reconciliation refuse to repeat completed work.\n");
        md.push_str(&format!(
            "- **Raw checkpoint:** `{}` (state.json advances only after each step's evidence is durable)\n\n",
            self.state_path().display()
        ));
        md.push_str(&format!(
            "_Written {} from the live evidence stores; counts above reflect the moment of writing._\n",
            Utc::now().to_rfc3339()
        ));
        if finalize_pending {
            md.push_str(
                "_The finalize evidence entry follows this record and cites its sha256; \
                 ledger counts above therefore exclude that one entry._\n",
            );
        }

        let path = self.record_path();
        std::fs::write(&path, &md)
            .with_context(|| format!("writing engineering record {}", path.display()))?;
        Ok(path)
    }

    /// §36 step 15: reproducibility package — inputs, outputs, environment,
    /// raw evidence, and hashes a third party can check without ZylCode.
    async fn export_repro(&self) -> Result<PathBuf> {
        let dir = self.repro_dir();
        if dir.exists() {
            // Never merge into a stale package: a partial previous export must
            // not survive alongside a new one (mixed evidence is worse than none).
            std::fs::remove_dir_all(&dir)
                .with_context(|| format!("clearing stale package {}", dir.display()))?;
        }
        std::fs::create_dir_all(&dir)?;

        let mut missing: Vec<&str> = Vec::new();
        let copies: [(PathBuf, &'static str); 6] = [
            (self.state_path(), "state.json"),
            (self.plan_path(), "plan.json"),
            (self.failures_path(), "failures.json"),
            (self.claims.path().to_path_buf(), "claims.json"),
            (self.graph.path().to_path_buf(), "evidence_graph.json"),
            (self.record_path(), "engineering_record.md"),
        ];
        for (src, name) in &copies {
            if src.exists() {
                std::fs::copy(src, dir.join(name))
                    .with_context(|| format!("copying {} into package", src.display()))?;
            } else {
                missing.push(name);
            }
        }

        // The raw ledger: chain-verifiable offline (step 10 evidence).
        let entries = self.ledger.get_entries(self.state.session_id).await?;
        let mut jsonl = String::new();
        for e in &entries {
            jsonl.push_str(&serde_json::to_string(e)?);
            jsonl.push('\n');
        }
        std::fs::write(dir.join("ledger.jsonl"), jsonl)?;

        // The original human specification, with drift detection.
        let mut spec_drift = false;
        match std::fs::read_to_string(&self.state.spec_path) {
            Ok(text) => {
                spec_drift = sha256_hex(text.as_bytes()) != self.spec_hash;
                std::fs::write(dir.join("specification.md"), text)?;
            }
            Err(_) => missing.push("specification.md"),
        }

        // Hash every packaged file (BTreeMap → stable, sorted output).
        let mut checksums: BTreeMap<String, String> = BTreeMap::new();
        let mut file_rows: Vec<serde_json::Value> = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let sha = sha256_file(&path)?;
            file_rows.push(serde_json::json!({
                "path": name,
                "sha256": sha,
                "bytes": entry.metadata()?.len(),
            }));
            checksums.insert(name, sha);
        }

        let manifest = serde_json::json!({
            "schema_version": 1,
            "kind": "zylcode-first-mission-reproducibility-package",
            "mission_id": &self.state.mission_id,
            "status": self.state.status.as_str(),
            "blocked_reason": &self.state.blocked_reason,
            "created_at": self.state.created_at.to_rfc3339(),
            "exported_at": Utc::now().to_rfc3339(),
            "spec": {
                "path": &self.state.spec_path,
                "sha256": &self.spec_hash,
                "drifted_since_start": spec_drift,
            },
            "verify_command": &self.spec.verify_command,
            "environment": &self.env,
            "resumed_count": self.state.resumed_count,
            "reconciled_steps": &self.state.reconciled_steps,
            "counts": {
                "ledger_entries": entries.len(),
                "graph_nodes": self.graph.document()?.nodes.len(),
                "claims": self.claims.list()?.len(),
                "failures": self.failures.len(),
            },
            "project_head": git_stdout(&["rev-parse", "HEAD"], &self.project_dir).ok(),
            "files": file_rows,
            "missing": missing,
            "checksums": "checksums.sha256 covers every file in this package except itself; verify with `sha256sum -c checksums.sha256`",
        });
        let manifest_path = dir.join("manifest.json");
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
        checksums.insert("manifest.json".to_string(), sha256_file(&manifest_path)?);

        let mut sums = String::new();
        for (name, sha) in &checksums {
            sums.push_str(&format!("{sha}  {name}\n"));
        }
        std::fs::write(dir.join("checksums.sha256"), sums)?;

        Ok(dir)
    }

    /// Step `finalize` (§36 steps 11–13, 15): fail-closed integrity gate,
    /// claim classification, engineering record, reproducibility package.
    async fn exec_finalize(&mut self, step: &PlanStep) -> Result<Option<String>> {
        // 1. Gate: evidence that does not verify can never be certified.
        let integrity = self.compute_integrity().await?;
        if !(integrity.ledger_chain && integrity.graph && integrity.claims_chain) {
            let reason = format!(
                "evidence integrity check FAILED at finalize (ledger_chain={}, graph={}, claims_chain={}) — refusing to certify the mission",
                integrity.ledger_chain, integrity.graph, integrity.claims_chain
            );
            let entry = self
                .append_evidence(
                    &step.id,
                    Uuid::new_v4(),
                    serde_json::json!({ "kind": "finalize_integrity_failed", "integrity": &integrity }),
                    Some(reason.clone()),
                )
                .await?;
            let mut node = Self::step_node(
                NodeKind::Verification,
                "finalize refused: evidence integrity check failed".to_string(),
                entry,
            );
            node.conclusion = Some("INTEGRITY BROKEN — mission blocked".to_string());
            node.unknowns = Some(reason.clone());
            self.push_spine(node, EdgeKind::DerivesFrom)?;
            return Ok(Some(reason));
        }

        // 2. Classification (§36 step 13) — idempotent across crash/resume.
        let classified = self.classify_claims()?;

        // 3. Side effect: the engineering record. The package is exported
        //    AFTER the evidence entry below so the packaged ledger is
        //    complete (it must contain this very entry).
        let record = self.write_record("**COMPLETED**", true).await?;
        self.maybe_crash(&step.id, "side_effects");

        // 4. Evidence: this entry cites the record's hash (the record itself
        //    cannot cite the entry that follows it — no self-reference loop).
        let record_sha = sha256_file(&record)?;
        let entries = self.ledger.get_entries(self.state.session_id).await?;
        let integrity = self.compute_integrity().await?;
        let payload = serde_json::json!({
            "kind": "finalize",
            "integrity": &integrity,
            "counts": {
                "ledger_entries": entries.len(),
                "graph_nodes": self.graph.document()?.nodes.len(),
                "claims": self.claims.list()?.len(),
                "failures": self.failures.len(),
                "note": "counts exclude this entry itself",
            },
            "record_sha256": record_sha,
            "record_path": record.display().to_string(),
            "package_path": self.repro_dir().display().to_string(),
        });
        let entry = self
            .append_evidence(&step.id, Uuid::new_v4(), payload, None)
            .await?;

        // 5. Reproducibility package (§36 step 15), now holding the complete
        //    ledger including this finalize entry.
        let package = self.export_repro().await?;

        // 6. Graph (§5): verification node, artifact node, claim nodes.
        let mut verify = Self::step_node(
            NodeKind::Verification,
            "finalize: evidence chains verified; record and reproducibility package produced"
                .to_string(),
            entry,
        );
        verify.verification = Some(format!(
            "ledger_chain={} graph={} claims_chain={}",
            integrity.ledger_chain, integrity.graph, integrity.claims_chain
        ));
        verify.conclusion = Some(format!("record sha256 {record_sha}"));
        self.push_spine(verify, EdgeKind::DerivesFrom)?;

        let mut artifact = EvidenceNode::new(
            NodeKind::Artifact,
            "engineering record + reproducibility package".to_string(),
            "zylcode:first_mission",
        )
        .with_evidence(EvidenceKind::LedgerEntry, entry.to_string());
        artifact.files_changed.push(record.display().to_string());
        artifact.files_changed.push(package.display().to_string());
        artifact.conclusion = Some(format!("record sha256 {record_sha}"));
        self.push_spine(artifact, EdgeKind::Produces)?;

        for c in &classified {
            let mut node = EvidenceNode::new(
                NodeKind::Claim,
                c.statement.clone(),
                "zylcode:first_mission",
            )
            .with_evidence(EvidenceKind::Claim, c.id.clone());
            node.conclusion = Some(format!(
                "{:?} @ {}",
                c.status,
                c.verification_level.as_str()
            ));
            self.push_spine(node, EdgeKind::DerivesFrom)?;
        }

        self.maybe_crash(&step.id, "post_evidence");
        Ok(None)
    }

    /// §9/§18: block the mission — structured failure, durable state, visible
    /// record. A block is never converted into success (the run loop returns
    /// the outcome immediately after this call).
    async fn handle_blocked(&mut self, step: &PlanStep, reason: &str) -> Result<()> {
        let op_key = format!("step:{}", step.id);
        let mut f = Failure::new(op_key.as_str(), reason, FailureStatus::Failed)?
            .with_mission(self.state.mission_id.clone());
        if let Some(id) = self.last_entry_id {
            f.add_evidence(EvidenceKind::LedgerEntry, id.to_string());
        }
        f.affect_artifact(format!("plan step '{}'", step.id));
        f.resolve_blocked(format!(
            "mission blocked at step '{}': {} — state persisted; a resume re-evaluates against current reality",
            step.id, reason
        ))?;
        f.validate()?;
        self.persist_failure(f, &op_key)?;

        // Durable mission state first (§10): everything else may re-run, but
        // the block itself must never be lost.
        self.state.status = RunStatus::Blocked;
        self.state.blocked_reason = Some(reason.to_string());
        self.save_state()?;
        self.save_checkpoint().await;

        // Mission-level evidence + graph decision node.
        let entry = self
            .append_evidence(
                "mission",
                Uuid::new_v4(),
                serde_json::json!({ "kind": "blocked", "step": &step.id, "reason": reason }),
                Some(reason.to_string()),
            )
            .await?;
        let mut node = Self::step_node(
            NodeKind::Decision,
            format!("mission blocked at step '{}': {}", step.id, reason),
            entry,
        );
        node.unknowns = Some(reason.to_string());
        self.push_spine(node, EdgeKind::DerivesFrom)?;

        // Record + package exist for blocked missions too (§18: the truth
        // surface shows blocks as clearly as completions).
        let status_line = format!("**BLOCKED** — {reason}");
        self.write_record(&status_line, false).await?;
        self.export_repro().await?;
        Ok(())
    }

    /// The closing statement (§35): what was verified, what is only derived
    /// or hypothesised, what is unknown — each with its evidence paths.
    async fn build_outcome(&mut self) -> Result<MissionOutcome> {
        let claims = self.claims.list()?;
        let mut verified = Vec::new();
        let mut observed = Vec::new();
        let mut derived = Vec::new();
        let mut hypotheses = Vec::new();
        let mut unknowns = Vec::new();
        for c in &claims {
            match c.status {
                ClaimStatus::Verified => verified.push(c.statement.clone()),
                ClaimStatus::Observed => observed.push(c.statement.clone()),
                ClaimStatus::Derived => derived.push(c.statement.clone()),
                ClaimStatus::Hypothesis => hypotheses.push(c.statement.clone()),
                ClaimStatus::Unknown => unknowns.push(c.statement.clone()),
                ClaimStatus::Unverified | ClaimStatus::Contradicted => {}
            }
        }
        let integrity = self.compute_integrity().await?;

        let (status, blocked_reason) = match self.state.status {
            RunStatus::Completed => (MissionOutcomeStatus::Completed, None),
            RunStatus::Blocked => (
                MissionOutcomeStatus::Blocked,
                self.state.blocked_reason.clone(),
            ),
            // Defensive (§18): never report completion for a mission that has
            // not finished. Unreachable from run(), which only asks for an
            // outcome once Completed/Blocked is durable.
            RunStatus::Running => (
                MissionOutcomeStatus::Blocked,
                Some(self.state.blocked_reason.clone().unwrap_or_else(|| {
                    "mission state is 'running': outcome not final".to_string()
                })),
            ),
        };

        // Keep the MissionQueue surface (existing product feature) truthful.
        match status {
            MissionOutcomeStatus::Completed => {
                let recovered = self
                    .failures
                    .iter()
                    .filter(|f| f.status == FailureStatus::Recovered)
                    .count();
                let summary = format!(
                    "first mission completed: {} verified, {} observed, {} derived, \
                     {} failures ({} recovered), interrupted {}x",
                    verified.len(),
                    observed.len(),
                    derived.len(),
                    self.failures.len(),
                    recovered,
                    self.state.resumed_count,
                );
                self.queue.finish(
                    &self.state.mission_id,
                    true,
                    &summary,
                    Some(self.state.session_id.to_string()),
                );
            }
            MissionOutcomeStatus::Blocked => {
                let reason = blocked_reason
                    .clone()
                    .unwrap_or_else(|| "blocked (no reason recorded)".to_string());
                self.queue.block(&self.state.mission_id, &reason);
            }
        }

        Ok(MissionOutcome {
            mission_id: self.state.mission_id.clone(),
            status,
            blocked_reason,
            workdir: self.workdir.clone(),
            mission_dir: self.mission_dir.clone(),
            record_path: self.record_path(),
            package_path: self.repro_dir(),
            verified_claims: verified,
            observed_claims: observed,
            derived_claims: derived,
            hypotheses,
            unknowns,
            failure_count: self.failures.len(),
            resumed_count: self.state.resumed_count,
            reconciled_steps: self.state.reconciled_steps.len(),
            integrity,
        })
    }
}

/// Library-level entry point behind the `zylcode mission` CLI commands:
/// open (or resume) a First Mission and run it to an outcome.
pub async fn run_first_mission(cfg: MissionRunConfig) -> Result<MissionOutcome> {
    let mut mission = FirstMission::open(cfg).await?;
    mission.run().await
}
