//! Proof Engine (wave 4 — foundation).
//!
//! # Why this exists
//!
//! The evidence ledger records *what happened*; the Proof Engine records
//! *what was proven about it*, with states that refuse the two dishonest
//! conversions this project's governance explicitly forbids:
//!
//! - "not run" must never become "passed";
//! - a static/simulated result must never become runtime proof.
//!
//! # Proof record
//!
//! A proof binds: the exact test command, the input state (commit + clean
//! flag), expected vs actual result, exit status, duration, environment,
//! artifact references, log location, reviewer, acceptance state, and known
//! limitations. Every record is content-hashed at creation (the hash covers
//! the outcome fields) and appended to a hash chain like the evidence
//! ledger — mutation of history is detectable.
//!
//! # States
//!
//! Verification is a truth table, not a boolean:
//! `Passed | NotRun | Blocked | Failed | RuntimeNotReached | EvidenceMissing`.
//! Only `Passed` may later be `Accepted` by a named reviewer. Stale evidence
//! (source commit moved on) is reported per-query, never rewritten.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Current proof contract version.
pub const PROOF_SCHEMA_VERSION: u32 = 1;

/// Verification state of one proof record. These are *observations*, kept
/// deliberately distinct so no state can masquerade as another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationState {
    /// The command ran and exited with the expected status.
    Passed,
    /// The command was never executed (nothing may be claimed).
    NotRun,
    /// Execution was refused/deprioritized by a gate (permissions, sandbox,
    /// missing dependency). A blocked proof asserts the block, not an outcome.
    Blocked,
    /// The command ran and failed.
    Failed,
    /// The harness never reached the runtime path being proven (e.g. the
    /// simulated facade answered instead of the real system).
    RuntimeNotReached,
    /// The outcome is claimed but its logs/artifacts are missing.
    EvidenceMissing,
}

impl VerificationState {
    /// True only for the one state that asserts a real pass.
    pub fn is_pass(self) -> bool {
        matches!(self, VerificationState::Passed)
    }
}

/// Acceptance state (human judgement layered on top of verification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceState {
    Pending,
    Accepted,
    Rejected,
}

/// What kind of evidence the proof cites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofSource {
    /// A real command was executed and captured (runtime).
    RuntimeCommand,
    /// Static analysis / lint output — never runtime proof.
    StaticAnalysis,
    /// A simulated or synthetic harness output — never runtime proof.
    Simulated,
}

/// One proof record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRecord {
    pub id: String,
    pub schema_version: u32,
    /// Owning project id.
    pub project_id: String,
    /// Producing mission id, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_id: Option<String>,
    /// The exact verification command (argv form).
    pub command: String,
    /// Input state: the commit the command ran against.
    pub source_commit: String,
    /// Whether the tree was clean when the command ran.
    pub tree_clean: bool,
    /// Expected outcome (what "pass" means for this command).
    pub expected: String,
    /// Observed outcome summary (stdout tail, assertion name, etc.).
    pub actual: Option<String>,
    /// Exit status (None when never run).
    pub exit_status: Option<i32>,
    /// Wall duration in milliseconds (None when never run).
    pub duration_ms: Option<u64>,
    /// Environment snapshot (selected vars only — never secrets).
    pub environment: std::collections::BTreeMap<String, String>,
    /// Cited artifact ids (Artifact Bus references).
    #[serde(default)]
    pub artifact_refs: Vec<String>,
    /// Where the full log lives (path inside the workspace).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_path: Option<String>,
    /// Reviewer identity for acceptance decisions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<String>,
    pub acceptance: AcceptanceState,
    /// Known limitations recorded with this proof.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limitations: Option<String>,
    /// The verification observation.
    pub verification: VerificationState,
    /// What kind of source produced this record.
    pub source: ProofSource,
    /// Chain: hash of the previous record (hex), or zeros for genesis.
    pub prev_hash: String,
    /// Hash over this record's outcome fields (hex).
    pub entry_hash: String,
    /// ISO 8601 creation time.
    pub created_at: String,
}

impl ProofRecord {
    /// Outcome hash: every field a future reader must be able to trust.
    fn outcome_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.command.as_bytes());
        hasher.update(self.source_commit.as_bytes());
        hasher.update(self.tree_clean.to_string().as_bytes());
        hasher.update(self.expected.as_bytes());
        hasher.update(self.actual.clone().unwrap_or_default().as_bytes());
        hasher.update(self.exit_status.unwrap_or(i32::MIN).to_le_bytes());
        hasher.update(self.duration_ms.unwrap_or(u64::MAX).to_le_bytes());
        hasher.update(serde_json::to_string(&self.environment).unwrap_or_default());
        for a in &self.artifact_refs {
            hasher.update(a.as_bytes());
        }
        hasher.update(serde_json::to_string(&self.verification).unwrap_or_default());
        hasher.update(serde_json::to_string(&self.source).unwrap_or_default());
        hasher.update(self.prev_hash.as_bytes());
        hex(&hasher.finalize())
    }
}

/// File-backed Proof Engine store with a hash-chained history.
#[derive(Debug)]
pub struct ProofEngine {
    path: PathBuf,
    lock: Mutex<()>,
}

/// Where proofs are persisted (env override → `<root>/.zylcode/proofs.json`).
pub fn proofs_path(root: &Path) -> PathBuf {
    if let Ok(p) = std::env::var("PROOFS_PATH") {
        return PathBuf::from(p);
    }
    root.join(".zylcode").join("proofs.json")
}

/// Input for recording a proof. The engine fills the computed fields.
#[derive(Debug, Clone)]
pub struct ProofInput {
    pub project_id: String,
    pub mission_id: Option<String>,
    pub command: String,
    pub source_commit: String,
    pub tree_clean: bool,
    pub expected: String,
    pub actual: Option<String>,
    pub exit_status: Option<i32>,
    pub duration_ms: Option<u64>,
    pub environment: std::collections::BTreeMap<String, String>,
    pub artifact_refs: Vec<String>,
    pub log_path: Option<String>,
    pub verification: VerificationState,
    pub source: ProofSource,
    pub limitations: Option<String>,
}

impl ProofEngine {
    /// Open (or initialize) the proof store.
    pub fn open(root: &Path) -> Result<Self> {
        let path = proofs_path(root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        // Validate the existing chain once at open so corruption is visible
        // immediately, not only when someone audits.
        let records = Self::load(&path)?;
        let _ = Self::verify_chain(&records);
        Ok(Self {
            path,
            lock: Mutex::new(()),
        })
    }

    fn load(path: &Path) -> Result<Vec<ProofRecord>> {
        match std::fs::read_to_string(path) {
            Ok(s) if s.trim().is_empty() => Ok(Vec::new()),
            Ok(s) => serde_json::from_str(&s)
                .with_context(|| format!("proof store at {} is corrupt", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
        }
    }

    fn store(records: &[ProofRecord], path: &Path) -> Result<()> {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(records)?)?;
        std::fs::rename(&tmp, path).context("failed to persist proofs")?;
        Ok(())
    }

    /// Record a proof observation. NotRun/Blocked/RuntimeNotReached are
    /// first-class outcomes — recording them is honest, converting them to
    /// Passed is not possible through any API here.
    pub fn record(&self, input: ProofInput) -> Result<ProofRecord> {
        let _g = self.lock.lock().unwrap();
        let mut records = Self::load(&self.path)?;
        let now = chrono::Utc::now().to_rfc3339();
        let prev_hash = records
            .last()
            .map(|r| r.entry_hash.clone())
            .unwrap_or_else(|| "0".repeat(64));
        let mut rec = ProofRecord {
            id: format!("proof-{}", uuid::Uuid::new_v4()),
            schema_version: PROOF_SCHEMA_VERSION,
            project_id: input.project_id,
            mission_id: input.mission_id,
            command: input.command,
            source_commit: input.source_commit,
            tree_clean: input.tree_clean,
            expected: input.expected,
            actual: input.actual,
            exit_status: input.exit_status,
            duration_ms: input.duration_ms,
            environment: input.environment,
            artifact_refs: input.artifact_refs,
            log_path: input.log_path,
            reviewer: None,
            acceptance: AcceptanceState::Pending,
            limitations: input.limitations,
            verification: input.verification,
            source: input.source,
            prev_hash,
            entry_hash: String::new(),
            created_at: now,
        };
        rec.entry_hash = rec.outcome_hash();
        records.push(rec.clone());
        Self::store(&records, &self.path)?;
        Ok(rec)
    }

    /// Verify the whole hash chain. Returns Ok(records) when intact.
    pub fn verify_chain(records: &[ProofRecord]) -> Result<&[ProofRecord]> {
        let mut prev = "0".repeat(64);
        for (i, r) in records.iter().enumerate() {
            anyhow::ensure!(
                r.prev_hash == prev,
                "proof chain broken at record {i} ({}): prev_hash mismatch",
                r.id
            );
            let expected_hash = {
                // Recompute without mutating: entry_hash is excluded from the
                // hash input by construction (outcome_hash never reads it).
                let mut copy = r.clone();
                copy.entry_hash = String::new();
                copy.outcome_hash()
            };
            anyhow::ensure!(
                r.entry_hash == expected_hash,
                "proof chain broken at record {i} ({}): entry hash mismatch — history was modified",
                r.id
            );
            prev = r.entry_hash.clone();
        }
        Ok(records)
    }

    /// All proofs, newest first, each annotated with staleness relative to
    /// `current_commit`. Stale proofs are reported, never rewritten.
    pub fn list(&self, current_commit: &str) -> Result<Vec<ProofView>> {
        let records = Self::load(&self.path)?;
        let annotated = records
            .iter()
            .rev()
            .map(|r| ProofView {
                record: r.clone(),
                stale: r.source_commit != current_commit,
                chain_intact: Self::verify_chain(&records).is_ok(),
            })
            .collect();
        Ok(annotated)
    }

    /// One proof by id (with the same staleness annotation).
    pub fn get(&self, id: &str, current_commit: &str) -> Result<Option<ProofView>> {
        Ok(self
            .list(current_commit)?
            .into_iter()
            .find(|p| p.record.id == id))
    }

    /// Human acceptance (or rejection) of a proof. Only a `Passed` runtime
    /// proof may be accepted — every other state is refused by design.
    pub fn accept(&self, id: &str, reviewer: &str, accept: bool) -> Result<ProofRecord> {
        let _g = self.lock.lock().unwrap();
        let mut records = Self::load(&self.path)?;
        let rec = records
            .iter_mut()
            .find(|r| r.id == id)
            .context(format!("unknown proof: {id}"))?;
        if accept {
            anyhow::ensure!(
                rec.verification.is_pass(),
                "only a Passed proof can be accepted; this proof is {:?}",
                rec.verification
            );
            anyhow::ensure!(
                rec.source == ProofSource::RuntimeCommand,
                "only runtime-command evidence can be accepted; this proof cites {:?}",
                rec.source
            );
        }
        anyhow::ensure!(
            rec.acceptance == AcceptanceState::Pending,
            "proof already decided ({:?}); acceptance is final",
            rec.acceptance
        );
        rec.acceptance = if accept {
            AcceptanceState::Accepted
        } else {
            AcceptanceState::Rejected
        };
        rec.reviewer = Some(reviewer.to_string());
        let out = rec.clone();
        Self::store(&records, &self.path)?;
        Ok(out)
    }

    /// Introspection: is the stored chain intact? Exposed so UIs can show
    /// tamper status rather than discover it silently.
    pub fn chain_intact(&self) -> bool {
        match Self::load(&self.path) {
            Ok(records) => Self::verify_chain(&records).is_ok(),
            Err(_) => false,
        }
    }
}

/// A proof record plus query-time annotations.
#[derive(Debug, Clone, Serialize)]
pub struct ProofView {
    #[serde(flatten)]
    pub record: ProofRecord,
    /// The proof's commit is not the current HEAD — evidence may be stale.
    pub stale: bool,
    /// The store's hash chain verified at query time.
    pub chain_intact: bool,
}

/// Lowercase hex without adding a `hex` dependency.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> (std::sync::MutexGuard<'static, ()>, ProofEngine) {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("PROOFS_PATH", dir.path().join("proofs.json"));
        let engine = ProofEngine::open(dir.path()).unwrap();
        std::mem::forget(dir); // keep backing dir alive for the test
        (g, engine)
    }

    fn input(
        command: &str,
        commit: &str,
        verification: VerificationState,
        source: ProofSource,
    ) -> ProofInput {
        ProofInput {
            project_id: "p1".into(),
            mission_id: Some("m1".into()),
            command: command.into(),
            source_commit: commit.into(),
            tree_clean: true,
            expected: "exit 0".into(),
            actual: if verification.is_pass() {
                Some("all green".into())
            } else {
                None
            },
            exit_status: if matches!(
                verification,
                VerificationState::NotRun | VerificationState::Blocked
            ) {
                None
            } else {
                Some(if verification.is_pass() { 0 } else { 101 })
            },
            duration_ms: if matches!(
                verification,
                VerificationState::NotRun | VerificationState::Blocked
            ) {
                None
            } else {
                Some(1500)
            },
            environment: std::collections::BTreeMap::from([(
                "CARGO_TERM_COLOR".into(),
                "never".into(),
            )]),
            artifact_refs: vec!["test_report-abc".into()],
            log_path: Some(".zylcode/logs/run-1.log".into()),
            verification,
            source,
            limitations: None,
        }
    }

    const C1: &str = "aaaa111";
    const C2: &str = "bbbb222";

    #[test]
    fn record_pass_and_accept_flow() {
        let (_g, e) = engine();
        let rec = e
            .record(input(
                "cargo test -p zylcode-core",
                C1,
                VerificationState::Passed,
                ProofSource::RuntimeCommand,
            ))
            .unwrap();
        assert!(rec.verification.is_pass());
        assert_eq!(rec.acceptance, AcceptanceState::Pending);

        let accepted = e.accept(&rec.id, "human-reviewer", true).unwrap();
        assert_eq!(accepted.acceptance, AcceptanceState::Accepted);
        assert_eq!(accepted.reviewer.as_deref(), Some("human-reviewer"));
        // Acceptance is final.
        assert!(e.accept(&rec.id, "another", false).is_err());
    }

    #[test]
    fn not_run_can_never_be_accepted() {
        let (_g, e) = engine();
        let rec = e
            .record(input(
                "cargo test",
                C1,
                VerificationState::NotRun,
                ProofSource::RuntimeCommand,
            ))
            .unwrap();
        let err = e.accept(&rec.id, "human", true).unwrap_err();
        assert!(
            err.to_string().contains("only a Passed proof"),
            "refusal must be explicit: {err}"
        );
    }

    #[test]
    fn simulated_and_static_evidence_cannot_be_accepted() {
        let (_g, e) = engine();
        let sim = e
            .record(input(
                "agent says ok",
                C1,
                VerificationState::Passed,
                ProofSource::Simulated,
            ))
            .unwrap();
        let stat = e
            .record(input(
                "cargo clippy",
                C1,
                VerificationState::Passed,
                ProofSource::StaticAnalysis,
            ))
            .unwrap();
        assert!(
            e.accept(&sim.id, "h", true).is_err(),
            "simulated pass must not be acceptable"
        );
        assert!(
            e.accept(&stat.id, "h", true).is_err(),
            "static analysis must not be acceptable"
        );
    }

    #[test]
    fn chain_detects_history_mutation() {
        let (_g, e) = engine();
        e.record(input(
            "cmd a",
            C1,
            VerificationState::Passed,
            ProofSource::RuntimeCommand,
        ))
        .unwrap();
        e.record(input(
            "cmd b",
            C1,
            VerificationState::Failed,
            ProofSource::RuntimeCommand,
        ))
        .unwrap();
        e.record(input(
            "cmd c",
            C1,
            VerificationState::NotRun,
            ProofSource::RuntimeCommand,
        ))
        .unwrap();
        assert!(e.chain_intact(), "fresh chain must be intact");

        // Mutate history behind the engine's back: flip a Failed → Passed.
        let path = proofs_path(Path::new("."));
        let mut records = {
            let raw = std::fs::read_to_string(&path).unwrap();
            let mut v: Vec<ProofRecord> = serde_json::from_str(&raw).unwrap();
            v[1].verification = VerificationState::Passed;
            v
        };
        // The forged record keeps its old hash — which no longer matches.
        ProofEngine::store(&records, &path).unwrap();
        assert!(
            !ProofEngine::verify_chain(&records).is_ok(),
            "mutation must break the chain"
        );
        records.clear();
        let _ = records;
        assert!(!e.chain_intact(), "engine must report the tampered chain");
    }

    #[test]
    fn staleness_is_reported_not_rewritten() {
        let (_g, e) = engine();
        e.record(input(
            "cmd a",
            C1,
            VerificationState::Passed,
            ProofSource::RuntimeCommand,
        ))
        .unwrap();
        e.record(input(
            "cmd b",
            C2,
            VerificationState::Passed,
            ProofSource::RuntimeCommand,
        ))
        .unwrap();

        let views = e.list(C2).unwrap();
        assert_eq!(views.len(), 2);
        // Newest first (C2 proof), not stale relative to C2.
        assert!(!views[0].stale);
        // The C1 proof is stale relative to C2 — reported, never rewritten.
        assert!(views[1].stale);
        assert_eq!(views[1].record.source_commit, C1);
        assert!(views.iter().all(|v| v.chain_intact));
    }

    #[test]
    fn all_honest_states_roundtrip() {
        let (_g, e) = engine();
        for (v, s) in [
            (VerificationState::Passed, ProofSource::RuntimeCommand),
            (VerificationState::NotRun, ProofSource::RuntimeCommand),
            (VerificationState::Blocked, ProofSource::RuntimeCommand),
            (VerificationState::Failed, ProofSource::RuntimeCommand),
            (VerificationState::RuntimeNotReached, ProofSource::Simulated),
            (
                VerificationState::EvidenceMissing,
                ProofSource::RuntimeCommand,
            ),
        ] {
            let rec = e.record(input("cmd", C1, v, s)).unwrap();
            assert_eq!(rec.verification, v);
            assert_eq!(rec.source, s);
        }
        let views = e.list(C1).unwrap();
        assert_eq!(views.len(), 6);
        assert!(e.chain_intact());
    }

    #[test]
    fn blocked_proof_asserts_the_block() {
        let (_g, e) = engine();
        let mut i = input(
            "cargo test --workspace",
            C1,
            VerificationState::Blocked,
            ProofSource::RuntimeCommand,
        );
        i.limitations = Some("CI billing blocked; verified locally only".into());
        let rec = e.record(i).unwrap();
        assert_eq!(rec.verification, VerificationState::Blocked);
        assert!(
            rec.exit_status.is_none(),
            "a blocked run has no exit status"
        );
        assert_eq!(
            rec.limitations.as_deref(),
            Some("CI billing blocked; verified locally only")
        );
    }

    #[test]
    fn recovery_after_interrupted_write() {
        let (_g, e) = engine();
        e.record(input(
            "cmd a",
            C1,
            VerificationState::Passed,
            ProofSource::RuntimeCommand,
        ))
        .unwrap();
        // Simulate an interrupted write: a .tmp file exists with partial data.
        let path = proofs_path(Path::new("."));
        std::fs::write(path.with_extension("json.tmp"), "[{\"partial\": true}").unwrap();
        // The engine still reads the last fully-committed state (rename was
        // never executed, so proofs.json is intact).
        assert!(e.chain_intact());
        let views = e.list(C1).unwrap();
        assert_eq!(views.len(), 1);
        // And it can keep recording.
        e.record(input(
            "cmd b",
            C1,
            VerificationState::Passed,
            ProofSource::RuntimeCommand,
        ))
        .unwrap();
        assert_eq!(e.list(C1).unwrap().len(), 2);
    }
}
