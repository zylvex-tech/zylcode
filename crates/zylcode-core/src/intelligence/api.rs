//! Repository Intelligence payload API.
//!
//! One function, two consumers: the CLI's `serve-intel` HTTP service (the
//! browser preview's data source) and the Tauri desktop `repo_context`
//! command. Both surfaces therefore show **the same** pipeline output —
//! the persisted-index-backed `RepoQuery` — rather than two divergent
//! reimplementations.
//!
//! Honesty rules: every count comes from a real index build (content-hash
//! validated); `results` are the retriever's actual ranked output with its
//! own provenance reasons; nothing here is synthesized to look busy.

use super::persisted::PersistedIndex;
use anyhow::Result;
use serde_json::{json, Value};
use std::path::Path;

/// Default task when a caller supplies an empty one.
pub const DEFAULT_TASK: &str = "repository overview and entry points";

/// Maximum ranked results returned per query.
pub const MAX_RESULTS: usize = 25;

/// Build the Repository Intelligence payload for `task` against `root`.
///
/// The first call indexes the tree (seconds on large repositories);
/// subsequent calls with an unchanged tree serve from the content-hash
/// validated persisted index.
pub fn repo_intel_payload(root: &Path, task: &str) -> Result<Value> {
    let task = task.trim();
    let task = if task.is_empty() { DEFAULT_TASK } else { task };

    let started = std::time::Instant::now();
    let query = PersistedIndex::new(root).build()?;
    let results = query.relevant_context(task);
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    let packages: Vec<String> = query.packages().iter().map(|p| p.name.clone()).collect();
    let languages: Vec<String> = {
        let mut langs: Vec<String> = query.files().iter().map(|f| f.language.label()).collect();
        langs.sort();
        langs.dedup();
        langs
    };
    let entry_points: Vec<String> = query
        .entry_points_list()
        .iter()
        .map(|e| {
            format!(
                "{} ({})",
                e.path.display(),
                serde_json::to_value(&e.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_else(|| "entry".to_string())
            )
        })
        .collect();
    let architecture_facts: Vec<Value> = query
        .architecture()
        .facts
        .iter()
        .map(|f| json!({ "fact": f.fact, "value": f.value }))
        .collect();
    let recent_changes: Vec<Value> = query
        .recent_changes(5)
        .iter()
        .map(|c| {
            json!({
                "short_id": c.short_id,
                "message": c.message.lines().next().unwrap_or("").to_string(),
                "author": c.author,
            })
        })
        .collect();

    let results_json: Vec<Value> = results
        .iter()
        .take(MAX_RESULTS)
        .map(|r| {
            json!({
                "resource": r.resource,
                "resource_type": r.resource_type,
                "relevance": r.relevance,
                "reason": r.reason,
            })
        })
        .collect();

    Ok(json!({
        "task": task,
        "elapsed_ms": elapsed_ms,
        "summary": {
            "file_count": query.file_count(),
            "symbol_count": query.symbol_count(),
            "package_count": query.package_count(),
            "packages": packages,
            "languages": languages,
            "entry_points": entry_points,
            "architecture_facts": architecture_facts,
            "recent_changes": recent_changes,
        },
        "results": results_json,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// A minimal git repo with one Rust file, so the index has content.
    fn sample_repo() -> (tempfile::TempDir, std::path::PathBuf) {
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
        run(&["config", "user.email", "t@example.com"]);
        run(&["config", "user.name", "t"]);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src").join("engine.rs"),
            "pub struct Engine;\n\nimpl Engine {\n    pub fn run(&self) {}\n}\n",
        )
        .unwrap();
        std::fs::write(root.join("README.md"), "# sample\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "initial commit"]);
        (dir, root)
    }

    #[test]
    fn payload_reflects_the_real_index_and_echoes_the_task() {
        let (_dir, root) = sample_repo();
        let payload = repo_intel_payload(&root, "engine run implementation").unwrap();

        assert_eq!(payload["task"], "engine run implementation");
        let summary = &payload["summary"];
        assert!(
            summary["file_count"].as_u64().unwrap() >= 2,
            "engine.rs + README.md must be indexed: {summary:?}"
        );
        assert!(
            summary["symbol_count"].as_u64().unwrap() >= 1,
            "Engine must be symbol-indexed: {summary:?}"
        );
        assert!(summary["packages"].as_array().unwrap().is_empty());
        assert!(summary["languages"]
            .as_array()
            .unwrap()
            .contains(&json!("Rust")));
        // The retriever ran and ranked something.
        let results = payload["results"].as_array().unwrap();
        assert!(
            !results.is_empty(),
            "a task mentioning 'engine' must surface engine.rs: {results:?}"
        );
        assert!(results[0]["relevance"].is_number());
        assert!(results[0]["reason"].is_string());
    }

    #[test]
    fn an_empty_task_falls_back_to_the_documented_default() {
        let (_dir, root) = sample_repo();
        let payload = repo_intel_payload(&root, "   ").unwrap();
        assert_eq!(payload["task"], DEFAULT_TASK);
    }

    #[test]
    fn payload_is_smaller_than_the_result_cap() {
        let (_dir, root) = sample_repo();
        let payload = repo_intel_payload(&root, DEFAULT_TASK).unwrap();
        let results = payload["results"].as_array().unwrap();
        assert!(results.len() <= MAX_RESULTS);
    }
}
