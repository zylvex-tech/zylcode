//! Git source-control state API.
//!
//! Same contract as [`super::api`]: one function, three surfaces (CLI
//! `serve-intel` HTTP route, Tauri `git_status` command, direct callers).
//! Every field is read from real git — `status --porcelain=v1 -b`, the
//! merge-base ahead/behind count, and `diff HEAD --stat`. Nothing here
//! fabricates a clean tree to look tidy: an honest dirty tree is the point.

use anyhow::Result;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

/// Maximum diff-stat rows returned.
pub const MAX_DIFF_ROWS: usize = 50;

struct GitOutput {
    stdout: String,
    stderr: String,
    success: bool,
}

fn git(root: &Path, args: &[&str]) -> GitOutput {
    let out = Command::new("git").args(args).current_dir(root).output();
    match out {
        Ok(out) => GitOutput {
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            success: out.status.success(),
        },
        Err(e) => GitOutput {
            stdout: String::new(),
            stderr: format!("failed to spawn git: {e}"),
            success: false,
        },
    }
}

/// One entry of `git status --porcelain=v1`: `XY <path>` (renames carry the
/// original path after `->`, which we keep verbatim for honesty).
struct StatusEntry {
    x: char,
    y: char,
    path: String,
}

fn parse_porcelain(output: &str) -> Vec<StatusEntry> {
    output
        .lines()
        .filter(|l| l.len() >= 3)
        .map(|line| StatusEntry {
            x: line.as_bytes()[0] as char,
            y: line.as_bytes()[1] as char,
            path: line[3..].trim_end().to_string(),
        })
        .collect()
}

/// Build the source-control payload for the repository at `root`.
pub fn git_status_payload(root: &Path) -> Result<Value> {
    anyhow::ensure!(
        root.is_dir(),
        "repository root '{}' is not a directory",
        root.display()
    );

    // 1. Branch + tracking + porcelain status in one call.
    let status = git(root, &["status", "--porcelain=v1", "-b"]);
    anyhow::ensure!(
        status.success,
        "git status failed: {}",
        status.stderr.trim()
    );

    let mut branch = String::from("(detached)");
    let mut upstream: Option<String> = None;
    let mut ahead: i64 = 0;
    let mut behind: i64 = 0;
    let mut entries: Vec<StatusEntry> = Vec::new();

    for line in status.stdout.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            // `## branch...origin/branch [ahead 1, behind 2]` or `## No commits yet on main`
            let mut rest = rest.to_string();
            if let Some(start) = rest.find('[') {
                let meta = rest[start + 1..].trim_end_matches(']').to_string();
                for part in meta.split(',') {
                    let part = part.trim();
                    if let Some(n) = part.strip_prefix("ahead ") {
                        ahead = n.parse().unwrap_or(0);
                    } else if let Some(n) = part.strip_prefix("behind ") {
                        behind = n.parse().unwrap_or(0);
                    }
                }
                rest = rest[..start].trim_end().to_string();
            }
            if let Some((b, u)) = rest.split_once("...") {
                branch = b.trim().to_string();
                upstream = Some(u.trim().to_string());
            } else {
                branch = rest.trim().to_string();
            }
        } else if line.trim().is_empty() {
            continue;
        } else {
            if let Some(entry) = parse_porcelain(line).into_iter().next() {
                entries.push(entry);
            }
        }
    }

    // 2. Ahead/behind via merge-base when an upstream exists (the `[ahead]`
    //    shorthand is only populated in some states).
    if upstream.is_some() {
        let ab = git(
            root,
            &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
        );
        if ab.success {
            let mut parts = ab.stdout.split_whitespace();
            if let (Some(b), Some(a)) = (parts.next(), parts.next()) {
                behind = b.parse().unwrap_or(behind);
                ahead = a.parse().unwrap_or(ahead);
            }
        }
    }

    // 3. Uncommitted diff stat (staged + unstaged, vs HEAD).
    let diffstat = git(root, &["diff", "HEAD", "--stat"]);
    let mut diff_rows: Vec<Value> = Vec::new();
    let mut files_changed = 0usize;
    let mut insertions = 0i64;
    let mut deletions = 0i64;
    if diffstat.success {
        for line in diffstat.stdout.lines() {
            // Final line: ` 4 files changed, 125 insertions(+), 51 deletions(-)`
            if line.contains("files changed") || line.contains("file changed") {
                for part in line.split(',') {
                    let part = part.trim();
                    if let Some(n) = part.strip_suffix(" files changed") {
                        files_changed = n.trim().parse().unwrap_or(0);
                    } else if let Some(n) = part.strip_suffix(" file changed") {
                        files_changed = n.trim().parse().unwrap_or(0);
                    } else if let Some(n) = part.strip_suffix(" insertions(+)") {
                        insertions = n.trim().parse().unwrap_or(0);
                    } else if let Some(n) = part.strip_suffix(" deletions(-)") {
                        deletions = n.trim().parse().unwrap_or(0);
                    }
                }
            } else if let Some((path, nums)) = line.rsplit_once('|') {
                let added = nums.matches('+').count() as i64;
                let removed = nums.matches('-').count() as i64;
                diff_rows.push(json!({
                    "path": path.trim(),
                    "added": added,
                    "removed": removed,
                }));
            }
        }
    }
    diff_rows.truncate(MAX_DIFF_ROWS);

    let staged = entries
        .iter()
        .filter(|e| e.x != ' ' && e.x != '?')
        .count();
    let unstaged = entries
        .iter()
        .filter(|e| e.y != ' ' && e.y != '?')
        .count();
    let untracked = entries.iter().filter(|e| e.x == '?' || e.y == '?').count();

    let head = git(root, &["rev-parse", "--short", "HEAD"]);
    let head_short = if head.success {
        head.stdout.trim().to_string()
    } else {
        String::new()
    };

    Ok(json!({
        "branch": branch,
        "upstream": upstream,
        "ahead": ahead,
        "behind": behind,
        "head": head_short,
        "clean": entries.is_empty(),
        "counts": {
            "total": entries.len(),
            "staged": staged,
            "unstaged": unstaged,
            "untracked": untracked,
        },
        "entries": entries
            .iter()
            .take(MAX_DIFF_ROWS)
            .map(|e| {
                json!({
                    "status": format!("{}{}", e.x, e.y),
                    "path": e.path,
                })
            })
            .collect::<Vec<_>>(),
        "diffstat": {
            "files_changed": files_changed,
            "insertions": insertions,
            "deletions": deletions,
            "rows": diff_rows,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git_ok(root: &Path, args: &[&str]) {
        let out = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn sample_repo() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        git_ok(&root, &["init", "-q"]);
        git_ok(&root, &["config", "user.email", "t@example.com"]);
        git_ok(&root, &["config", "user.name", "t"]);
        std::fs::write(root.join("README.md"), "# t\n").unwrap();
        git_ok(&root, &["add", "."]);
        git_ok(&root, &["commit", "-q", "-m", "base"]);
        (dir, root)
    }

    #[test]
    fn a_clean_repo_reports_clean_with_the_real_branch() {
        let (_dir, root) = sample_repo();
        let payload = git_status_payload(&root).unwrap();
        assert_eq!(payload["clean"], true);
        // Whatever init.defaultBranch is on this machine — the payload must
        // report the repo's real branch, not an assumed name.
        let actual = {
            let out = Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&root)
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        assert_eq!(payload["branch"], actual);
        assert_eq!(payload["counts"]["total"], 0);
        assert!(payload["head"].as_str().unwrap().len() >= 7);
    }

    #[test]
    fn a_dirty_repo_reports_the_actual_changes() {
        let (_dir, root) = sample_repo();
        std::fs::write(root.join("README.md"), "# changed\n").unwrap(); // unstaged M
        std::fs::write(root.join("new.txt"), "hi\n").unwrap(); // untracked
        std::fs::write(root.join("staged.txt"), "x\n").unwrap();
        git_ok(&root, &["add", "staged.txt"]); // staged A

        let payload = git_status_payload(&root).unwrap();
        assert_eq!(payload["clean"], false);
        assert_eq!(payload["counts"]["total"], 3);
        assert_eq!(payload["counts"]["staged"], 1);
        assert_eq!(payload["counts"]["untracked"], 1);

        let entries = payload["entries"].as_array().unwrap();
        assert!(entries
            .iter()
            .any(|e| e["path"] == "staged.txt" && e["status"].as_str().unwrap().starts_with('A')));
        assert!(entries.iter().any(|e| e["path"] == "README.md"));

        // The diffstat reflects the real uncommitted change to README.md.
        let diffstat = &payload["diffstat"];
        assert!(
            diffstat["insertions"].as_i64().unwrap() >= 1,
            "real insertion expected: {diffstat:?}"
        );
        assert!(diffstat["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["path"] == "README.md"));
    }

    #[test]
    fn a_stale_upstream_does_not_break_the_payload() {
        // No upstream configured: ahead/behind stay 0 and upstream is null.
        let (_dir, root) = sample_repo();
        let payload = git_status_payload(&root).unwrap();
        assert!(payload["upstream"].is_null());
        assert_eq!(payload["ahead"], 0);
        assert_eq!(payload["behind"], 0);
    }

    #[test]
    fn a_non_repository_fails_with_the_git_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = git_status_payload(dir.path());
        assert!(err.is_err(), "a non-repo must fail, not fabricate a payload");
    }
}
