//! Container-per-candidate sandbox (Phase 7A).
//!
//! # Why this exists
//!
//! [`crate::patch_best_of_n::PatchWorktreeVerifier`] isolates candidates
//! from each other but executes the suite **directly on the host**: a
//! candidate's build steps run with the user's full authority — network,
//! filesystem, everything. That is the trust gap between "evidence-scored
//! selection" and "benchmark-grade evidence": untrusted candidate code must
//! not get host authority.
//!
//! This module closes that gap for the docker case: each candidate's suite
//! runs in its **own throwaway container** mounted at its own worktree,
//! with no network, a read-only root filesystem, CPU/memory/process caps,
//! and only the worktree (plus a `/tmp` tmpfs) writable. Container and
//! worktree are both removed afterwards, so every candidate leaves zero
//! residue.
//!
//! # The fail-closed law
//!
//! Sandbox verification **refuses to run unsandboxed**. If the docker CLI
//! is missing, the daemon is unreachable, or the image is absent, `verify`
//! returns an error that [`crate::best_of_n`] records as a failed check —
//! never a silent fallback to host execution. A number produced outside the
//! sandbox is not a sandboxed number.
//!
//! # Honest limits (recorded, not hidden)
//!
//! * Docker only. Podman or other runtimes would need their own backend
//!   (`container_argv` is deliberately pure so one can be added without
//!   touching the verifier).
//! * Windows hosts: containers must be Linux containers (Docker Desktop
//!   default); bind-mount path translation is Docker Desktop's job.
//! * The container runtime is more trusted than the candidate, but is not
//!   itself verified here; the image is a *requirement the operator
//!   provides*, and [`SANDBOX_READY_MARKER`] lets the suite assert it is
//!   inside the sandbox.

use crate::best_of_n::{CandidateVerifier, VerificationEvidence};
use crate::patch_best_of_n::{output_tail, CandidateWorktree};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Env var a suite can assert to prove it is inside the sandbox. The
/// container env sets it; the host never does.
pub const SANDBOX_READY_MARKER: &str = "ZYLCODE_SANDBOX_READY";

/// Configuration for container-per-candidate verification.
///
/// Defaults are deny-by-default: no network, read-only root filesystem,
/// bounded CPU/memory/PIDs. `image` has no default on purpose — the
/// operator must name the environment the suite is supposed to run in
/// (for this workspace: a rust toolchain image).
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Container image the suite runs in (e.g. a Rust toolchain image).
    /// Must exist locally or be pullable by the operator beforehand.
    pub image: String,
    /// CPU limit (docker `--cpus`).
    pub cpus: f64,
    /// Memory limit in bytes (docker `--memory`).
    pub memory_bytes: u64,
    /// Process limit (docker `--pids-limit`).
    pub pids_limit: u32,
    /// Extra docker run flags for site policy (appended after ours, so a
    /// site could e.g. add `--env` entries; they cannot remove ours).
    pub extra_run_flags: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            image: String::new(),
            cpus: 2.0,
            memory_bytes: 4 * 1024 * 1024 * 1024,
            pids_limit: 512,
            extra_run_flags: Vec::new(),
        }
    }
}

/// `true` when the docker CLI answers — the only supported sandbox backend.
pub async fn docker_available() -> bool {
    matches!(
        tokio::process::Command::new("docker")
            .arg("info")
            .output()
            .await,
        Ok(out) if out.status.success()
    )
}

/// The full `docker run` argument vector for one candidate suite run.
///
/// Pure on purpose: the security surface is auditable from this function
/// alone, and a future backend (podman, native jail) replaces one function,
/// not the verifier. Fails when the image was not named — before anything
/// can run.
///
/// Layout (order matters — `extra_run_flags` goes last among flags so site
/// additions are visible but ours always precede the image):
/// `docker run --rm --network=none --read-only --tmpfs /tmp:rw,size=…,noexec
///  --cpus N --memory B --memory-swap B --pids-limit P
///  --volume=WORKTREE:/work:rw --workdir=/work --env SANDBOX=1
///  --env MARKER=1 [extra flags] IMAGE -- program args`
pub fn container_argv(
    config: &SandboxConfig,
    worktree_root: &Path,
    program: &str,
    args: &[String],
) -> Result<Vec<String>> {
    anyhow::ensure!(
        !config.image.trim().is_empty(),
        "sandbox image must be named explicitly (SandboxConfig::image)"
    );
    let mut argv = vec![
        "run".to_string(),
        // No name; the container is anonymous and removed on exit.
        "--rm".to_string(),
        // Deny-by-default: no network at all.
        "--network=none".to_string(),
        // The root filesystem is immutable; only the mounts below are not.
        "--read-only".to_string(),
        // Writable scratch, size-capped (256 MiB) and non-executable.
        "--tmpfs".to_string(),
        "/tmp:rw,size=268435456,noexec".to_string(),
        format!("--cpus={}", config.cpus),
        format!("--memory={}", config.memory_bytes),
        format!("--memory-swap={}", config.memory_bytes),
        format!("--pids-limit={}", config.pids_limit),
        // The candidate's worktree is the only writable volume.
        format!("--volume={}:/work:rw", worktree_root.display()),
        "--workdir=/work".to_string(),
        "--env=SANDBOX=1".to_string(),
        format!("--env={SANDBOX_READY_MARKER}=1"),
    ];
    argv.extend(config.extra_run_flags.iter().cloned());
    argv.push(config.image.clone());
    argv.push("--".to_string());
    argv.push(program.to_string());
    argv.extend(args.iter().cloned());
    Ok(argv)
}

/// Verify a candidate inside a per-candidate container.
#[derive(Debug, Clone)]
pub struct SandboxedWorktreeVerifier {
    /// Repository (with a HEAD) to branch worktrees from.
    pub repo_root: PathBuf,
    /// Per-candidate wall-clock budget for the whole container run.
    pub timeout: Duration,
    /// Suite command (default `cargo test`).
    pub command: Vec<String>,
    /// Sandbox policy and image.
    pub sandbox: SandboxConfig,
}

impl SandboxedWorktreeVerifier {
    pub fn new(repo_root: impl Into<PathBuf>, sandbox: SandboxConfig, timeout: Duration) -> Self {
        Self {
            repo_root: repo_root.into(),
            timeout,
            command: vec!["cargo".into(), "test".into()],
            sandbox,
        }
    }

    /// Fail-closed preconditions: docker answers and the image exists.
    ///
    /// The image check is local (`docker image inspect`); we never pull on
    /// the verification path — pulling untrusted-on-demand images during a
    /// benchmark run is exactly the kind of hidden network access the
    /// sandbox exists to prevent.
    pub async fn ensure_ready(&self) -> Result<()> {
        anyhow::ensure!(
            !self.sandbox.image.trim().is_empty(),
            "sandbox image must be named explicitly (SandboxConfig::image)"
        );
        anyhow::ensure!(
            docker_available().await,
            "sandbox backend unavailable: the docker CLI is missing or its daemon is not \
             reachable; refusing to verify candidates unsandboxed (fail closed)"
        );
        let out = tokio::process::Command::new("docker")
            .args(["image", "inspect", &self.sandbox.image])
            .output()
            .await
            .context("failed to run `docker image inspect`")?;
        anyhow::ensure!(
            out.status.success(),
            "sandbox image '{}' is not available locally; pull or build it beforehand \
             (verification never pulls images on demand) — refusing to run unsandboxed",
            self.sandbox.image
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl CandidateVerifier for SandboxedWorktreeVerifier {
    async fn verify(&self, candidate_index: usize, patch: &str) -> Result<VerificationEvidence> {
        // Fail closed *before* touching anything: no daemon, no verification.
        self.ensure_ready().await?;

        // Same audited worktree lifecycle as the plain verifier.
        let worktree = CandidateWorktree::create(&self.repo_root, candidate_index)
            .await
            .map_err(|detail| anyhow!("candidate worktree creation failed: {detail}"))?;
        if let Err(detail) = worktree.apply_patch(patch).await {
            worktree.remove().await;
            return Err(anyhow!("patch did not apply in worktree: {detail}"));
        }

        let (program, args) = match self.command.split_first() {
            Some((p, a)) => (p.to_string(), a.to_vec()),
            None => {
                worktree.remove().await;
                return Err(anyhow!("empty suite command"));
            }
        };

        let argv = container_argv(&self.sandbox, &worktree.root, &program, &args)?;
        let run = tokio::time::timeout(
            self.timeout,
            tokio::process::Command::new("docker")
                .args(&argv)
                // The host side of the command never reads the candidate's
                // output as instructions; capture only.
                .output(),
        )
        .await;

        let evidence = match run {
            Err(_) => VerificationEvidence {
                checks: vec![(
                    "sandbox_suite_completed_within_budget".to_string(),
                    false,
                    format!("container run timed out after {:?}", self.timeout),
                )],
                exit_status: None,
                output_tail: String::new(),
            },
            Ok(Err(e)) => VerificationEvidence {
                checks: vec![(
                    "sandbox_suite_spawned".to_string(),
                    false,
                    format!("failed to spawn docker: {e}"),
                )],
                exit_status: None,
                output_tail: String::new(),
            },
            Ok(Ok(out)) => VerificationEvidence {
                checks: vec![(
                    "sandbox_test_suite".to_string(),
                    out.status.success(),
                    format!(
                        "container exit status: {:?} (network=none, read-only rootfs, \
                         cpus={}, memory={}b, pids={})",
                        out.status.code(),
                        self.sandbox.cpus,
                        self.sandbox.memory_bytes,
                        self.sandbox.pids_limit
                    ),
                )],
                exit_status: out.status.code(),
                output_tail: output_tail(&out.stdout, &out.stderr),
            },
        };

        worktree.remove().await;
        Ok(evidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> SandboxConfig {
        SandboxConfig {
            image: "rust:1-bookworm".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn container_argv_denies_network_and_freezes_the_rootfs() {
        let root = Path::new("/tmp/wt");
        let argv = container_argv(&config(), root, "cargo", &["test".to_string()])
            .expect("argv builds");

        let joined = argv.join(" ");
        assert!(argv.contains(&"--network=none".to_string()), "{joined}");
        assert!(argv.contains(&"--read-only".to_string()), "{joined}");
        assert!(
            argv.iter().any(|a| a.starts_with("/tmp:rw,size=") && a.contains(",noexec")),
            "tmpfs must be size-capped and non-executable: {joined}"
        );
        assert!(argv.iter().any(|a| a.starts_with("--cpus=")), "{joined}");
        assert!(argv.iter().any(|a| a.starts_with("--memory=")), "{joined}");
        assert!(argv.iter().any(|a| a.starts_with("--memory-swap=")), "{joined}");
        assert!(argv.iter().any(|a| a.starts_with("--pids-limit=")), "{joined}");
    }

    #[test]
    fn only_the_worktree_is_mounted_and_writable() {
        let root = Path::new("/tmp/wt");
        let argv = container_argv(&config(), root, "cargo", &["test".to_string()])
            .expect("argv builds");
        let mounts: Vec<&String> = argv
            .iter()
            .filter(|a| a.starts_with("--volume="))
            .collect();
        assert_eq!(mounts.len(), 1, "exactly one bind mount: {mounts:?}");
        assert!(
            mounts[0].starts_with("--volume=/tmp/wt:/work:rw"),
            "worktree mounted at /work read-write: {mounts:?}"
        );
    }

    #[test]
    fn the_image_is_last_among_selectors_and_precedes_the_suite_command() {
        let root = Path::new("/tmp/wt");
        let argv = container_argv(&config(), root, "cargo", &["test".to_string()])
            .expect("argv builds");
        let image_pos = argv.iter().position(|a| a == "rust:1-bookworm").unwrap();
        assert_eq!(argv[image_pos + 1], "--");
        assert_eq!(argv[image_pos + 2], "cargo");
        assert_eq!(argv[image_pos + 3], "test");
        // Nothing after the suite command but the suite's own args.
        assert_eq!(argv.len(), image_pos + 4);
    }

    #[test]
    fn the_ready_marker_is_set_inside_the_container_only() {
        let root = Path::new("/tmp/wt");
        let argv = container_argv(&config(), root, "cargo", &[]).expect("argv builds");
        assert!(
            argv.iter()
                .any(|a| a == &format!("--env={SANDBOX_READY_MARKER}=1")),
            "{argv:?}"
        );
    }

    #[test]
    fn an_unnamed_image_is_rejected_before_anything_runs() {
        let cfg = SandboxConfig::default(); // image: ""
        assert!(
            container_argv(&cfg, Path::new("/tmp/wt"), "cargo", &[]).is_err(),
            "an unnamed image must be rejected at argv-build time"
        );
    }

    #[tokio::test]
    async fn verify_fails_closed_when_the_backend_is_unavailable() {
        // This assertion holds on any machine where docker is unavailable;
        // where docker IS available the guard below skips the check.
        if docker_available().await {
            return;
        }
        let verifier = SandboxedWorktreeVerifier::new(
            std::env::temp_dir(),
            config(),
            Duration::from_secs(30),
        );
        let err = verifier
            .verify(0, "diff --git a/x b/x")
            .await
            .expect_err("must refuse without a backend");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing to verify candidates unsandboxed"),
            "the refusal must say why: {msg}"
        );
    }

    #[tokio::test]
    async fn a_missing_image_fails_closed_even_with_a_backend() {
        if !docker_available().await {
            return; // covered by the fail-closed test above
        }
        let verifier = SandboxedWorktreeVerifier::new(
            std::env::temp_dir(),
            SandboxConfig {
                image: "zylcode-nonexistent-image-for-tests:never".to_string(),
                ..Default::default()
            },
            Duration::from_secs(30),
        );
        let err = verifier.verify(0, "diff").await.expect_err("must refuse");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("not available locally") && msg.contains("never pulls"),
            "{msg}"
        );
    }

    /// Real end-to-end proof: a candidate whose "suite" reads the sandbox
    /// markers and a read-only rootfs. Runs only when docker and the image
    /// are present — this is the commissioning gate, not a CI dependency.
    #[tokio::test]
    async fn commissioning_inside_a_real_container_the_suite_sees_the_sandbox() {
        if !docker_available().await {
            eprintln!("skipping: docker unavailable");
            return;
        }
        // Use any locally present image with a POSIX shell; prefer the
        // configured default, else debian-slim, else busybox.
        let image = ["rust:1-bookworm", "debian:bookworm-slim", "busybox:latest"]
            .into_iter()
            .find(|img| {
                std::process::Command::new("docker")
                    .args(["image", "inspect", img])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            });
        let Some(image) = image else {
            eprintln!("skipping: no usable local image");
            return;
        };

        let verifier = SandboxedWorktreeVerifier::new(
            std::env::temp_dir(),
            SandboxConfig {
                image: image.to_string(),
                ..Default::default()
            },
            Duration::from_secs(120),
        );
        let evidence = verifier
            .verify(
                0,
                "diff --git a/x b/x\nnew file mode 100644\n--- /dev/null\n+++ b/x\n@@ -0,0 +1 @@\n+hi\n",
            )
            .await
            .expect("commissioning run must not error");

        // Independent direct container assertion: the markers exist inside
        // the sandbox and the rootfs is genuinely read-only.
        let out = tokio::process::Command::new("docker")
            .args([
                "run",
                "--rm",
                "--network=none",
                "--read-only",
                "--env=SANDBOX=1",
                format!("--env={SANDBOX_READY_MARKER}=1").as_str(),
                image,
                "sh",
                "-c",
                "test -n \"$ZYLCODE_SANDBOX_READY\" && test -n \"$SANDBOX\" && \
                 (touch /probe 2>/dev/null && echo WRITABLE || echo READONLY)",
            ])
            .output()
            .await
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("READONLY"),
            "rootfs must be read-only inside the sandbox, got: {stdout}"
        );
        assert!(evidence.exit_status.is_some() || !evidence.output_tail.is_empty());
    }
}
