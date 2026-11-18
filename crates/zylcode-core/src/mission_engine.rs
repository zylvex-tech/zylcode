//! Phase 3B Mission Engine: the durable mission state machine.
//!
//! # Why this exists
//!
//! The Mission Queue ([`crate::missions`]) tracks *scheduling*: queued, running,
//! parked, done. That is a queue, not a mission. This module implements the
//! Phase 3B contract: a mission has an objective, explicit acceptance
//! criteria, and one of eleven states. It cannot reach `COMPLETE` while any
//! criterion lacks a **cited** proof, and it survives the process dying in the
//! middle of `EXECUTING`.
//!
//! # Durability model
//!
//! The journal (`mission.journal.jsonl`) is the source of truth. Every accepted
//! change appends one line containing the *complete* mission state after the
//! change, flushed with `fsync`. A crash during a write can therefore only
//! leave an incomplete final line, which is discarded on load — the previous
//! complete state is still on disk. `mission.json` is a derived snapshot for
//! humans and tools; when it lags the journal it is rewritten on load, and the
//! load report says so rather than pretending nothing happened.
//!
//! Refusals are deliberate: an interior unparsable line, a sequence gap, or a
//! journal that is missing entirely under a snapshot is corruption, and this
//! module returns an error instead of guessing. A mission engine that invents
//! its own past is worse than one that refuses to answer.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Schema version of the engine's on-disk state.
pub const MISSION_ENGINE_SCHEMA_VERSION: u32 = 1;

/// The eleven states of a mission (Phase 3B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Phase {
    Created,
    Understanding,
    Planning,
    AwaitingApproval,
    Executing,
    Verifying,
    Repairing,
    Blocked,
    Complete,
    Failed,
    Cancelled,
}

impl Phase {
    /// Terminal phases accept no further transitions.
    pub fn is_terminal(self) -> bool {
        matches!(self, Phase::Complete | Phase::Failed | Phase::Cancelled)
    }

    /// Whether `self` may legally move to `next`.
    ///
    /// `Blocked` is resumable through `Repairing` (the plan needs work) or
    /// `Planning` (the world changed); `Failed`, `Cancelled` and `Complete` are
    /// final, because reopening them would rewrite history.
    pub fn can_transition_to(self, next: Phase) -> bool {
        use Phase::*;
        match self {
            Created => matches!(next, Understanding | Cancelled),
            Understanding => matches!(next, Planning | Blocked | Failed | Cancelled),
            Planning => matches!(next, AwaitingApproval | Blocked | Failed | Cancelled),
            AwaitingApproval => matches!(next, Executing | Blocked | Cancelled),
            Executing => matches!(next, Verifying | Repairing | Blocked | Failed | Cancelled),
            Verifying => matches!(next, Complete | Repairing | Blocked | Failed | Cancelled),
            Repairing => matches!(next, Executing | Blocked | Failed | Cancelled),
            Blocked => matches!(next, Planning | Repairing | Failed | Cancelled),
            Complete | Failed | Cancelled => false,
        }
    }
}

/// Kind of evidence a citation points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// A test run, cited by name and result.
    Test,
    /// An entry in the hash-chained evidence ledger, cited by entry hash/id.
    Ledger,
    /// A file whose bytes were hashed.
    File,
    /// A command and its observed exit status/output.
    Command,
    /// An Artifact Bus artifact.
    Artifact,
    /// A human statement. Allowed, but visibly not machine-checked.
    Human,
}

/// One proof attached to an acceptance criterion.
///
/// A citation is a *pointer*. For `File` evidence the pointer is made
/// re-checkable: `content_sha256` is the digest of the bytes at citation time,
/// so `verify_citations` can say whether the cited file still says what the
/// mission claimed it said. Without that, "cited proof" only ever meant "a
/// string was typed here".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Citation {
    pub evidence_id: String,
    pub kind: EvidenceKind,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub summary: String,
    /// SHA-256 of the cited bytes, recorded when the citation was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_sha256: Option<String>,
    pub cited_at: DateTime<Utc>,
}

/// What re-checking the citations of a mission found.
///
/// `unverifiable` is not a failure: a test name or a human statement has no
/// bytes to hash. It is reported so an operator can tell "checked and matched"
/// apart from "nothing to check" instead of reading both as green.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CitationAudit {
    /// Citations with a recorded digest that were re-read from disk.
    pub verified: Vec<String>,
    /// `(evidence_id, expected, actual)` — the bytes changed after citation.
    pub mismatched: Vec<(String, String, String)>,
    /// File citations whose path no longer resolves.
    pub missing: Vec<String>,
    /// Citations that carry no digest (test, ledger, command, human...).
    pub unverifiable: Vec<String>,
}

impl CitationAudit {
    /// True when every digest-bearing citation still matches and no cited file
    /// has vanished.
    pub fn is_clean(&self) -> bool {
        self.mismatched.is_empty() && self.missing.is_empty()
    }

    pub fn checked(&self) -> usize {
        self.verified.len() + self.mismatched.len() + self.missing.len()
    }
}

/// A condition the mission must satisfy, with the proofs that it does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Criterion {
    pub id: String,
    pub statement: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<Citation>,
}

impl Criterion {
    /// A criterion is satisfied only by a non-empty, non-blank evidence id.
    pub fn is_satisfied(&self) -> bool {
        !self.citations.is_empty()
    }
}

/// One unit of execution. Steps are completed one at a time and each
/// completion is persisted, which is what makes `EXECUTING` resumable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub done: bool,
}

/// The whole persisted mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub schema_version: u32,
    pub id: String,
    pub objective: String,
    pub criteria: Vec<Criterion>,
    pub steps: Vec<Step>,
    pub phase: Phase,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Journal sequence number of this state. Starts at 1 (creation).
    pub seq: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}

impl Mission {
    /// Criteria that still lack a cited proof, in declaration order.
    pub fn unsatisfied(&self) -> Vec<&Criterion> {
        self.criteria.iter().filter(|c| !c.is_satisfied()).collect()
    }

    /// Steps not yet completed.
    pub fn remaining_steps(&self) -> Vec<&Step> {
        self.steps.iter().filter(|s| !s.done).collect()
    }

    /// True only when every criterion is cited. Phase is not consulted: a
    /// mission that is `COMPLETE` but has an uncited criterion is corrupt, and
    /// callers should see that rather than a comfortable boolean.
    pub fn evidence_complete(&self) -> bool {
        self.criteria.iter().all(Criterion::is_satisfied)
    }
}

/// One journal record: the full state after a change.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JournalEntry {
    seq: u64,
    at: DateTime<Utc>,
    /// Human-readable reason for the change ("plan ready", "step s2 done").
    reason: String,
    mission: Mission,
}

/// What happened when the engine was asked to move forward.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Progress {
    /// Parked at the human approval gate.
    AwaitingApproval,
    /// Executing, with steps left (either the budget ran out or a step is
    /// waiting); state is persisted and a later `resume` continues here.
    Executing { done: usize, remaining: usize },
    /// Executing finished; verification is now waiting on citations.
    AwaitingEvidence { unsatisfied: Vec<String> },
    /// Every criterion is cited and the mission is `COMPLETE`.
    Complete { mission_id: String },
    /// The mission was already terminal; nothing was changed.
    Terminal { phase: Phase },
}

/// Result of loading mission state from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadReport {
    /// The journal's trailing partial line was discarded (crash mid-write).
    pub discarded_partial_tail: bool,
    /// The derived snapshot was stale and has been rewritten.
    pub repaired_snapshot: bool,
    /// Number of journal records applied.
    pub entries: usize,
}

/// Durable mission engine rooted at a work directory.
#[derive(Debug, Clone)]
pub struct MissionEngine {
    workdir: PathBuf,
}

impl MissionEngine {
    pub fn new(workdir: impl Into<PathBuf>) -> Self {
        Self {
            workdir: workdir.into(),
        }
    }

    fn state_dir(&self) -> PathBuf {
        self.workdir.join(".zylcode")
    }

    /// Append-only journal; the authoritative record.
    pub fn journal_path(&self) -> PathBuf {
        self.state_dir().join("mission.journal.jsonl")
    }

    /// Derived snapshot, written for humans and external tools.
    pub fn snapshot_path(&self) -> PathBuf {
        self.state_dir().join("mission.json")
    }

    /// Create a mission in `CREATED` and journal it.
    ///
    /// Rejects an empty objective, a mission with no acceptance criteria
    /// (nothing could ever make it complete), and blank criterion text.
    pub fn create(
        &self,
        objective: &str,
        criteria: &[String],
        steps: &[String],
    ) -> Result<Mission> {
        if objective.trim().is_empty() {
            bail!("mission objective must not be empty");
        }
        if criteria.is_empty() {
            bail!(
                "a mission needs at least one acceptance criterion; \
                 without one it could never reach COMPLETE"
            );
        }
        if self.journal_path().exists() {
            bail!(
                "mission state already exists at {}; use `resume` or choose a new workdir",
                self.state_dir().display()
            );
        }

        let mut seen = Vec::new();
        let mut crits = Vec::new();
        for (i, text) in criteria.iter().enumerate() {
            let text = text.trim();
            if text.is_empty() {
                bail!("acceptance criterion {} is empty", i + 1);
            }
            if seen.contains(&text) {
                bail!("acceptance criterion '{text}' is declared twice");
            }
            seen.push(text);
            crits.push(Criterion {
                id: format!("c{}", i + 1),
                statement: text.to_string(),
                citations: Vec::new(),
            });
        }

        let now = Utc::now();
        let mission = Mission {
            schema_version: MISSION_ENGINE_SCHEMA_VERSION,
            id: Uuid::new_v4().to_string(),
            objective: objective.trim().to_string(),
            criteria: crits,
            steps: steps
                .iter()
                .enumerate()
                .map(|(i, d)| Step {
                    id: format!("s{}", i + 1),
                    description: d.trim().to_string(),
                    done: false,
                })
                .collect(),
            phase: Phase::Created,
            created_at: now,
            updated_at: now,
            seq: 1,
            blocked_reason: None,
            failure_reason: None,
        };
        self.append(&mission, "mission created")?;
        Ok(mission)
    }

    /// Read the authoritative state, verifying journal integrity.
    pub fn load(&self) -> Result<(Mission, LoadReport)> {
        let path = self.journal_path();
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("no mission state at {}; create one first", path.display()))?;

        let lines: Vec<&str> = text.lines().collect();
        let mut missions: Vec<Mission> = Vec::new();
        let mut discarded_partial_tail = false;
        let mut expected_seq = 1u64;

        for (idx, line) in lines.iter().enumerate() {
            let is_last = idx + 1 == lines.len();
            if line.trim().is_empty() {
                if is_last {
                    continue;
                }
                bail!("journal line {} is blank but not last", idx + 1);
            }
            let entry: JournalEntry = match serde_json::from_str(line) {
                Ok(e) => e,
                Err(e) if is_last => {
                    // A crash during append can only damage the final line.
                    discarded_partial_tail = true;
                    let _ = e;
                    continue;
                }
                Err(e) => bail!("journal line {} is corrupt: {e}", idx + 1),
            };
            if entry.seq != expected_seq {
                bail!(
                    "journal sequence gap: expected {}, found {}",
                    expected_seq,
                    entry.seq
                );
            }
            if entry.mission.seq != entry.seq {
                bail!(
                    "journal entry {} disagrees with the state it carries (seq {})",
                    idx + 1,
                    entry.mission.seq
                );
            }
            if let Some(prev) = missions.last() {
                let prev: &Mission = prev;
                if prev.phase != entry.mission.phase
                    && !prev.phase.can_transition_to(entry.mission.phase)
                {
                    bail!(
                        "journal contains an illegal transition {} -> {} at entry {}",
                        phase_name(prev.phase),
                        phase_name(entry.mission.phase),
                        entry.seq
                    );
                }
            } else if entry.mission.phase != Phase::Created {
                bail!(
                    "journal does not begin at CREATED (begins at {})",
                    phase_name(entry.mission.phase)
                );
            }
            expected_seq += 1;
            missions.push(entry.mission);
        }

        let Some(mission) = missions.last().cloned() else {
            bail!("journal at {} contains no mission state", path.display());
        };
        if discarded_partial_tail && missions.is_empty() {
            bail!("journal is unusable: only a partial record exists");
        }

        let repaired_snapshot = self.write_snapshot_if_stale(&mission)?;
        Ok((
            mission,
            LoadReport {
                discarded_partial_tail,
                repaired_snapshot,
                entries: missions.len(),
            },
        ))
    }

    /// Objective, criteria and state as one string for `status`.
    pub fn describe(&self) -> Result<String> {
        let (m, report) = self.load()?;
        let mut out = String::new();
        out.push_str(&format!("mission {} — {}\n", m.id, m.objective));
        out.push_str(&format!("phase: {}\n", phase_name(m.phase)));
        out.push_str(&format!(
            "journal: {} record(s){}{}\n",
            report.entries,
            if report.discarded_partial_tail {
                " (discarded a partial trailing record after an interrupted write)"
            } else {
                ""
            },
            if report.repaired_snapshot {
                " (repaired a stale snapshot)"
            } else {
                ""
            }
        ));
        if let Some(reason) = &m.blocked_reason {
            out.push_str(&format!("blocked: {reason}\n"));
        }
        if let Some(reason) = &m.failure_reason {
            out.push_str(&format!("failed: {reason}\n"));
        }
        out.push_str("acceptance criteria:\n");
        for c in &m.criteria {
            let mark = if c.is_satisfied() {
                "cited"
            } else {
                "UNSATISFIED"
            };
            out.push_str(&format!("  {} [{}] {}\n", c.id, mark, c.statement));
            for cite in &c.citations {
                let digest = match &cite.content_sha256 {
                    Some(d) => format!(" sha256:{}", &d[..d.len().min(12)]),
                    None => String::new(),
                };
                out.push_str(&format!(
                    "      <- {:?} {}{} {}\n",
                    cite.kind, cite.evidence_id, digest, cite.summary
                ));
            }
        }
        let done = m.steps.iter().filter(|s| s.done).count();
        out.push_str(&format!("steps: {done}/{} complete\n", m.steps.len()));
        Ok(out)
    }

    /// Resolve a citation's evidence id to a path on disk, if it is one.
    ///
    /// Absolute ids are used as given; relative ids are resolved against the
    /// mission's working directory, which is what `--evidence src/lib.rs`
    /// means to an operator running the CLI from the project root.
    pub fn resolve_evidence_path(&self, evidence_id: &str) -> Option<PathBuf> {
        let candidate = Path::new(evidence_id);
        if candidate.is_absolute() {
            return candidate.is_file().then(|| candidate.to_path_buf());
        }
        let under_workdir = self.workdir.join(candidate);
        if under_workdir.is_file() {
            return Some(under_workdir);
        }
        candidate.is_file().then(|| candidate.to_path_buf())
    }

    /// Re-read every digest-bearing citation and report what still matches.
    ///
    /// This is the difference between "the mission cites a proof" and "the
    /// proof can be checked": a citation written by a previous run is only
    /// trustworthy if the bytes it named still hash to what was recorded.
    pub fn verify_citations(&self) -> Result<CitationAudit> {
        let (mission, _) = self.load()?;
        let mut audit = CitationAudit::default();
        for criterion in &mission.criteria {
            for cite in &criterion.citations {
                let Some(expected) = cite.content_sha256.as_deref() else {
                    audit.unverifiable.push(cite.evidence_id.clone());
                    continue;
                };
                let Some(path) = self.resolve_evidence_path(&cite.evidence_id) else {
                    audit.missing.push(cite.evidence_id.clone());
                    continue;
                };
                match file_digest(&path) {
                    Ok(actual) if actual == expected => {
                        audit.verified.push(cite.evidence_id.clone())
                    }
                    Ok(actual) => audit.mismatched.push((
                        cite.evidence_id.clone(),
                        expected.to_string(),
                        actual,
                    )),
                    Err(_) => audit.missing.push(cite.evidence_id.clone()),
                }
            }
        }
        Ok(audit)
    }

    /// Move the mission forward as far as the rules and evidence allow.
    ///
    /// `auto_approve` passes the human approval gate; `step_budget` stops after
    /// that many step completions, which is how an interrupted execution is
    /// modelled honestly: every completion is already durable, so the caller
    /// can stop at any point and resume later.
    pub fn resume(&self, auto_approve: bool, step_budget: Option<usize>) -> Result<Progress> {
        let mut budget = step_budget;
        loop {
            let (mut mission, _) = self.load()?;
            match mission.phase {
                // Every branch ends by reloading, so the value a transition
                // returns is deliberately dropped: the journal is the truth.
                Phase::Created => {
                    self.transition(mission, Phase::Understanding, "mission admitted")?;
                }
                Phase::Understanding => {
                    let n = mission.criteria.len();
                    self.transition(
                        mission,
                        Phase::Planning,
                        &format!("objective understood; {n} acceptance criteria"),
                    )?;
                }
                Phase::Planning => {
                    let n = mission.steps.len();
                    self.transition(
                        mission,
                        Phase::AwaitingApproval,
                        &format!("plan ready: {n} step(s)"),
                    )?;
                }
                Phase::AwaitingApproval => {
                    if !auto_approve {
                        return Ok(Progress::AwaitingApproval);
                    }
                    self.transition(mission, Phase::Executing, "plan approved by operator")?;
                }
                Phase::Repairing => {
                    self.transition(mission, Phase::Executing, "repair accepted; re-executing")?;
                }
                Phase::Blocked => {
                    bail!(
                        "mission is BLOCKED ({}); unblock it explicitly before resuming",
                        mission
                            .blocked_reason
                            .as_deref()
                            .unwrap_or("no reason recorded")
                    )
                }
                Phase::Executing => {
                    let remaining = mission.remaining_steps().len();
                    if remaining == 0 {
                        let total = mission.steps.len();
                        self.transition(
                            mission,
                            Phase::Verifying,
                            &format!("all {total} step(s) executed"),
                        )?;
                        continue;
                    }
                    let take = match budget {
                        Some(0) => {
                            return Ok(Progress::Executing {
                                done: mission.steps.iter().filter(|s| s.done).count(),
                                remaining,
                            })
                        }
                        Some(b) => b.min(remaining),
                        None => remaining,
                    };
                    for _ in 0..take {
                        let next = mission
                            .steps
                            .iter()
                            .position(|s| !s.done)
                            .expect("remaining steps checked above");
                        mission.steps[next].done = true;
                        let reason = format!("step {} done", mission.steps[next].id);
                        mission = self.transition(mission, Phase::Executing, &reason)?;
                    }
                    if let Some(b) = budget.as_mut() {
                        *b -= take;
                    }
                    continue;
                }
                Phase::Verifying => {
                    let unsatisfied = mission.unsatisfied();
                    if unsatisfied.is_empty() {
                        let cited = mission
                            .criteria
                            .iter()
                            .map(|c| c.citations.len())
                            .sum::<usize>();
                        let criteria = mission.criteria.len();
                        let mission_id = mission.id.clone();
                        self.transition(
                            mission,
                            Phase::Complete,
                            &format!("{criteria} criteria satisfied by {cited} cited proof(s)"),
                        )?;
                        return Ok(Progress::Complete { mission_id });
                    }
                    return Ok(Progress::AwaitingEvidence {
                        unsatisfied: unsatisfied.iter().map(|c| c.id.clone()).collect(),
                    });
                }
                phase if phase.is_terminal() => return Ok(Progress::Terminal { phase }),
                other => bail!("mission is in unhandled phase {}", phase_name(other)),
            }
        }
    }

    /// Attach a proof to a criterion. Only legal while `VERIFYING`, so evidence
    /// cannot be pre-baked before the work that produces it.
    pub fn cite(
        &self,
        criterion_id: &str,
        evidence_id: &str,
        kind: EvidenceKind,
        summary: &str,
    ) -> Result<Mission> {
        self.cite_with_digest(criterion_id, evidence_id, kind, summary, None)
    }

    /// Cite evidence, recording a digest so the claim can be re-checked.
    ///
    /// For `EvidenceKind::File` the digest is taken from the cited bytes unless
    /// the caller supplies one (the CLI does not have to, a producer of the
    /// evidence can). A path that does not resolve is cited without a digest
    /// rather than refused: the operator may be citing a file that a later step
    /// creates, and `verify_citations` will report it as missing instead of the
    /// mission silently pretending it was checked.
    pub fn cite_with_digest(
        &self,
        criterion_id: &str,
        evidence_id: &str,
        kind: EvidenceKind,
        summary: &str,
        digest: Option<String>,
    ) -> Result<Mission> {
        if evidence_id.trim().is_empty() {
            bail!("evidence id must not be empty; an uncited criterion cannot complete a mission");
        }
        let (mut mission, _) = self.load()?;
        if mission.phase != Phase::Verifying {
            bail!(
                "evidence can only be cited while VERIFYING; mission is {}",
                phase_name(mission.phase)
            );
        }
        let criterion = mission
            .criteria
            .iter_mut()
            .find(|c| c.id == criterion_id)
            .with_context(|| format!("unknown criterion '{criterion_id}'"))?;
        if criterion
            .citations
            .iter()
            .any(|c| c.evidence_id == evidence_id)
        {
            return Ok(mission);
        }
        let evidence_id = evidence_id.trim().to_string();
        let content_sha256 = match (digest, kind) {
            (Some(d), _) if !d.trim().is_empty() => Some(d.trim().to_string()),
            (None, EvidenceKind::File) => self
                .resolve_evidence_path(&evidence_id)
                .and_then(|p| file_digest(&p).ok()),
            _ => None,
        };
        criterion.citations.push(Citation {
            evidence_id: evidence_id.clone(),
            kind,
            summary: summary.trim().to_string(),
            content_sha256,
            cited_at: Utc::now(),
        });
        self.append(
            &mission,
            &format!("criterion {criterion_id} cited evidence {evidence_id}"),
        )
    }

    /// Stop the mission. A terminal mission cannot be cancelled, and
    /// `COMPLETE` in particular is never undone.
    pub fn cancel(&self, reason: &str) -> Result<Mission> {
        let (mission, _) = self.load()?;
        if mission.phase.is_terminal() {
            bail!(
                "mission is already {}; it cannot be cancelled",
                phase_name(mission.phase)
            );
        }
        let reason = reason.trim();
        if reason.is_empty() {
            bail!("cancelling a mission requires a reason");
        }
        self.transition(mission, Phase::Cancelled, reason)
    }

    /// Assert a block with its reason. A block is a first-class state, not a
    /// failure: the mission is neither dead nor quietly done.
    pub fn block(&self, reason: &str) -> Result<Mission> {
        let (mut mission, _) = self.load()?;
        if mission.phase.is_terminal() {
            bail!("cannot block a {} mission", phase_name(mission.phase));
        }
        let reason = reason.trim();
        if reason.is_empty() {
            bail!("blocking a mission requires a reason");
        }
        mission.blocked_reason = Some(reason.to_string());
        self.append_transition(mission, Phase::Blocked, reason)
    }

    /// Move from `VERIFYING` back to `REPAIRING` because a criterion is unmet
    /// in a way that needs more work rather than more evidence.
    pub fn request_repair(&self, reason: &str) -> Result<Mission> {
        let (mission, _) = self.load()?;
        let reason = reason.trim();
        if reason.is_empty() {
            bail!("requesting a repair requires a reason");
        }
        self.transition(mission, Phase::Repairing, reason)
    }

    // -- internals ---------------------------------------------------------

    fn transition(&self, mission: Mission, to: Phase, reason: &str) -> Result<Mission> {
        self.append_transition(mission, to, reason)
    }

    fn append_transition(&self, mut mission: Mission, to: Phase, reason: &str) -> Result<Mission> {
        // A same-phase record is progress inside a phase (a completed step),
        // not a transition; it still has to be journaled.
        if to != mission.phase && !mission.phase.can_transition_to(to) {
            bail!(
                "illegal mission transition {} -> {}",
                phase_name(mission.phase),
                phase_name(to)
            );
        }
        if to == Phase::Complete && !mission.evidence_complete() {
            let names: Vec<String> = mission
                .unsatisfied()
                .iter()
                .map(|c| format!("{} ({})", c.id, c.statement))
                .collect();
            bail!(
                "cannot COMPLETE: {} criterion/criteria lack a cited proof: {}",
                names.len(),
                names.join("; ")
            );
        }
        mission.phase = to;
        if to != Phase::Blocked {
            mission.blocked_reason = None;
        }
        self.append(&mission, reason)
    }

    /// Append one durable state record, then refresh the derived snapshot.
    ///
    /// Journal first, snapshot second: a crash between the two leaves a stale
    /// snapshot, which is repaired on the next load. The reverse order would
    /// allow a snapshot that claims work the journal never recorded.
    fn append(&self, mission: &Mission, reason: &str) -> Result<Mission> {
        let seq = match self.load() {
            Ok((prev, report)) => {
                // A crash mid-append left a partial final record. Drop it before
                // appending, otherwise the new record would follow a corrupt
                // line and every later load would have to fail closed.
                if report.discarded_partial_tail {
                    self.drop_partial_tail()?;
                }
                prev.seq + 1
            }
            // Creation is the one write that legitimately happens first.
            Err(_) if mission.seq == 1 => 1,
            Err(e) => return Err(e),
        };

        let mut m = mission.clone();
        m.seq = seq;
        m.updated_at = Utc::now();
        m.schema_version = MISSION_ENGINE_SCHEMA_VERSION;

        std::fs::create_dir_all(self.state_dir())
            .with_context(|| format!("failed to create {}", self.state_dir().display()))?;
        let entry = JournalEntry {
            seq,
            at: m.updated_at,
            reason: reason.to_string(),
            mission: m.clone(),
        };
        let line = serde_json::to_string(&entry)?;
        {
            let mut f: File = OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.journal_path())
                .with_context(|| format!("failed to open {}", self.journal_path().display()))?;
            f.write_all(line.as_bytes())?;
            f.write_all(b"\n")?;
            f.sync_all()?;
        }
        self.write_snapshot(&m, true)?;
        Ok(m)
    }

    /// Remove a final line that has no terminating newline (a torn append).
    fn drop_partial_tail(&self) -> Result<()> {
        let path = self.journal_path();
        let text = std::fs::read_to_string(&path)?;
        if text.ends_with('\n') {
            return Ok(());
        }
        let keep = text.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let f = OpenOptions::new().write(true).open(&path)?;
        f.set_len(keep as u64)?;
        f.sync_all()?;
        Ok(())
    }

    fn write_snapshot(&self, mission: &Mission, force: bool) -> Result<bool> {
        let path = self.snapshot_path();
        if !force {
            if let Ok(existing) = std::fs::read_to_string(&path) {
                if let Ok(prev) = serde_json::from_str::<Mission>(&existing) {
                    if prev.seq >= mission.seq {
                        return Ok(false);
                    }
                }
            }
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(mission)?)?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(true)
    }

    fn write_snapshot_if_stale(&self, mission: &Mission) -> Result<bool> {
        self.write_snapshot(mission, false)
    }
}

/// Stable uppercase name, as the roadmap spells the states.
pub fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Created => "CREATED",
        Phase::Understanding => "UNDERSTANDING",
        Phase::Planning => "PLANNING",
        Phase::AwaitingApproval => "AWAITING_APPROVAL",
        Phase::Executing => "EXECUTING",
        Phase::Verifying => "VERIFYING",
        Phase::Repairing => "REPAIRING",
        Phase::Blocked => "BLOCKED",
        Phase::Complete => "COMPLETE",
        Phase::Failed => "FAILED",
        Phase::Cancelled => "CANCELLED",
    }
}

/// Path helper for callers that only need to know where state lives.
pub fn mission_state_dir(workdir: &Path) -> PathBuf {
    workdir.join(".zylcode")
}

/// SHA-256 of a file's bytes, lowercase hex — the same shape as the project
/// filesystem's content hashes, so a citation and an editor share a vocabulary.
fn file_digest(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> (tempfile::TempDir, MissionEngine) {
        let dir = tempfile::tempdir().unwrap();
        let e = MissionEngine::new(dir.path());
        (dir, e)
    }

    fn create(e: &MissionEngine) -> Mission {
        e.create(
            "ship the engine",
            &["tests pass".to_string(), "docs updated".to_string()],
            &["write code".to_string(), "run tests".to_string()],
        )
        .unwrap()
    }

    /// Drive to VERIFYING with every step executed.
    fn executing_done(e: &MissionEngine) {
        let p = e.resume(true, None).unwrap();
        // No evidence yet, so it parks in verification.
        assert!(matches!(p, Progress::AwaitingEvidence { .. }), "{p:?}");
    }

    #[test]
    fn a_file_citation_records_a_digest_that_verifies() {
        let (dir, e) = engine();
        create(&e);
        std::fs::write(dir.path().join("proof.txt"), b"the tests passed").unwrap();

        executing_done(&e);
        let mission = e
            .cite("c1", "proof.txt", EvidenceKind::File, "observed output")
            .unwrap();
        let digest = mission.criteria[0].citations[0]
            .content_sha256
            .clone()
            .expect("file citation must carry a digest");
        assert_eq!(digest.len(), 64);

        let audit = e.verify_citations().unwrap();
        assert!(audit.is_clean(), "{audit:?}");
        assert_eq!(audit.verified, vec!["proof.txt".to_string()]);
        assert_eq!(audit.checked(), 1);
    }

    #[test]
    fn a_changed_file_is_reported_as_a_mismatch() {
        let (dir, e) = engine();
        create(&e);
        let proof = dir.path().join("proof.txt");
        std::fs::write(&proof, b"before").unwrap();

        executing_done(&e);
        e.cite("c1", "proof.txt", EvidenceKind::File, "checked")
            .unwrap();
        std::fs::write(&proof, b"after the claim").unwrap();

        let audit = e.verify_citations().unwrap();
        assert!(!audit.is_clean());
        assert_eq!(audit.mismatched.len(), 1);
        assert_eq!(audit.mismatched[0].0, "proof.txt");
        assert_ne!(audit.mismatched[0].1, audit.mismatched[0].2);
    }

    #[test]
    fn a_deleted_cited_file_is_reported_as_missing_not_as_verified() {
        let (dir, e) = engine();
        create(&e);
        let proof = dir.path().join("proof.txt");
        std::fs::write(&proof, b"here for now").unwrap();

        executing_done(&e);
        e.cite("c1", "proof.txt", EvidenceKind::File, "checked")
            .unwrap();
        std::fs::remove_file(&proof).unwrap();

        let audit = e.verify_citations().unwrap();
        assert!(!audit.is_clean());
        assert_eq!(audit.missing, vec!["proof.txt".to_string()]);
        assert!(audit.verified.is_empty());
    }

    #[test]
    fn non_file_citations_are_visible_as_unverifiable() {
        let (_dir, e) = engine();
        create(&e);
        executing_done(&e);
        e.cite(
            "c1",
            "cargo test --workspace",
            EvidenceKind::Command,
            "exit 0",
        )
        .unwrap();

        let audit = e.verify_citations().unwrap();
        assert!(
            audit.is_clean(),
            "nothing to check is not a failure: {audit:?}"
        );
        assert_eq!(audit.verified.len(), 0);
        assert_eq!(audit.unverifiable.len(), 1);
    }

    #[test]
    fn create_requires_an_objective_and_criteria() {
        let (_d, e) = engine();
        assert!(e.create("  ", &["x".into()], &[]).is_err());
        let err = e
            .create("do work", &[], &["step".into()])
            .unwrap_err()
            .to_string();
        assert!(err.contains("acceptance criterion"), "{err}");
        assert!(e
            .create("do work", &["  ".into()], &["step".into()])
            .is_err());
        assert!(e
            .create("do work", &["same".into(), "same".into()], &[])
            .is_err());
    }

    #[test]
    fn creation_is_journaled_and_survives_a_new_instance() {
        let (_d, e) = engine();
        let m = create(&e);
        assert_eq!(m.phase, Phase::Created);
        let reopened = MissionEngine::new(e.workdir.clone());
        let (loaded, report) = reopened.load().unwrap();
        assert_eq!(loaded.id, m.id);
        assert_eq!(loaded.seq, 1);
        assert_eq!(report.entries, 1);
        assert!(loaded.criteria.iter().all(|c| !c.is_satisfied()));
    }

    #[test]
    fn resume_stops_at_the_approval_gate() {
        let (_d, e) = engine();
        let _ = create(&e);
        assert_eq!(e.resume(false, None).unwrap(), Progress::AwaitingApproval);
        let (m, _) = e.load().unwrap();
        assert_eq!(m.phase, Phase::AwaitingApproval);
        assert!(
            m.steps.iter().all(|s| !s.done),
            "nothing ran before approval"
        );
    }

    #[test]
    fn complete_requires_a_citation_for_every_criterion() {
        let (_d, e) = engine();
        let _ = create(&e);
        executing_done(&e);

        // One citation is not enough.
        e.cite("c1", "test:mission_engine", EvidenceKind::Test, "12 passed")
            .unwrap();
        match e.resume(false, None).unwrap() {
            Progress::AwaitingEvidence { unsatisfied } => assert_eq!(unsatisfied, vec!["c2"]),
            other => panic!("expected AwaitingEvidence, got {other:?}"),
        }
        let (m, _) = e.load().unwrap();
        assert_eq!(m.phase, Phase::Verifying, "must not be COMPLETE");

        // A direct attempt to force COMPLETE fails closed too.
        let err = e
            .append_transition(m.clone(), Phase::Complete, "wishful")
            .unwrap_err()
            .to_string();
        assert!(err.contains("cannot COMPLETE"), "{err}");
        assert!(err.contains("c2"), "{err}");

        e.cite("c2", "docs:PROJECT_SYSTEM", EvidenceKind::File, "§13")
            .unwrap();
        assert_eq!(
            e.resume(false, None).unwrap(),
            Progress::Complete {
                mission_id: m.id.clone()
            }
        );
        let (done, _) = e.load().unwrap();
        assert_eq!(done.phase, Phase::Complete);
        assert!(done.evidence_complete());
    }

    #[test]
    fn evidence_cannot_be_cited_before_verification() {
        let (_d, e) = engine();
        let _ = create(&e);
        let err = e
            .cite("c1", "test:x", EvidenceKind::Test, "early")
            .unwrap_err()
            .to_string();
        assert!(err.contains("VERIFYING"), "{err}");
    }

    #[test]
    fn unknown_criterion_and_empty_evidence_are_rejected() {
        let (_d, e) = engine();
        let _ = create(&e);
        executing_done(&e);
        assert!(e.cite("c9", "test:x", EvidenceKind::Test, "").is_err());
        assert!(e.cite("c1", "   ", EvidenceKind::Test, "").is_err());
    }

    #[test]
    fn step_budget_pauses_execution_and_resume_continues() {
        let (_d, e) = engine();
        let _ = create(&e);
        // Admitting, planning, approving, then exactly one step.
        match e.resume(true, Some(1)).unwrap() {
            Progress::Executing { done, remaining } => {
                assert_eq!((done, remaining), (1, 1));
            }
            other => panic!("expected partial execution, got {other:?}"),
        }

        // A brand-new engine instance reads the persisted cursor.
        let reopened = MissionEngine::new(e.workdir.clone());
        let (m, _) = reopened.load().unwrap();
        assert_eq!(m.phase, Phase::Executing);
        assert_eq!(m.steps.iter().filter(|s| s.done).count(), 1);

        match reopened.resume(true, None).unwrap() {
            Progress::AwaitingEvidence { unsatisfied } => assert_eq!(unsatisfied.len(), 2),
            other => panic!("expected verification, got {other:?}"),
        }
        let (m, _) = reopened.load().unwrap();
        assert!(m.steps.iter().all(|s| s.done));
    }

    #[test]
    fn a_truncated_final_journal_record_is_discarded() {
        let (_d, e) = engine();
        let _ = create(&e);
        let _ = e.resume(false, None).unwrap();
        let good = std::fs::read_to_string(e.journal_path()).unwrap();

        // Simulate a crash mid-append: half a record, no newline.
        let mut broken = good.clone();
        broken.push_str("{\"seq\":3,\"at\":\"2026-11-15T00:00:00Z\",\"reason\":\"pl");
        std::fs::write(e.journal_path(), broken).unwrap();

        let (m, report) = e.load().unwrap();
        assert!(report.discarded_partial_tail);
        assert_eq!(m.phase, Phase::AwaitingApproval);
        // created, admitted, planned, awaiting approval — the partial fifth is gone.
        assert_eq!(report.entries, 4);

        // And the engine can still move forward, appending after recovery.
        assert_eq!(
            e.resume(true, None).unwrap(),
            Progress::AwaitingEvidence {
                unsatisfied: vec!["c1".into(), "c2".into()]
            }
        );
    }

    #[test]
    fn interior_corruption_fails_closed() {
        let (_d, e) = engine();
        let _ = create(&e);
        let _ = e.resume(false, None).unwrap();
        let good = std::fs::read_to_string(e.journal_path()).unwrap();
        let mut lines: Vec<String> = good.lines().map(str::to_string).collect();
        lines.insert(1, "{\"garbage\":true}".to_string());
        std::fs::write(e.journal_path(), lines.join("\n") + "\n").unwrap();
        let err = e.load().unwrap_err().to_string();
        assert!(err.contains("corrupt"), "{err}");
    }

    #[test]
    fn illegal_and_missing_transitions_are_refused() {
        let (_d, e) = engine();
        let _ = create(&e);
        let (m, _) = e.load().unwrap();
        // CREATED cannot jump straight to COMPLETE.
        let err = e
            .append_transition(m, Phase::Complete, "shortcut")
            .unwrap_err()
            .to_string();
        assert!(err.contains("illegal"), "{err}");

        executing_done(&e);
        e.cite("c1", "test:a", EvidenceKind::Test, "").unwrap();
        e.cite("c2", "test:b", EvidenceKind::Test, "").unwrap();
        let _ = e.resume(false, None).unwrap();
        let (done, _) = e.load().unwrap();
        assert_eq!(done.phase, Phase::Complete);
        assert_eq!(
            e.resume(false, None).unwrap(),
            Progress::Terminal {
                phase: Phase::Complete
            }
        );
        assert!(e.cancel("too late").is_err(), "COMPLETE is final");
        assert!(!Phase::Complete.can_transition_to(Phase::Executing));
    }

    #[test]
    fn cancel_requires_a_reason_and_is_final() {
        let (_d, e) = engine();
        let _ = create(&e);
        assert!(e.cancel(" ").is_err());
        let m = e.cancel("owner withdrew the objective").unwrap();
        assert_eq!(m.phase, Phase::Cancelled);
        assert_eq!(
            e.resume(false, None).unwrap(),
            Progress::Terminal {
                phase: Phase::Cancelled
            }
        );
        assert!(e.block("nope").is_err());
    }

    #[test]
    fn block_is_distinct_from_failure_and_can_resume() {
        let (_d, e) = engine();
        let _ = create(&e);
        let _ = e.resume(false, None).unwrap();
        let blocked = e.block("approval service unreachable").unwrap();
        assert_eq!(blocked.phase, Phase::Blocked);
        assert_eq!(
            blocked.blocked_reason.as_deref(),
            Some("approval service unreachable")
        );
        // Resume refuses to silently unblock.
        let err = e.resume(true, None).unwrap_err().to_string();
        assert!(err.contains("BLOCKED"), "{err}");

        // An explicit repair request re-enters planning, clearing the block.
        let repaired = e.request_repair("approval service restored").unwrap();
        assert_eq!(repaired.phase, Phase::Repairing);
        assert!(repaired.blocked_reason.is_none());
        assert!(matches!(
            e.resume(true, None).unwrap(),
            Progress::AwaitingEvidence { .. }
        ));
    }

    #[test]
    fn duplicate_citation_is_idempotent() {
        let (_d, e) = engine();
        let _ = create(&e);
        executing_done(&e);
        e.cite("c1", "test:same", EvidenceKind::Test, "pass")
            .unwrap();
        let before = e.load().unwrap().0.seq;
        e.cite("c1", "test:same", EvidenceKind::Test, "pass")
            .unwrap();
        let after = e.load().unwrap().0.seq;
        assert_eq!(before, after, "an idempotent cite must not journal");
    }

    #[test]
    fn stale_snapshot_is_repaired_without_losing_state() {
        let (_d, e) = engine();
        let _ = create(&e);
        let _ = e.resume(false, None).unwrap();
        // Remove the derived snapshot; the journal is authoritative.
        std::fs::remove_file(e.snapshot_path()).unwrap();
        let (m, report) = e.load().unwrap();
        assert_eq!(m.phase, Phase::AwaitingApproval);
        assert!(report.repaired_snapshot);
        assert!(e.snapshot_path().exists());
    }

    #[test]
    fn create_refuses_to_overwrite_existing_state() {
        let (_d, e) = engine();
        let _ = create(&e);
        let err = e
            .create("another", &["c".into()], &[])
            .unwrap_err()
            .to_string();
        assert!(err.contains("already exists"), "{err}");
    }

    #[test]
    fn phase_names_match_the_roadmap_spelling() {
        let names: Vec<&str> = [
            Phase::Created,
            Phase::Understanding,
            Phase::Planning,
            Phase::AwaitingApproval,
            Phase::Executing,
            Phase::Verifying,
            Phase::Repairing,
            Phase::Blocked,
            Phase::Complete,
            Phase::Failed,
            Phase::Cancelled,
        ]
        .iter()
        .map(|p| phase_name(*p))
        .collect();
        assert_eq!(
            names,
            vec![
                "CREATED",
                "UNDERSTANDING",
                "PLANNING",
                "AWAITING_APPROVAL",
                "EXECUTING",
                "VERIFYING",
                "REPAIRING",
                "BLOCKED",
                "COMPLETE",
                "FAILED",
                "CANCELLED",
            ]
        );
    }
}
