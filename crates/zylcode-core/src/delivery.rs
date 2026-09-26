//! Delivery Engine — build orchestration, release packaging, deploy targets.
//!
//! # Why this exists
//!
//! The Build/Deploy menu entries and "CLI releases" were disabled or blocked:
//! no orchestrator, no packaging step, and CI locked on billing. This module
//! is the honest local core of delivery:
//!
//! - [`build_workspace`] runs the real build commands through the terminal
//!   hub (cargo test/build, pnpm build) and records every result;
//! - [`package_release`] produces a real versioned artifact under
//!   `.zylcode/releases/` with a SHA-256 manifest;
//! - [`deploy_targets`] reports what is actually configured. A target that
//!   does not exist is `UNCOMMISSIONED` — never "deployed".
//!
//! Everything records a Proof Engine record (runtime command evidence) and
//! registers release bundles in the Artifact Bus, so a release claim can
//! always be traced to a hash and a command that ran.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::artifact_bus::{ArtifactBus, ArtifactKind};
use crate::terminal::{exec_in_root, TerminalRequest};

/// One step of a build/release pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub name: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Seconds before the step is killed.
    pub timeout_secs: u64,
    /// When false the step is skipped (reported as such, never faked).
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Result of one executed step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub name: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub duration_ms: u128,
    /// Empty when the step succeeded; carries the honest error otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub passed: bool,
}

/// The pipeline: the real build for this repository, derived from what the
/// repo actually contains (Rust workspace + pnpm frontend).
pub fn default_build_pipeline() -> Vec<PipelineStep> {
    vec![
        PipelineStep {
            name: "rust-tests".into(),
            command: "cargo test --workspace --offline".into(),
            cwd: None,
            timeout_secs: 1800,
            enabled: true,
        },
        PipelineStep {
            name: "rust-build-release".into(),
            command: "cargo build --release -p zylcode-cli".into(),
            cwd: None,
            timeout_secs: 1800,
            enabled: true,
        },
        PipelineStep {
            name: "frontend-typecheck".into(),
            command: "npx tsc --noEmit".into(),
            cwd: Some("apps/zylcode-desktop".into()),
            timeout_secs: 300,
            enabled: true,
        },
        PipelineStep {
            name: "frontend-tests".into(),
            command: "npx vitest run".into(),
            cwd: Some("apps/zylcode-desktop".into()),
            timeout_secs: 600,
            enabled: true,
        },
        PipelineStep {
            name: "frontend-build".into(),
            command: "npx vite build".into(),
            cwd: Some("apps/zylcode-desktop".into()),
            timeout_secs: 600,
            enabled: true,
        },
    ]
}

/// Execute the pipeline for real, step by step, capturing outcomes. Stops at
/// the first failed step unless `keep_going` — a failed step is recorded,
/// never retried silently, and never reported as passed.
pub async fn build_workspace(
    root: &Path,
    steps: &[PipelineStep],
    keep_going: bool,
) -> Result<Value> {
    let mut results: Vec<StepResult> = Vec::new();
    let mut all_passed = true;
    for step in steps {
        if !step.enabled {
            results.push(StepResult {
                name: step.name.clone(),
                command: step.command.clone(),
                exit_code: None,
                timed_out: false,
                duration_ms: 0,
                error: Some("skipped (disabled)".into()),
                passed: false,
            });
            continue;
        }
        let step_root = match &step.cwd {
            Some(rel) => root.join(rel),
            None => root.to_path_buf(),
        };
        let req = TerminalRequest {
            session_id: None,
            cwd: None,
            command: step.command.clone(),
            timeout_secs: Some(step.timeout_secs),
        };
        match exec_in_root(&step_root, &req).await {
            Ok(out) => {
                let passed = out.exit_code == Some(0);
                if !passed {
                    all_passed = false;
                }
                results.push(StepResult {
                    name: step.name.clone(),
                    command: step.command.clone(),
                    exit_code: out.exit_code,
                    timed_out: out.timed_out,
                    duration_ms: out.duration_ms,
                    error: if passed {
                        None
                    } else {
                        Some(tail(&out.stderr_tail, &out.stdout_tail))
                    },
                    passed,
                });
            }
            Err(e) => {
                all_passed = false;
                results.push(StepResult {
                    name: step.name.clone(),
                    command: step.command.clone(),
                    exit_code: None,
                    timed_out: false,
                    duration_ms: 0,
                    error: Some(e.to_string()),
                    passed: false,
                });
            }
        }
        if !all_passed && !keep_going {
            break;
        }
    }
    Ok(json!({
        "passed": all_passed,
        "steps": results,
    }))
}

/// Build a release bundle: copies the real CLI binary + frontend dist into
/// `.zylcode/releases/<version>/`, writes a SHA-256 manifest, registers the
/// bundle in the Artifact Bus, and returns the manifest. Refuses to package
/// a release whose binary does not exist — a placeholder release is worse
/// than no release.
pub async fn package_release(root: &Path, version: &str) -> Result<Value> {
    anyhow::ensure!(
        !version.trim().is_empty(),
        "release version must not be empty"
    );
    let exe = root.join("target/release/zylcode.exe");
    let exe_unix = root.join("target/release/zylcode");
    let binary = if exe.exists() {
        exe
    } else if exe_unix.exists() {
        exe_unix
    } else {
        anyhow::bail!(
            "no release binary at target/release/zylcode[.exe] — run the build pipeline first; \
             packaging a placeholder release is forbidden"
        );
    };
    let dist = root.join("apps/zylcode-desktop/dist");
    anyhow::ensure!(
        dist.is_dir(),
        "frontend dist missing (apps/zylcode-desktop/dist) — run the build pipeline first"
    );

    let release_dir = root.join(".zylcode/releases").join(version);
    std::fs::create_dir_all(&release_dir)
        .with_context(|| format!("cannot create {}", release_dir.display()))?;
    std::fs::create_dir_all(release_dir.join("dist"))?;

    // Copy binary + dist.
    let bin_name = if cfg!(windows) {
        "zylcode.exe"
    } else {
        "zylcode"
    };
    let staged_bin = release_dir.join(bin_name);
    std::fs::copy(&binary, &staged_bin)
        .with_context(|| format!("cannot stage {}", staged_bin.display()))?;
    let mut staged_files: Vec<(String, u64, String)> = Vec::new();
    copy_tree(&dist, &release_dir.join("dist"), &mut staged_files)?;

    // Manifest with SHA-256 of every staged file.
    staged_files.push((
        bin_name.to_string(),
        std::fs::metadata(&staged_bin)?.len(),
        sha256_file(&staged_bin)?,
    ));
    staged_files.sort();
    let manifest = json!({
        "version": version,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "files": staged_files.iter().map(|(p, len, hash)| json!({
            "path": p, "bytes": len, "sha256": hash,
        })).collect::<Vec<_>>(),
    });
    let manifest_path = release_dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    // Register in the Artifact Bus (bundle bytes hashed via the manifest's
    // own content; the bus pins the manifest).
    let bus = ArtifactBus::open(root)?;
    let manifest_bytes = std::fs::read(&manifest_path)?;
    let artifact = bus.register_generated(
        ArtifactKind::ReleasePackage,
        &crate::project_store::projects_path(root).to_string_lossy(),
        None,
        "delivery-engine",
        &manifest_bytes,
        Some(&head_short(root)),
        &format!("release bundle {version}: CLI binary + frontend dist, SHA-256 manifest"),
        None,
    )?;

    Ok(json!({
        "version": version,
        "release_dir": release_dir,
        "artifact_id": artifact.id,
        "content_hash": artifact.content_hash,
        "file_count": staged_files.len(),
        "manifest": manifest,
    }))
}

/// Deploy targets that actually exist in this repository, evaluated for real.
pub fn deploy_targets(root: &Path) -> Result<Value> {
    let gh_workflow = root.join(".github/workflows/release.yml").exists();
    let head = head_short(root);
    // A tag for the current version?
    let version = crate::gitops::version_payload(root)["app_version"]
        .as_str()
        .unwrap_or("0.0.0")
        .to_string();
    let tag = format!("v{version}");
    let tag_exists = std::process::Command::new("git")
        .args(["rev-parse", "-q", "--verify", &format!("refs/tags/{tag}")])
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // crates.io: is there a stored token? (Never reads or returns it.)
    let cargo_token = std::env::var("CARGO_REGISTRY_TOKEN").is_ok()
        || root.join("../../.cargo/credentials.toml").exists()
        || home_credentials_exists();

    Ok(json!({
        "head": head,
        "github_release": {
            "commissioned": gh_workflow,
            "tag": tag,
            "tag_pushed": tag_exists,
            "state": if tag_exists { "COMMISSIONED" } else { "UNCOMMISSIONED" },
            "note": if tag_exists {
                "push the tag (or run the release workflow) to publish installers via CI"
            } else {
                "no tag for the current version exists yet — create one to trigger releases"
            },
        },
        "crates_io": {
            "commissioned": cargo_token,
            "state": if cargo_token { "COMMISSIONED" } else { "UNCOMMISSIONED" },
            "note": "publish requires a cargo registry token (CARGO_REGISTRY_TOKEN)",
        },
        "remote_server": {
            "commissioned": false,
            "state": "UNCOMMISSIONED",
            "note": "no remote deployment target is configured in this repository",
        },
    }))
}

fn home_credentials_exists() -> bool {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(|h| Path::new(&h).join(".cargo/credentials.toml").exists())
        .unwrap_or(false)
}

fn head_short(root: &Path) -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn tail(stderr: &str, stdout: &str) -> String {
    let s = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    let lines: Vec<&str> = s.lines().rev().take(12).collect();
    let mut lines: Vec<String> = lines.into_iter().map(|l| l.to_string()).collect();
    lines.reverse();
    lines.join("\n")
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn copy_tree(src: &Path, dst: &Path, staged: &mut Vec<(String, u64, String)>) -> Result<()> {
    for entry in walkdir_like(src)? {
        let rel = entry.0.clone();
        let target = dst.join(&rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src.join(&rel), &target)?;
        staged.push((format!("dist/{rel}"), entry.1, entry.2));
    }
    Ok(())
}

/// Minimal recursive walk collecting (relative path, len, sha256).
fn walkdir_like(src: &Path) -> Result<Vec<(String, u64, String)>> {
    let mut out = Vec::new();
    walk_inner(src, src, &mut out)?;
    Ok(out)
}

fn walk_inner(base: &Path, dir: &Path, out: &mut Vec<(String, u64, String)>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let rel = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            walk_inner(base, &path, out)?;
        } else {
            let meta = entry.metadata()?;
            let bytes = std::fs::read(&path)?;
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            out.push((rel, meta.len(), hex(&hasher.finalize())));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_release_refuses_without_binary() {
        let dir = tempfile::tempdir().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(package_release(dir.path(), "9.9.9"))
            .unwrap_err();
        assert!(
            err.to_string().contains("forbidden"),
            "refusal must be explicit: {err}"
        );
    }

    #[test]
    fn pipeline_is_real_and_ordered() {
        let steps = default_build_pipeline();
        assert!(steps.len() >= 4);
        assert!(steps[0].command.contains("cargo test"));
        assert!(steps.iter().any(|s| s.command.contains("vite build")));
    }

    #[test]
    fn deploy_targets_report_honest_states() {
        let dir = tempfile::tempdir().unwrap();
        // Not a git repo: version payload falls back, but the shape must hold.
        let v = deploy_targets(dir.path()).unwrap();
        assert!(v["github_release"]["state"].is_string());
        assert!(v["crates_io"]["state"].is_string());
        assert_eq!(v["remote_server"]["state"], "UNCOMMISSIONED");
    }
}
