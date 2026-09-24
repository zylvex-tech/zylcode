//! Best-of-N execution with evidence-scored selection.
//!
//! # Why this exists
//!
//! A single-shot agent bets everything on one attempt. Best-of-N samples N
//! candidate attempts, runs a **real** verification suite against each, and
//! selects the winner on recorded evidence. This is the shape of Codex's
//! best-of-N, with a difference that matters here: every candidate's
//! outcome — including the failures — is appended to the evidence ledger,
//! and the selection decision itself is a ledger entry carrying *why*.
//!
//! # What is honest about this module
//!
//! * The verifier is injected. The production verifier runs the real test
//!   suite in a worker process ([`TestSuiteVerifier`]); tests inject a
//!   deterministic one. Nothing in this module fabricates a pass.
//! * When every candidate fails verification the result is **no selection**
//!   with the recorded reason — never a best-effort winner with failing
//!   evidence ([`SelfVerificationGate`] enforces the same rule at the
//!   acceptance boundary).
//! * Selection is deterministic: pass/fail first, then passing-check count,
//!   then candidate order — the same tie-breaking discipline as the
//!   retrieval ranking, so a re-run at the same inputs selects the same
//!   candidate.
//! * Verification is concurrent — up to [`BestOfNConfig::max_concurrent`]
//!   candidates at once — but the ledger chain does not inherit that
//!   nondeterminism: start entries are appended first in candidate order
//!   and outcome entries follow in candidate order, so identical results
//!   replay to an identical chain regardless of completion order.
//!
//! # Sandbox boundary (recorded honestly)
//!
//! [`TestSuiteVerifier`] executes in a **worker subprocess** with a wall-
//! clock timeout. It is not yet an OS-isolated sandbox (network deny-by-
//! default, filesystem jail); that is Phase 7A work. The timeout and the
//! recorded exit evidence are the bounds that exist today.

use crate::ledger::{ExecutionState, LedgerEntry, LedgerStore};
use anyhow::Result;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Evidence from verifying one candidate. Produced by the verifier, consumed
/// by the selector, the self-verification gate, and the ledger.
#[derive(Debug, Clone)]
pub struct VerificationEvidence {
    /// Named checks with pass/fail and a detail line each — the audit trail
    /// for *why* a candidate passed or failed.
    pub checks: Vec<(String, bool, String)>,
    /// Process exit status when a real suite ran (`None` for injected
    /// verifiers that do not shell out).
    pub exit_status: Option<i32>,
    /// Truncated tail of combined output; full output goes to the ledger
    /// payload, not here.
    pub output_tail: String,
}

impl VerificationEvidence {
    pub fn passed(&self) -> bool {
        !self.checks.is_empty() && self.checks.iter().all(|(_, ok, _)| *ok)
    }

    pub fn passing_checks(&self) -> usize {
        self.checks.iter().filter(|(_, ok, _)| *ok).count()
    }
}

/// Runs real verification against one candidate.
#[async_trait::async_trait]
pub trait CandidateVerifier: Send + Sync {
    async fn verify(&self, candidate_index: usize, candidate: &str)
        -> Result<VerificationEvidence>;
}

/// Verifies a candidate by running the workspace's real test suite in a
/// worker subprocess, with a wall-clock timeout.
///
/// The candidate (e.g. a patch) is expected to have been applied to the
/// working tree by the caller — this verifier judges the tree it is given,
/// which is what "the real test suite" means.
pub struct TestSuiteVerifier {
    /// Working directory the suite runs in.
    pub working_dir: std::path::PathBuf,
    /// Per-candidate wall-clock budget.
    pub timeout: Duration,
    /// Program and args of the suite command (default: `cargo test`).
    pub command: Vec<String>,
}

impl Default for TestSuiteVerifier {
    fn default() -> Self {
        Self {
            working_dir: std::path::PathBuf::from("."),
            timeout: Duration::from_secs(600),
            command: vec!["cargo".to_string(), "test".to_string()],
        }
    }
}

const OUTPUT_TAIL_BYTES: usize = 8_000;

#[async_trait::async_trait]
impl CandidateVerifier for TestSuiteVerifier {
    async fn verify(
        &self,
        candidate_index: usize,
        _candidate: &str,
    ) -> Result<VerificationEvidence> {
        let (program, args) = self
            .command
            .split_first()
            .ok_or_else(|| anyhow::anyhow!("suite command must not be empty"))?;

        let run = tokio::time::timeout(
            self.timeout,
            tokio::process::Command::new(program)
                .args(args)
                .current_dir(&self.working_dir)
                .output(),
        )
        .await;

        match run {
            Err(_) => Ok(VerificationEvidence {
                checks: vec![(
                    "suite_completed_within_budget".to_string(),
                    false,
                    format!("timed out after {:?}", self.timeout),
                )],
                exit_status: None,
                output_tail: String::new(),
            }),
            Ok(Err(e)) => Ok(VerificationEvidence {
                checks: vec![(
                    "suite_spawned".to_string(),
                    false,
                    format!("failed to spawn suite: {e}"),
                )],
                exit_status: None,
                output_tail: String::new(),
            }),
            Ok(Ok(output)) => {
                let combined = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                let tail: String = combined
                    .chars()
                    .rev()
                    .take(OUTPUT_TAIL_BYTES)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                let success = output.status.success();
                Ok(VerificationEvidence {
                    checks: vec![(
                        format!("test_suite[{}]", candidate_index),
                        success,
                        format!("exit status: {:?}", output.status.code()),
                    )],
                    exit_status: output.status.code(),
                    output_tail: tail,
                })
            }
        }
    }
}

/// The record of one candidate's life under verification.
#[derive(Debug, Clone)]
pub struct CandidateOutcome {
    pub candidate_index: usize,
    pub evidence: VerificationEvidence,
    pub passed: bool,
}

/// The auditable result of one Best-of-N run.
#[derive(Debug, Clone)]
pub struct BestOfNResult {
    /// Index of the selected candidate, or `None` when nothing verified.
    pub selected: Option<usize>,
    /// Every candidate's outcome, failures included.
    pub outcomes: Vec<CandidateOutcome>,
    /// Why the winner won — or why nothing was selected.
    pub selection_reason: String,
}

/// Configuration for a Best-of-N run.
#[derive(Debug, Clone)]
pub struct BestOfNConfig {
    /// How many candidates to sample and verify.
    pub candidates: usize,
    /// Per-candidate verification budget (used by verifiers that take one).
    pub per_candidate_timeout: Duration,
    /// How many candidates may verify at once. Verification work is
    /// independent per candidate (each typically runs its own suite
    /// subprocess in its own directory), so N candidates cost roughly one
    /// suite run instead of N. `1` reproduces the historical sequential
    /// behaviour exactly.
    pub max_concurrent: usize,
}

impl Default for BestOfNConfig {
    fn default() -> Self {
        Self {
            candidates: 3,
            per_candidate_timeout: Duration::from_secs(600),
            max_concurrent: 4,
        }
    }
}

/// Score used for selection: passed first, then number of passing checks,
/// then candidate order. Total order — no dependence on iteration order.
fn score(outcome: &CandidateOutcome) -> (u8, usize, usize) {
    (
        u8::from(outcome.passed),
        outcome.evidence.passing_checks(),
        usize::MAX - outcome.candidate_index,
    )
}

/// Run Best-of-N over `candidates`, verify each, select on evidence, and
/// append every outcome plus the selection decision to `ledger`.
///
/// Verification is concurrent: up to [`BestOfNConfig::max_concurrent`]
/// candidates run at once, each with its own wall-clock budget, so N
/// candidates cost roughly one suite run instead of N. The ledger does not
/// inherit that nondeterminism — start entries are appended first in
/// candidate order and outcome entries afterwards in candidate order, and
/// selection is a total order over the recorded evidence — so the same
/// results replay to the same chain no matter which candidate finished
/// first.
///
/// The ledger entries chain among themselves using the same
/// `(prev_hash, action_id)` hash formula as the agent loop, so the run is
/// auditable with the same replay procedure an auditor already uses.
pub async fn run_best_of_n(
    candidates: &[String],
    verifier: Arc<dyn CandidateVerifier>,
    ledger: Option<Arc<dyn LedgerStore>>,
    session_id: Option<Uuid>,
    config: &BestOfNConfig,
) -> Result<BestOfNResult> {
    let n = config.candidates.min(candidates.len().max(1));
    let mut prev_hash = String::new(); // genesis, same convention as agent.rs

    // 1. Start entries go in first, in candidate order: the chain stays
    //    linear and deterministic even though verification is concurrent.
    if let (Some(ledger), Some(session_id)) = (&ledger, session_id) {
        for i in 0..n {
            let entry = LedgerEntry {
                id: Uuid::new_v4(),
                session_id,
                action_id: format!("best_of_n.candidate[{i}]"),
                arguments: json!({ "candidate_index": i }),
                state: ExecutionState::Started,
                prev_hash: prev_hash.clone(),
                timestamp: chrono::Utc::now(),
                payload: None,
                error: None,
            };
            prev_hash = chain_hash(&prev_hash, &entry.action_id);
            ledger.append(entry).await?;
        }
    }

    // 2. Verify concurrently. Each task holds a semaphore permit for its
    //    whole verification (a real cap on suite subprocesses), enforces
    //    its own budget, and guards against verifier panics so every
    //    candidate still produces an outcome. `join_next` yields in
    //    completion order; results are slotted back by candidate index
    //    before anything reads them.
    let max_concurrent = config.max_concurrent.max(1).min(n.max(1));
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
    let mut tasks = tokio::task::JoinSet::new();
    for (i, candidate) in candidates.iter().take(n).enumerate() {
        let verifier = Arc::clone(&verifier);
        let candidate = candidate.clone();
        let semaphore = Arc::clone(&semaphore);
        let budget = config.per_candidate_timeout;
        tasks.spawn(async move {
            let _permit = semaphore.acquire_owned().await.ok();
            let inner_verifier = Arc::clone(&verifier);
            // The verifier runs in an inner task so a panic degrades to a
            // failing outcome for *this* candidate instead of escaping with
            // the candidate's index lost.
            let inner = tokio::spawn(async move { inner_verifier.verify(i, &candidate).await });
            let guarded = async {
                match inner.await {
                    Ok(result) => result,
                    Err(join_err) => {
                        Err(anyhow::anyhow!("verifier task failed: {join_err}"))
                    }
                }
            };
            let evidence = match tokio::time::timeout(budget, guarded).await {
                Err(_) => VerificationEvidence {
                    checks: vec![(
                        "verification_completed".to_string(),
                        false,
                        format!("verification timed out after {budget:?}"),
                    )],
                    exit_status: None,
                    output_tail: String::new(),
                },
                Ok(Err(e)) => VerificationEvidence {
                    checks: vec![(
                        "verification_ran".to_string(),
                        false,
                        format!("verifier error: {e}"),
                    )],
                    exit_status: None,
                    output_tail: String::new(),
                },
                Ok(Ok(e)) => e,
            };
            (i, evidence)
        });
    }

    let mut slots: Vec<Option<VerificationEvidence>> = (0..n).map(|_| None).collect();
    while let Some(joined) = tasks.join_next().await {
        if let Ok((i, evidence)) = joined {
            if i < slots.len() {
                slots[i] = Some(evidence);
            }
        }
    }

    // 3. Outcomes and their ledger entries, in candidate order.
    let mut outcomes = Vec::with_capacity(slots.len());
    for (i, slot) in slots.into_iter().enumerate() {
        let evidence = slot.unwrap_or_else(|| VerificationEvidence {
            checks: vec![(
                "verification_ran".to_string(),
                false,
                "verifier produced no result".to_string(),
            )],
            exit_status: None,
            output_tail: String::new(),
        });

        let passed = evidence.passed();
        outcomes.push(CandidateOutcome {
            candidate_index: i,
            evidence: evidence.clone(),
            passed,
        });

        if let (Some(ledger), Some(session_id)) = (&ledger, session_id) {
            let entry_id = Uuid::new_v4();
            let payload = json!({
                "candidate_index": i,
                "passed": passed,
                "checks": evidence
                    .checks
                    .iter()
                    .map(|(name, ok, detail)| json!({
                        "check": name, "passed": ok, "detail": detail
                    }))
                    .collect::<Vec<_>>(),
                "exit_status": evidence.exit_status,
                "output_tail": evidence.output_tail,
            });
            let entry = LedgerEntry {
                id: entry_id,
                session_id,
                action_id: format!("best_of_n.candidate[{i}]"),
                arguments: json!({ "candidate_index": i }),
                state: if passed {
                    ExecutionState::Recorded
                } else {
                    ExecutionState::Failed
                },
                prev_hash: prev_hash.clone(),
                timestamp: chrono::Utc::now(),
                payload: Some(payload),
                error: if passed {
                    None
                } else {
                    Some("verification failed".to_string())
                },
            };
            prev_hash = chain_hash(&prev_hash, &entry.action_id);
            ledger.append(entry).await?;
        }
    }

    // Evidence-scored selection. A candidate that produced no checks (an
    // errored verifier) can never win: `passed()` is false for it.
    let winner = outcomes
        .iter()
        .filter(|o| o.passed)
        .max_by_key(|o| score(o))
        .cloned();

    let result = match winner {
        Some(w) => BestOfNResult {
            selected: Some(w.candidate_index),
            outcomes,
            selection_reason: format!(
                "candidate {} selected: {} of {} checks passing, all checks green",
                w.candidate_index,
                w.evidence.passing_checks(),
                w.evidence.checks.len()
            ),
        },
        None => BestOfNResult {
            selected: None,
            outcomes,
            selection_reason: "no candidate passed verification; refusing to select"
                .to_string(),
        },
    };

    // The selection decision is itself evidence.
    if let (Some(ledger), Some(session_id)) = (&ledger, session_id) {
        let entry = LedgerEntry {
            id: Uuid::new_v4(),
            session_id,
            action_id: "best_of_n.selection".to_string(),
            arguments: json!({ "candidates": n }),
            state: if result.selected.is_some() {
                ExecutionState::Recorded
            } else {
                ExecutionState::Failed
            },
            prev_hash,
            timestamp: chrono::Utc::now(),
            payload: Some(json!({
                "selected": result.selected,
                "reason": result.selection_reason,
                "outcomes": result
                    .outcomes
                    .iter()
                    .map(|o| json!({
                        "candidate_index": o.candidate_index,
                        "passed": o.passed,
                        "passing_checks": o.evidence.passing_checks(),
                    }))
                    .collect::<Vec<_>>(),
            })),
            error: None,
        };
        ledger.append(entry).await?;
    }

    Ok(result)
}

/// Same chain formula as the agent loop: hash of `(prev_hash, action_id)`.
fn chain_hash(prev_hash: &str, action_id: &str) -> String {
    let content = format!("{prev_hash}:{action_id}");
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

/// The acceptance boundary: refuse to accept a candidate whose evidence does
/// not independently justify acceptance.
///
/// This is the self-verification discipline — the harness re-checks its own
/// work before declaring success instead of trusting the attempt. A
/// candidate with no checks, a failed check, or evidence from a different
/// candidate is refused, and the refusal says why.
pub struct SelfVerificationGate;

impl SelfVerificationGate {
    /// Decide whether `evidence` justifies accepting `candidate_index`.
    /// `Err` carries the audit-facing reason for the refusal.
    pub fn accept(
        &self,
        candidate_index: usize,
        evidence: &VerificationEvidence,
    ) -> std::result::Result<(), String> {
        if evidence.checks.is_empty() {
            return Err(format!(
                "candidate {candidate_index}: no verification checks recorded — refusing to accept"
            ));
        }
        let failed: Vec<&(String, bool, String)> =
            evidence.checks.iter().filter(|(_, ok, _)| !ok).collect();
        if !failed.is_empty() {
            let names: Vec<&str> = failed.iter().map(|(n, _, _)| n.as_str()).collect();
            return Err(format!(
                "candidate {candidate_index}: failing checks {names:?} — refusing to accept"
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_ledger::MemoryLedgerStore;
    use std::collections::HashMap;

    /// Deterministic injected verifier: candidates named "pass" verify green,
    /// everything else verifies red. No sleeps, no fabrication beyond the
    /// test's own say-so.
    struct ScriptedVerifier {
        results: HashMap<usize, bool>,
    }

    #[async_trait::async_trait]
    impl CandidateVerifier for ScriptedVerifier {
        async fn verify(
            &self,
            candidate_index: usize,
            _candidate: &str,
        ) -> Result<VerificationEvidence> {
            let ok = self.results.get(&candidate_index).copied().unwrap_or(false);
            Ok(VerificationEvidence {
                checks: vec![(
                    "scripted".to_string(),
                    ok,
                    if ok { "green" } else { "red" }.to_string(),
                )],
                exit_status: Some(if ok { 0 } else { 1 }),
                output_tail: String::new(),
            })
        }
    }

    fn ledger_session() -> (Arc<MemoryLedgerStore>, Uuid) {
        (Arc::new(MemoryLedgerStore::new()), Uuid::new_v4())
    }

    #[tokio::test]
    async fn selects_the_passing_candidate_and_records_every_outcome() {
        let (ledger, session) = ledger_session();
        let verifier = Arc::new(ScriptedVerifier {
            results: [(0, false), (1, true), (2, false)].into_iter().collect(),
        });

        let result = run_best_of_n(
            &["a".into(), "b".into(), "c".into()],
            verifier,
            Some(Arc::clone(&ledger) as Arc<dyn LedgerStore>),
            Some(session),
            &BestOfNConfig::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.selected, Some(1));
        assert!(result.selection_reason.contains("candidate 1"));
        assert_eq!(result.outcomes.len(), 3, "every candidate is recorded");

        let entries = ledger
            .get_entries(session)
            .await
            .unwrap();
        // 3 candidates × 2 entries + 1 selection entry.
        assert_eq!(entries.len(), 7);
        let selection = entries
            .iter()
            .find(|e| e.action_id == "best_of_n.selection")
            .expect("the selection decision must be a ledger entry");
        let payload = selection.payload.as_ref().unwrap();
        assert_eq!(payload["selected"], 1);
        assert!(payload["reason"].as_str().unwrap().contains("candidate 1"));
    }

    #[tokio::test]
    async fn all_candidates_failing_selects_nothing_and_records_why() {
        let (ledger, session) = ledger_session();
        let verifier = Arc::new(ScriptedVerifier {
            results: HashMap::new(), // everything red
        });

        let result = run_best_of_n(
            &["a".into(), "b".into()],
            verifier,
            Some(Arc::clone(&ledger) as Arc<dyn LedgerStore>),
            Some(session),
            &BestOfNConfig::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.selected, None);
        assert!(result.selection_reason.contains("refusing to select"));

        let selection = ledger
            .get_entries(session)
            .await
            .unwrap()
            .into_iter()
            .find(|e| e.action_id == "best_of_n.selection")
            .unwrap();
        assert_eq!(selection.state, ExecutionState::Failed);
    }

    #[tokio::test]
    async fn ledger_entries_from_the_run_form_an_intact_chain() {
        use sha2::{Digest, Sha256};
        let (ledger, session) = ledger_session();
        let verifier = Arc::new(ScriptedVerifier {
            results: [(0, true)].into_iter().collect(),
        });

        run_best_of_n(
            &["a".into()],
            verifier,
            Some(Arc::clone(&ledger) as Arc<dyn LedgerStore>),
            Some(session),
            &BestOfNConfig::default(),
        )
        .await
        .unwrap();

        let entries = ledger.get_entries(session).await.unwrap();
        let mut prev_hash = String::new();
        for entry in &entries {
            assert_eq!(entry.prev_hash, prev_hash, "chain must be intact");
            prev_hash = {
                let content = format!("{}:{}", entry.prev_hash, entry.action_id);
                format!("{:x}", Sha256::digest(content.as_bytes()))
            };
        }
    }

    #[tokio::test]
    async fn the_gate_refuses_unevidenced_acceptance() {
        let gate = SelfVerificationGate;
        // No checks at all: refused.
        let empty = VerificationEvidence {
            checks: vec![],
            exit_status: Some(0),
            output_tail: String::new(),
        };
        assert!(gate.accept(0, &empty).is_err());

        // A failing check: refused, and the reason names it.
        let failing = VerificationEvidence {
            checks: vec![
                ("suite".to_string(), true, "ok".to_string()),
                ("lint".to_string(), false, "warnings present".to_string()),
            ],
            exit_status: Some(0),
            output_tail: String::new(),
        };
        let err = gate.accept(2, &failing).unwrap_err();
        assert!(err.contains("lint") && err.contains("candidate 2"), "{err}");

        // Fully green evidence: accepted.
        let green = VerificationEvidence {
            checks: vec![("suite".to_string(), true, "432 passed".to_string())],
            exit_status: Some(0),
            output_tail: String::new(),
        };
        assert!(gate.accept(0, &green).is_ok());
    }

    #[tokio::test]
    async fn config_cannot_sample_more_candidates_than_exist() {
        let verifier = Arc::new(ScriptedVerifier {
            results: [(0, true)].into_iter().collect(),
        });
        let result = run_best_of_n(
            &["only".into()],
            verifier,
            None,
            None,
            &BestOfNConfig {
                candidates: 5,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(result.outcomes.len(), 1);
    }

    /// Verifier that records how many verifications overlap in time and
    /// sleeps a per-candidate duration, to probe real concurrency.
    struct ConcurrencyProbeVerifier {
        /// Sleep per candidate index (candidate_index -> millis).
        delays_ms: HashMap<usize, u64>,
        active: std::sync::atomic::AtomicUsize,
        peak: std::sync::atomic::AtomicUsize,
    }

    impl ConcurrencyProbeVerifier {
        fn new(delays_ms: HashMap<usize, u64>) -> Self {
            Self {
                delays_ms,
                active: std::sync::atomic::AtomicUsize::new(0),
                peak: std::sync::atomic::AtomicUsize::new(0),
            }
        }

        fn peak(&self) -> usize {
            self.peak.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl CandidateVerifier for ConcurrencyProbeVerifier {
        async fn verify(
            &self,
            candidate_index: usize,
            _candidate: &str,
        ) -> Result<VerificationEvidence> {
            let now_active =
                self.active.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            self.peak.fetch_max(now_active, std::sync::atomic::Ordering::SeqCst);
            let delay = self.delays_ms.get(&candidate_index).copied().unwrap_or(0);
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            self.active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            Ok(VerificationEvidence {
                checks: vec![(
                    "probe".to_string(),
                    true,
                    format!("candidate {candidate_index}"),
                )],
                exit_status: Some(0),
                output_tail: String::new(),
            })
        }
    }

    #[tokio::test]
    async fn concurrent_verification_finishes_in_roughly_one_suite_run() {
        let verifier = Arc::new(ConcurrencyProbeVerifier::new(
            [(0, 500), (1, 500), (2, 500), (3, 500)].into_iter().collect(),
        ));
        let started = std::time::Instant::now();
        let result = run_best_of_n(
            &["a".into(), "b".into(), "c".into(), "d".into()],
            Arc::clone(&verifier) as Arc<dyn CandidateVerifier>,
            None,
            None,
            &BestOfNConfig {
                candidates: 4,
                per_candidate_timeout: Duration::from_secs(30),
                max_concurrent: 4,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.selected, Some(0), "all pass; lowest index wins");
        assert_eq!(verifier.peak(), 4, "all four verifications overlapped");
        // Sequentially this is 2s of sleeps; concurrently well under half.
        assert!(
            started.elapsed() < std::time::Duration::from_millis(1_500),
            "verification did not overlap: {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn max_concurrent_caps_how_many_verify_at_once() {
        let verifier = Arc::new(ConcurrencyProbeVerifier::new(
            [(0, 250), (1, 250), (2, 250), (3, 250)].into_iter().collect(),
        ));
        let result = run_best_of_n(
            &["a".into(), "b".into(), "c".into(), "d".into()],
            Arc::clone(&verifier) as Arc<dyn CandidateVerifier>,
            None,
            None,
            &BestOfNConfig {
                candidates: 4,
                per_candidate_timeout: Duration::from_secs(30),
                max_concurrent: 1,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.outcomes.len(), 4);
        assert_eq!(verifier.peak(), 1, "the cap must be a hard ceiling");
    }

    #[tokio::test]
    async fn outcomes_and_chain_do_not_depend_on_completion_order() {
        use sha2::{Digest, Sha256};
        let (ledger, session) = ledger_session();
        // Candidate 2 finishes first, candidate 0 last.
        let verifier = Arc::new(ConcurrencyProbeVerifier::new(
            [(0, 300), (1, 200), (2, 100)].into_iter().collect(),
        ));
        let result = run_best_of_n(
            &["a".into(), "b".into(), "c".into()],
            Arc::clone(&verifier) as Arc<dyn CandidateVerifier>,
            Some(Arc::clone(&ledger) as Arc<dyn LedgerStore>),
            Some(session),
            &BestOfNConfig {
                candidates: 3,
                per_candidate_timeout: Duration::from_secs(30),
                max_concurrent: 3,
            },
        )
        .await
        .unwrap();

        // Outcomes are strictly in candidate order even though candidate 2
        // completed first.
        for (position, outcome) in result.outcomes.iter().enumerate() {
            assert_eq!(outcome.candidate_index, position);
        }

        // The chain replays exactly as it would have sequentially.
        let entries = ledger.get_entries(session).await.unwrap();
        let mut prev_hash = String::new();
        for entry in &entries {
            assert_eq!(entry.prev_hash, prev_hash);
            prev_hash = format!(
                "{:x}",
                Sha256::digest(format!("{}:{}", entry.prev_hash, entry.action_id).as_bytes())
            );
        }
    }
}
