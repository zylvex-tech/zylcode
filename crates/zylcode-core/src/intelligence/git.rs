//! Git / Change Intelligence.
//!
//! Captures current HEAD, branch, dirty state, recent commits,
//! and files changed by commit. Uses git CLI (shell-out).

use crate::intelligence::types::{ChangeType, GitChange, GitCommit, GitIdentity};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::path::Path;

/// Get the current git identity of a repository.
pub fn get_git_identity(root: &Path) -> Result<GitIdentity> {
    let head = run_git(root, &["rev-parse", "HEAD"])?;
    let branch = run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"]).ok();
    let dirty = !run_git(root, &["status", "--porcelain"])?.trim().is_empty();
    let remote_url = run_git(root, &["remote", "get-url", "origin"]).ok();

    Ok(GitIdentity {
        head: head.trim().to_string(),
        branch: branch.map(|b| b.trim().to_string()),
        dirty,
        remote_url: remote_url.map(|u| u.trim().to_string()),
    })
}

/// Get recent commits (last N).
pub fn get_recent_commits(root: &Path, count: usize) -> Result<Vec<GitCommit>> {
    let format = "%H|%h|%s|%an|%aI";
    let output = run_git(
        root,
        &[
            "log",
            &format!("--format={}", format),
            &format!("-{}", count),
        ],
    )?;

    let mut commits = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(5, '|').collect();
        if parts.len() < 5 {
            continue;
        }

        let timestamp = DateTime::parse_from_rfc3339(parts[4])
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        commits.push(GitCommit {
            id: parts[0].to_string(),
            short_id: parts[1].to_string(),
            message: parts[2].to_string(),
            author: parts[3].to_string(),
            timestamp,
            files_changed: Vec::new(), // Filled by get_commit_files
        });
    }

    Ok(commits)
}

/// Get files changed by a specific commit.
pub fn get_commit_files(root: &Path, commit_id: &str) -> Result<Vec<String>> {
    // Try diff-tree first (works for non-root commits)
    let output = run_git(
        root,
        &[
            "diff-tree",
            "--no-commit-id",
            "-r",
            "--name-only",
            commit_id,
        ],
    );

    match output {
        Ok(output) if !output.trim().is_empty() => Ok(output
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()),
        _ => {
            // Fallback for root commit: use git show --name-only
            let output = run_git(root, &["show", "--name-only", "--format=", commit_id])?;
            Ok(output
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect())
        }
    }
}

/// Get recent commits with their changed files.
pub fn get_recent_commits_with_files(root: &Path, count: usize) -> Result<Vec<GitCommit>> {
    let mut commits = get_recent_commits(root, count)?;

    for commit in &mut commits {
        commit.files_changed = get_commit_files(root, &commit.id).unwrap_or_default();
    }

    Ok(commits)
}

/// Parse one `git log --name-status` record line into changed file paths.
///
/// Statuses `A`/`M`/`D`/`T` carry a single path; `R`/`C` (rename, copy) may
/// carry a similarity score (`R100`) and carry two paths. A rename
/// contributes both endpoints: the coupling moves, it is not severed.
fn commit_file_paths_from_name_status(line: &str) -> Vec<String> {
    let mut parts = line.split('\t');
    let status = parts.next().unwrap_or("");
    let first = parts.next().unwrap_or("");
    if first.is_empty() {
        return Vec::new();
    }
    match status.chars().next() {
        Some('R') | Some('C') => {
            let second = parts.next().unwrap_or("");
            let mut out = vec![first.to_string()];
            if !second.is_empty() {
                out.push(second.to_string());
            }
            out
        }
        _ => vec![first.to_string()],
    }
}

/// Mine co-change evidence from the repository's **full** commit history.
///
/// The 40-commit window previously used here made coupling assertions
/// HEAD-sensitive: a real, tested coupling slid out of the window as commits
/// accumulated, silently changing retrieval behaviour on every push.
/// Co-change evidence is structural — the coupling exists whether it is
/// recent or not — so it is mined from the complete history. Recency
/// semantics stay pinned to the 20 newest commits upstream (see
/// `ContextSignals::new`).
///
/// One pass over `git log --name-status` replaces the per-commit
/// `diff-tree` fan-out, so the full history costs about one subprocess
/// invocation instead of one per commit.
pub fn get_full_history_commits(root: &Path) -> Result<Vec<GitCommit>> {
    const SEP: &str = "\u{1f}ZYLCOMMIT\u{1f}";
    let output = run_git(
        root,
        &[
            "log",
            &format!("--format={}", SEP),
            "--name-status",
        ],
    )?;

    let mut commits: Vec<GitCommit> = Vec::new();
    let mut current: Option<GitCommit> = None;
    for line in output.lines() {
        if let Some(meta) = line.strip_prefix(SEP) {
            if let Some(done) = current.take() {
                commits.push(done);
            }
            let parts: Vec<&str> = meta.splitn(5, '|').collect();
            if parts.len() < 5 {
                current = None;
                continue;
            }
            let timestamp = DateTime::parse_from_rfc3339(parts[4])
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            current = Some(GitCommit {
                id: parts[0].to_string(),
                short_id: parts[1].to_string(),
                message: parts[2].to_string(),
                author: parts[3].to_string(),
                timestamp,
                files_changed: Vec::new(),
            });
        } else if line.trim().is_empty() {
            continue;
        } else if let Some(commit) = current.as_mut() {
            commit
                .files_changed
                .extend(commit_file_paths_from_name_status(line));
        }
    }
    if let Some(done) = current.take() {
        commits.push(done);
    }

    Ok(commits)
}

/// Get changes affecting a specific file (recent N commits).
pub fn get_file_changes(root: &Path, file_path: &str, count: usize) -> Result<Vec<GitChange>> {
    let output = run_git(
        root,
        &[
            "log",
            &format!("-{}", count),
            "--format=%H",
            "--follow",
            "--",
            file_path,
        ],
    )?;

    let mut changes = Vec::new();
    for commit_id in output.lines() {
        let commit_id = commit_id.trim();
        if commit_id.is_empty() {
            continue;
        }

        // Get the diff stat for this commit
        let stat = run_git(
            root,
            &[
                "diff",
                &format!("{}^..{}", commit_id, commit_id),
                "--stat",
                "--",
                file_path,
            ],
        )
        .unwrap_or_default();

        let (lines_added, lines_removed) = parse_diff_stat(&stat);

        // Determine change type
        let change_type = if stat.contains("=>") {
            ChangeType::Modified // Simplified — could detect renames
        } else if lines_added > 0 && lines_removed == 0 {
            ChangeType::Added
        } else if lines_removed > 0 && lines_added == 0 {
            ChangeType::Deleted
        } else {
            ChangeType::Modified
        };

        changes.push(GitChange {
            commit: commit_id.to_string(),
            file: file_path.to_string(),
            change_type,
            lines_added,
            lines_removed,
        });
    }

    Ok(changes)
}

/// Get the current diff (uncommitted changes).
pub fn get_current_diff(root: &Path) -> Result<String> {
    run_git(root, &["diff"])
}

/// Get staged files.
pub fn get_staged_files(root: &Path) -> Result<Vec<String>> {
    let output = run_git(root, &["diff", "--cached", "--name-only"])?;
    Ok(output
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// Get untracked files.
pub fn get_untracked_files(root: &Path) -> Result<Vec<String>> {
    let output = run_git(root, &["ls-files", "--others", "--exclude-standard"])?;
    Ok(output
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// Parse diff stat line to extract added/removed lines.
fn parse_diff_stat(stat: &str) -> (u32, u32) {
    let mut added = 0u32;
    let mut removed = 0u32;

    for line in stat.lines() {
        if let Some(insertions) = line.find("insertion") {
            let before = &line[..insertions];
            if let Some(num) = before.split_whitespace().last() {
                added = num.parse().unwrap_or(0);
            }
        }
        if let Some(deletions) = line.find("deletion") {
            let before = &line[..deletions];
            if let Some(num) = before.split_whitespace().last() {
                removed = num.parse().unwrap_or(0);
            }
        }
    }

    (added, removed)
}

/// Run a git command and return stdout.
fn run_git(root: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .context("Failed to run git")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn init_git_repo(dir: &Path) -> bool {
        let result = std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output();
        match result {
            Ok(output) if output.status.success() => {
                std::process::Command::new("git")
                    .args(["config", "user.email", "test@test.com"])
                    .current_dir(dir)
                    .output()
                    .unwrap();
                std::process::Command::new("git")
                    .args(["config", "user.name", "Test"])
                    .current_dir(dir)
                    .output()
                    .unwrap();
                true
            }
            _ => false,
        }
    }

    fn make_commit(dir: &Path, file: &str, content: &str, message: &str) {
        let path = dir.join(file);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        std::process::Command::new("git")
            .args(["add", file])
            .current_dir(dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn get_identity() {
        let dir = TempDir::new().unwrap();
        if !init_git_repo(dir.path()) {
            return; // Git not available
        }
        make_commit(dir.path(), "README.md", "# Test", "Initial commit");

        let identity = get_git_identity(dir.path()).unwrap();
        assert!(!identity.head.is_empty());
        assert!(identity.branch.is_some());
        assert!(!identity.dirty);
    }

    #[test]
    fn get_recent_commits_test() {
        let dir = TempDir::new().unwrap();
        if !init_git_repo(dir.path()) {
            return; // Git not available
        }
        make_commit(dir.path(), "a.txt", "a", "First commit");
        make_commit(dir.path(), "b.txt", "b", "Second commit");

        let commits = get_recent_commits(dir.path(), 10).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "Second commit");
        assert_eq!(commits[1].message, "First commit");
    }

    #[test]
    fn get_commit_files_test() {
        let dir = TempDir::new().unwrap();
        if !init_git_repo(dir.path()) {
            println!("Git not available, skipping test");
            return; // Git not available
        }
        make_commit(dir.path(), "src/main.rs", "fn main() {}", "Add main");

        let commits = get_recent_commits(dir.path(), 1).unwrap();
        if commits.is_empty() {
            return;
        }
        let files = get_commit_files(dir.path(), &commits[0].id).unwrap();
        // On Windows, git might return backslashes
        let has_main = files.iter().any(|f| f.contains("main.rs"));
        assert!(
            has_main,
            "Should find main.rs in changed files, got: {:?}",
            files
        );
    }

    #[test]
    fn dirty_state_detection() {
        let dir = TempDir::new().unwrap();
        if !init_git_repo(dir.path()) {
            return; // Git not available
        }
        make_commit(dir.path(), "README.md", "# Test", "Initial");

        // Make a dirty change
        fs::write(dir.path().join("new_file.txt"), "dirty").unwrap();

        let identity = get_git_identity(dir.path()).unwrap();
        assert!(identity.dirty);
    }
}
