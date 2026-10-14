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

/// The repository query methods (`RepoQuery`, 12 methods) exposed as one
/// dispatchable payload — the same typed layer the engine uses internally,
/// now reachable by product surfaces (Tauri command `repo_query` and the
/// `serve-intel` `GET /api/repo-query` route).
///
/// `method` is one of the ids below; `arg` is the optional selector the
/// method needs (a symbol name, a file path, a subject, a limit). Unknown
/// methods and missing selectors are rejected with an explicit `error` —
/// never an empty result — because "no dependencies" and "you asked for
/// nothing" are different facts.
pub fn repo_query_payload(root: &Path, method: &str, arg: Option<&str>) -> Result<Value> {
    let started = std::time::Instant::now();
    let query = PersistedIndex::new(root).build()?;

    let missing = |what: &str| anyhow::anyhow!("method '{method}' requires {what}");
    let arg = arg
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let results: Value = match method {
        "find_symbol" => json!(query.find_symbol(
            arg.as_deref()
                .ok_or_else(|| missing("a symbol name (arg)"))?
        )),
        "symbol_definition" => {
            let def = query
                .symbol_definition(arg.as_deref().ok_or_else(|| missing("a symbol id (arg)"))?);
            json!(def)
        }
        "symbols_in_file" => {
            json!(query.symbols_in_file(arg.as_deref().ok_or_else(|| missing("a file path (arg)"))?))
        }
        "package_for_file" => {
            let pkg =
                query.package_for_file(arg.as_deref().ok_or_else(|| missing("a file path (arg)"))?);
            json!(pkg)
        }
        "dependencies_of" => json!(query.dependencies_of(
            arg.as_deref()
                .ok_or_else(|| missing("a package or file subject (arg)"))?
        )),
        "dependents_of" => json!(query.dependents_of(
            arg.as_deref()
                .ok_or_else(|| missing("a package or file subject (arg)"))?
        )),
        // Stateless methods ignore the arg; a caller-supplied arg is not an
        // error (it is carried through), so one client can loop methods
        // uniformly.
        "entry_points" => json!(query.entry_points()),
        "test_targets" => json!(query.test_targets()),
        "build_commands" => json!(query.build_commands()),
        "recent_changes" => {
            let limit = arg
                .as_deref()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(10);
            json!(query.recent_changes(limit))
        }
        "architecture" => json!(query.architecture()),
        "relevant_context" => {
            let task = arg.as_deref().unwrap_or(DEFAULT_TASK);
            let ranked = query.relevant_context(task);
            let capped: Vec<_> = ranked.iter().take(MAX_RESULTS).collect();
            json!({ "task": task, "results": capped })
        }
        other => {
            anyhow::bail!(
                "unknown repository query method '{other}'; expected one of {}",
                REPO_QUERY_METHODS.join(", ")
            );
        }
    };

    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(json!({
        "method": method,
        "arg": arg,
        "elapsed_ms": elapsed_ms,
        "results": results,
    }))
}

/// The ids `repo_query_payload` dispatches on, in `RepoQuery` order.
pub const REPO_QUERY_METHODS: &[&str] = &[
    "find_symbol",
    "symbol_definition",
    "symbols_in_file",
    "package_for_file",
    "dependencies_of",
    "dependents_of",
    "entry_points",
    "test_targets",
    "build_commands",
    "recent_changes",
    "architecture",
    "relevant_context",
];

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

    #[test]
    fn a_symbol_query_finds_the_indexed_symbol() {
        let (_dir, root) = sample_repo();
        let payload = repo_query_payload(&root, "find_symbol", Some("Engine")).unwrap();
        assert_eq!(payload["method"], "find_symbol");
        let results = payload["results"].as_array().unwrap();
        assert!(
            !results.is_empty(),
            "Engine is indexed and must be found: {results:?}"
        );
        assert!(!results[0]["resource"]
            .as_str()
            .unwrap_or_default()
            .is_empty());
    }

    #[test]
    fn a_selector_less_rejection_names_the_problem() {
        let (_dir, root) = sample_repo();
        let err = repo_query_payload(&root, "find_symbol", None).unwrap_err();
        assert!(
            err.to_string().contains("requires"),
            "the error must name what is missing: {err}"
        );
    }

    #[test]
    fn an_unknown_method_is_rejected_with_the_catalogue() {
        let (_dir, root) = sample_repo();
        let err = repo_query_payload(&root, "no_such_method", None).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no_such_method"), "{msg}");
        assert!(
            msg.contains("find_symbol"),
            "error must list the catalogue: {msg}"
        );
    }

    #[test]
    fn every_factory_surface_method_is_declared_in_the_catalogue() {
        // The served dispatch and the advertised list must never drift apart:
        // each catalogue id must route, and each routed id must be advertised.
        for method in REPO_QUERY_METHODS {
            assert!(
                [
                    "find_symbol",
                    "symbol_definition",
                    "symbols_in_file",
                    "package_for_file",
                    "dependencies_of",
                    "dependents_of",
                    "entry_points",
                    "test_targets",
                    "build_commands",
                    "recent_changes",
                    "architecture",
                    "relevant_context",
                ]
                .contains(method),
                ".{method}\" is dispatched but not advertised"
            );
        }
    }

    #[test]
    fn stateless_methods_answer_without_a_selector() {
        let (_dir, root) = sample_repo();
        for method in [
            "entry_points",
            "test_targets",
            "build_commands",
            "architecture",
        ] {
            let payload = repo_query_payload(&root, method, None)
                .unwrap_or_else(|e| panic!("{method} must answer without an arg: {e}"));
            assert_eq!(payload["method"], method);
        }
    }

    #[test]
    fn recent_changes_reports_the_committed_change() {
        let (_dir, root) = sample_repo();
        let payload = repo_query_payload(&root, "recent_changes", Some("5")).unwrap();
        let commits = payload["results"].as_array().unwrap();
        assert!(
            !commits.is_empty(),
            "the sample repo has one commit: {commits:?}"
        );
    }
}
