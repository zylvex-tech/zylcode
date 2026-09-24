//! Patch-sampling Best-of-N (Phase 3A).
//!
//! # What this adds to `best_of_n`
//!
//! [`crate::best_of_n`] verifies candidates someone else produced. This
//! module closes the loop end to end:
//!
//! 1. a [`CandidateSource`] produces N **unified-diff patches** for a task
//!    (the production source asks the model; tests inject scripted diffs);
//! 2. the [`PatchWorktreeVerifier`] verifies each patch in **isolation** —
//!    a detached `git worktree` at the base commit, patch applied there, the
//!    real suite run there — so candidates cannot corrupt each other's
//!    evidence or the caller's working tree;
//! 3. the winner is selected on recorded evidence (never fabricated), and
//!    the winning patch is exported byte-identical to what was verified.
//!
//! # Honest limits (recorded, not hidden)
//!
//! * The base for every candidate is the repository's **current HEAD**. A
//!   task that needs a different base must move HEAD first — there is no
//!   base-selection policy yet.
//! * The suite still runs as a plain subprocess inside the worktree, not an
//!   OS sandbox (Phase 7A).
//! * An unparseable or empty model response becomes a candidate that fails
//!   verification. It is never silently retried into a fake pass.

use crate::agent::ModelClient;
use crate::best_of_n::{
    run_best_of_n, BestOfNConfig, BestOfNResult, CandidateVerifier, VerificationEvidence,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sha2::Digest;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Produces candidate patches for a task.
#[async_trait]
pub trait CandidateSource: Send + Sync {
    /// Up to `n` unified-diff candidates. Sources may return fewer; an
    /// empty result means "no candidates could be generated", which the
    /// runner reports rather than papering over.
    async fn generate(&self, task: &str, n: usize) -> Result<Vec<String>>;
}

/// Production source: ask the model for a unified diff, once per candidate.
///
/// The prompt demands a unified diff and nothing else. Whatever comes back
/// is the candidate — a response that is not a diff simply fails to apply
/// later, which is the fail-closed behaviour we want from a probabilistic
/// generator.
pub struct ModelPatchSource {
    pub client: Arc<dyn ModelClient>,
    /// Extra context prepended to the task (repo summary, entry points…).
    pub context: Option<String>,
}

const MODEL_SYSTEM_PROMPT: &str = "You are a software engineering agent. \
Respond with a single unified diff (git format) that implements the task. \
No prose, no code fences — only the diff.";

#[async_trait]
impl CandidateSource for ModelPatchSource {
    async fn generate(&self, task: &str, n: usize) -> Result<Vec<String>> {
        let prompt = match &self.context {
            Some(ctx) => format!("{ctx}\n\nTask: {task}"),
            None => task.to_string(),
        };
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            // One sample per call: independent attempts, not N copies of one.
            let response = self.client.call(&prompt, MODEL_SYSTEM_PROMPT).await?;
            // Normalize, don't fabricate: strip surrounding prose/whitespace,
            // then guarantee the trailing newline a unified diff needs for
            // `git apply` (an empty response stays empty — it fails later).
            let trimmed = response.trim();
            out.push(if trimmed.is_empty() {
                String::new()
            } else {
                format!("{trimmed}\n")
            });
        }
        Ok(out)
    }
}

/// The working tree's own uncommitted diff as a single candidate —
/// "verify what I have before I commit it".
pub struct WorkingTreeSource {
    /// Repository to diff. The CLI passes the user's project root; relying
    /// on the process cwd would make behaviour depend on launch directory.
    pub repo_root: PathBuf,
}

#[async_trait]
impl CandidateSource for WorkingTreeSource {
    async fn generate(&self, _task: &str, n: usize) -> Result<Vec<String>> {
        anyhow::ensure!(n >= 1, "at least one candidate must be requested");
        let output = tokio::process::Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(&self.repo_root)
            .output()
            .await
            .context("failed to run `git diff HEAD`")?;
        anyhow::ensure!(
            output.status.success(),
            "git diff HEAD failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(vec![String::from_utf8_lossy(&output.stdout).to_string()])
    }
}

/// Scripted candidates. Tests only — named so the injection is explicit.
pub struct ScriptedPatchSource {
    pub patches: Vec<String>,
}

#[async_trait]
impl CandidateSource for ScriptedPatchSource {
    async fn generate(&self, _task: &str, n: usize) -> Result<Vec<String>> {
        Ok(self.patches.iter().take(n).cloned().collect())
    }
}

/// One candidate's isolated git worktree: `git worktree add --detach` at
/// HEAD, patch applied inside, removed on demand. Shared by the plain
/// verifier and the sandboxed one so both auditors read the same lifecycle.
pub struct CandidateWorktree {
    /// Scratch directory containing the worktree (and the patch file).
    pub scratch: PathBuf,
    /// The worktree root — a checkout of HEAD.
    pub root: PathBuf,
}

impl CandidateWorktree {
    /// Create the worktree. `Ok(worktree)` on success; `Err(detail)` carries
    /// the git failure for the evidence trail.
    pub async fn create(repo_root: &Path, candidate_index: usize) -> Result<Self, String> {
        let scratch = std::env::temp_dir().join(format!(
            "zylcode-bon-{}-{}",
            Uuid::new_v4(),
            candidate_index
        ));
        let root = scratch.join("wt");
        let out = tokio::process::Command::new("git")
            .args([
                "worktree",
                "add",
                "--detach",
                root.to_string_lossy().as_ref(),
                "HEAD",
            ])
            .current_dir(repo_root)
            .output()
            .await;
        match out {
            Ok(out) if out.status.success() => Ok(Self { scratch, root }),
            Ok(out) => Err(format!(
                "git worktree add failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
            Err(e) => Err(format!("spawn failed: {e}")),
        }
    }

    /// Write the patch into the scratch dir and apply it inside the
    /// worktree. `Err(detail)` on any failure.
    pub async fn apply_patch(&self, patch: &str) -> Result<(), String> {
        let patch_file = self.scratch.join("candidate.patch");
        if let Err(e) = std::fs::write(&patch_file, patch) {
            return Err(format!("cannot write patch: {e}"));
        }
        let out = tokio::process::Command::new("git")
            .args([
                "apply",
                "--whitespace=nowarn",
                patch_file.to_string_lossy().as_ref(),
            ])
            .current_dir(&self.root)
            .output()
            .await;
        match out {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
            Err(e) => Err(format!("spawn failed: {e}")),
        }
    }

    /// Remove the worktree and its scratch directory (best-effort).
    pub async fn remove(&self) {
        let _ = tokio::process::Command::new("git")
            .args([
                "worktree",
                "remove",
                "--force",
                self.root.to_string_lossy().as_ref(),
            ])
            .output()
            .await;
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

/// Truncated tail of combined process output, for evidence payloads.
pub(crate) fn output_tail(stdout: &[u8], stderr: &[u8]) -> String {
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    );
    combined
        .chars()
        .rev()
        .take(8_000)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

/// Verifies one patch in an isolated `git worktree`.
///
/// Per candidate: `git worktree add --detach <tmp> HEAD`, apply the patch
/// there, run the suite there, then `git worktree remove --force`. The
/// caller's working tree is never touched, so candidates cannot poison each
/// other's evidence. Isolation is per-candidate directory only — for
/// OS-level isolation use [`crate::sandbox::SandboxedWorktreeVerifier`].
pub struct PatchWorktreeVerifier {
    /// Repository (with a HEAD) to branch worktrees from.
    pub repo_root: PathBuf,
    /// Suite wall-clock budget inside the worktree.
    pub timeout: Duration,
    /// Suite command (default `cargo test`).
    pub command: Vec<String>,
    /// Keep failing worktrees for post-mortem (never in CI).
    pub keep_on_failure: bool,
}

impl PatchWorktreeVerifier {
    pub fn new(repo_root: impl Into<PathBuf>, timeout: Duration) -> Self {
        Self {
            repo_root: repo_root.into(),
            timeout,
            command: vec!["cargo".into(), "test".into()],
            keep_on_failure: false,
        }
    }
}

#[async_trait]
impl CandidateVerifier for PatchWorktreeVerifier {
    async fn verify(&self, candidate_index: usize, patch: &str) -> Result<VerificationEvidence> {
        let mut checks: Vec<(String, bool, String)> = Vec::new();
        let mut keep = self.keep_on_failure;

        // 1. Isolated worktree at HEAD.
        let worktree = match CandidateWorktree::create(&self.repo_root, candidate_index).await {
            Ok(wt) => {
                checks.push(("worktree_created".into(), true, wt.root.display().to_string()));
                wt
            }
            Err(detail) => {
                checks.push(("worktree_created".into(), false, detail));
                return Ok(VerificationEvidence { checks, exit_status: None, output_tail: String::new() });
            }
        };

        // 2. Apply the patch inside the worktree.
        if let Err(detail) = worktree.apply_patch(patch).await {
            checks.push(("patch_applied".into(), false, detail));
            if !self.keep_on_failure {
                worktree.remove().await;
            }
            return Ok(VerificationEvidence { checks, exit_status: None, output_tail: String::new() });
        }
        checks.push(("patch_applied".into(), true, "applied cleanly".into()));

        // 3. The real suite, inside the worktree.
        let suite_result = match self.command.split_first() {
            None => {
                checks.push(("suite_configured".into(), false, "empty suite command".into()));
                None
            }
            Some((program, args)) => {
                let run = tokio::time::timeout(
                    self.timeout,
                    tokio::process::Command::new(program)
                        .args(args)
                        .current_dir(&worktree.root)
                        .output(),
                )
                .await;
                match run {
                    Err(_) => {
                        keep = true;
                        checks.push((
                            "test_suite".into(),
                            false,
                            format!("timed out after {:?}", self.timeout),
                        ));
                        None
                    }
                    Ok(Err(e)) => {
                        checks.push(("test_suite".into(), false, format!("spawn failed: {e}")));
                        None
                    }
                    Ok(Ok(out)) => {
                        let success = out.status.success();
                        checks.push((
                            "test_suite".into(),
                            success,
                            format!("exit status: {:?}", out.status.code()),
                        ));
                        Some(out)
                    }
                }
            }
        };

        if !keep {
            worktree.remove().await;
        }

        let (exit_status, tail) = match suite_result {
            Some(out) => (out.status.code(), output_tail(&out.stdout, &out.stderr)),
            None => (None, String::new()),
        };

        Ok(VerificationEvidence { checks, exit_status, output_tail: tail })
    }
}

/// The end-to-end result: selection evidence plus the exported winner.
#[derive(Debug, Clone)]
pub struct PatchBestOfNResult {
    /// The underlying evidence-scored selection.
    pub selection: BestOfNResult,
    /// All generated candidates, index-aligned with `selection.outcomes`.
    pub patches: Vec<String>,
}

impl PatchBestOfNResult {
    /// The winning patch — byte-identical to what was verified.
    pub fn winning_patch(&self) -> Option<&str> {
        self.selection
            .selected
            .and_then(|i| self.patches.get(i))
            .map(|s| s.as_str())
    }

    /// Export the winner to `path` (parent created). Returns bytes written.
    pub fn export_winner(&self, path: &Path) -> Result<usize> {
        let patch = self
            .winning_patch()
            .ok_or_else(|| anyhow::anyhow!("no winning patch to export"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, patch)?;
        Ok(patch.len())
    }
}

/// Generate, verify in isolation, select on evidence, export.
///
/// `verifier_override` replaces the default worktree verifier — used to
/// inject [`crate::sandbox::SandboxedWorktreeVerifier`] (or any other
/// [`CandidateVerifier`]); with `None` the plain per-candidate worktree
/// verifier runs the suite on the host.
///
/// When `ledger` is provided a **fresh session** should be used — the run
/// writes a self-contained chain starting at genesis.
#[allow(clippy::too_many_arguments)]
pub async fn run_patch_best_of_n(
    task: &str,
    source: Arc<dyn CandidateSource>,
    verifier_override: Option<Arc<dyn crate::best_of_n::CandidateVerifier>>,
    repo_root: &Path,
    ledger: Option<Arc<dyn crate::ledger::LedgerStore>>,
    session_id: Option<Uuid>,
    config: &BestOfNConfig,
    suite_command: Vec<String>,
    suite_timeout: Duration,
) -> Result<PatchBestOfNResult> {
    let patches = source.generate(task, config.candidates).await?;
    anyhow::ensure!(
        !patches.is_empty(),
        "no candidates could be generated for this task"
    );

    let verifier: Arc<dyn crate::best_of_n::CandidateVerifier> = match verifier_override {
        Some(v) => v,
        None => Arc::new(PatchWorktreeVerifier {
            repo_root: repo_root.to_path_buf(),
            timeout: suite_timeout,
            command: suite_command,
            keep_on_failure: false,
        }),
    };

    let selection = run_best_of_n(
        &patches,
        verifier,
        ledger,
        session_id,
        config,
    )
    .await?;

    Ok(PatchBestOfNResult { selection, patches })
}

/// Append a `best_of_n.patch_export` entry continuing the run's chain.
pub async fn record_patch_export(
    ledger: &Arc<dyn crate::ledger::LedgerStore>,
    session_id: Uuid,
    candidate_index: usize,
    path: &Path,
    bytes: usize,
) -> Result<Uuid> {
    // Continue the chain the run wrote (fresh session ⇒ genesis).
    let entries = ledger
        .get_entries(session_id)
        .await
        .context("cannot read ledger to continue the chain")?;
    let prev_hash = match entries.last() {
        Some(last) => {
            let content = format!("{}:{}", last.prev_hash, last.action_id);
            format!("{:x}", sha2::Sha256::digest(content.as_bytes()))
        }
        None => String::new(),
    };
    let id = Uuid::new_v4();
    ledger
        .append(crate::ledger::LedgerEntry {
            id,
            session_id,
            action_id: "best_of_n.patch_export".to_string(),
            arguments: serde_json::json!({ "candidate_index": candidate_index }),
            state: crate::ledger::ExecutionState::Recorded,
            prev_hash,
            timestamp: chrono::Utc::now(),
            payload: Some(serde_json::json!({
                "candidate_index": candidate_index,
                "path": path.display().to_string(),
                "bytes": bytes,
            })),
            error: None,
        })
        .await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::best_of_n::BestOfNConfig;
    use crate::ledger::LedgerStore;
    use crate::memory_ledger::MemoryLedgerStore;
    use std::process::Command;

    /// A minimal git repo with one commit, so worktrees have a HEAD.
    /// Shared across tests: one base repo is enough (worktrees are per-candidate
    /// temps). `TempDir::keep()` keeps the directory for the process lifetime
    /// without a leaking `forget`.
    fn shared_git_repo() -> &'static PathBuf {
        static REPO: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        REPO.get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().to_path_buf();
            let run = |args: &[&str]| {
                let out = Command::new("git")
                    .args(args)
                    .current_dir(&root)
                    .output()
                    .unwrap();
                assert!(
                    out.status.success(),
                    "git {:?} failed: {}",
                    args,
                    String::from_utf8_lossy(&out.stderr)
                );
            };
            run(&["init", "-q"]);
            run(&["config", "user.email", "test@example.com"]);
            run(&["config", "user.name", "test"]);
            std::fs::write(root.join("README.md"), "# t\n").unwrap();
            run(&["add", "."]);
            run(&["commit", "-q", "-m", "base"]);
            dir.keep()
        })
    }

    fn add_file_patch(name: &str, body: &str) -> String {
        format!(
            "diff --git a/{name} b/{name}\nnew file mode 100644\n--- /dev/null\n+++ b/{name}\n@@ -0,0 +1 @@\n+{body}\n"
        )
    }

    fn fast_pass_command() -> Vec<String> {
        // A "suite" that always succeeds, so tests exercise apply/verify
        // isolation without compiling a crate.
        if cfg!(target_os = "windows") {
            vec!["cmd".into(), "/C".into(), "exit".into(), "0".into()]
        } else {
            vec!["true".into()]
        }
    }

    fn fast_fail_command() -> Vec<String> {
        if cfg!(target_os = "windows") {
            vec!["cmd".into(), "/C".into(), "exit".into(), "1".into()]
        } else {
            vec!["false".into()]
        }
    }

    async fn run(
        patches: Vec<String>,
        suite: Vec<String>,
        ledger: Option<(Arc<MemoryLedgerStore>, Uuid)>,
    ) -> PatchBestOfNResult {
        let root: &PathBuf = shared_git_repo();
        let (ledger_arc, session) = match &ledger {
            Some((l, s)) => (Some(Arc::clone(l) as Arc<dyn crate::ledger::LedgerStore>), Some(*s)),
            None => (None, None),
        };
        run_patch_best_of_n(
            "add a file",
            Arc::new(ScriptedPatchSource { patches }),
            None,
            root,
            ledger_arc,
            session,
            &BestOfNConfig {
                candidates: 3,
                per_candidate_timeout: Duration::from_secs(60),
                ..Default::default()
            },
            suite,
            Duration::from_secs(60),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn selects_the_applicable_patch_and_exports_it_byte_identical() {
        let good = add_file_patch("good.txt", "hello");
        let result = run(
            vec![good.clone(), "not a diff at all".into()],
            fast_pass_command(),
            None,
        )
        .await;

        assert_eq!(result.selection.selected, Some(0), "{:?}", result.selection.selection_reason);
        let exported = std::env::temp_dir().join(format!("winner-{}.patch", Uuid::new_v4()));
        let bytes = result.export_winner(&exported).unwrap();
        assert_eq!(bytes, good.len());
        assert_eq!(std::fs::read_to_string(&exported).unwrap(), good);
        let _ = std::fs::remove_file(&exported);
    }

    #[tokio::test]
    async fn all_candidates_failing_to_apply_selects_nothing() {
        let result = run(
            vec!["garbage".into(), "".into()],
            fast_pass_command(),
            None,
        )
        .await;
        assert_eq!(result.selection.selected, None);
        assert!(result.selection.selection_reason.contains("refusing"));
        // The refusals name the failing stage.
        let first = &result.selection.outcomes[0].evidence.checks;
        assert!(first.iter().any(|(n, ok, _)| n == "patch_applied" && !ok));
    }

    #[tokio::test]
    async fn a_passing_apply_with_a_failing_suite_is_not_selected() {
        let result = run(
            vec![add_file_patch("x.txt", "x")],
            fast_fail_command(),
            None,
        )
        .await;
        assert_eq!(result.selection.selected, None);
        let checks = &result.selection.outcomes[0].evidence.checks;
        assert!(checks.iter().any(|(n, ok, _)| n == "patch_applied" && *ok));
        assert!(checks.iter().any(|(n, ok, _)| n == "test_suite" && !ok));
    }

    #[tokio::test]
    async fn the_run_writes_a_self_contained_ledger_chain_including_export() {
        use sha2::{Digest, Sha256};
        let (ledger, session) = (Arc::new(MemoryLedgerStore::new()), Uuid::new_v4());
        let result = run(
            vec![add_file_patch("y.txt", "y")],
            fast_pass_command(),
            Some((Arc::clone(&ledger), session)),
        )
        .await;

        let exported = std::env::temp_dir().join(format!("export-{}.patch", Uuid::new_v4()));
        let bytes = result.export_winner(&exported).unwrap();
        record_patch_export(
            &(Arc::clone(&ledger) as Arc<dyn crate::ledger::LedgerStore>),
            session,
            result.selection.selected.unwrap(),
            &exported,
            bytes,
        )
        .await
        .unwrap();
        let _ = std::fs::remove_file(&exported);

        let entries = ledger.get_entries(session).await.unwrap();
        assert!(!entries.is_empty());
        let mut prev = String::new();
        for e in &entries {
            assert_eq!(e.prev_hash, prev, "chain must be intact through export");
            prev = format!(
                "{:x}",
                Sha256::digest(format!("{}:{}", e.prev_hash, e.action_id).as_bytes())
            );
        }
        assert!(entries.iter().any(|e| e.action_id == "best_of_n.patch_export"));
    }

    #[tokio::test]
    async fn the_model_source_treats_its_response_as_the_candidate() {
        let diff = add_file_patch("m.txt", "from model");
        let client: Arc<dyn ModelClient> = Arc::new(crate::agent::TestModelClient::new(vec![
            diff.clone(),
            String::new(), // an empty generation is a candidate that fails later
        ]));
        let patches = ModelPatchSource { client, context: None }
            .generate("add m.txt", 2)
            .await
            .unwrap();
        assert_eq!(patches.len(), 2);
        assert_eq!(patches[0], diff);
        assert_eq!(patches[1], "");
    }

    #[tokio::test]
    async fn the_working_tree_source_returns_exactly_one_diff_candidate() {
        // The shared base repo is clean, so the diff is empty — and an empty
        // patch is a candidate that fails to apply. That is the honest
        // "nothing to verify" outcome, not a fabricated pass.
        let patches = WorkingTreeSource {
            repo_root: shared_git_repo().clone(),
        }
        .generate("t", 1)
        .await
        .unwrap();
        assert_eq!(patches.len(), 1, "exactly one working-tree candidate");
    }
}
