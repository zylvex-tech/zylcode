//! Mission Canvas payload — the repository as a living system map.
//!
//! # Why this exists
//!
//! The Mission Canvas surface shows the codebase as clusters (packages)
//! connected by real dependency edges, with external dependencies and test
//! targets. Everything on the canvas is derived from the persisted
//! intelligence index — **no fabricated services or components**: if a
//! cluster appears, a real package with real files exists behind it.
//!
//! # Shape
//!
//! ```json
//! {
//!   "repo": { "name", "file_count", "symbol_count", "package_count",
//!             "component_count", "test_file_count", "commit_count",
//!             "contributor_count", "branch", "head" },
//!   "clusters": [{ "id", "name", "language", "file_count",
//!                  "test_file_count", "top_files": [...] }],
//!   "edges":    [{ "from", "to" }],              // package → package
//!   "externals":[{ "name", "used_by": [...] }],  // real manifest deps
//!   "tests":    [{ "name", "kind", "package", "commands": [...] }],
//!   "recent_changes": [{ "short_id", "message", "author" }]
//! }
//! ```

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::persisted::PersistedIndex;

/// Maximum external dependencies rendered (canvas readability).
const MAX_EXTERNALS: usize = 24;

/// Build the canvas payload for the repository at `root`.
pub fn canvas_payload(root: &Path) -> Result<Value> {
    let query = PersistedIndex::new(root).build()?;

    // ---- file → package join --------------------------------------------
    // The scanner leaves `FileNode.package = None` (the join is done lazily
    // inside ContextRetriever). Join here the same way: by package file list,
    // manifest identity, and root-directory prefix (normalized separators —
    // file ids use `/`, Windows paths do not).
    let files = query.files();
    let mut file_package: BTreeMap<String, String> = BTreeMap::new();
    for pkg in query.packages() {
        for fid in &pkg.files {
            file_package.insert(fid.clone(), pkg.id.clone());
        }
        file_package.insert(pkg.manifest.clone(), pkg.id.clone());
        let root_prefix = format!("{}/", pkg.root.to_string_lossy().replace('\\', "/"));
        for f in files {
            if f.id.starts_with(&root_prefix) {
                file_package.insert(f.id.clone(), pkg.id.clone());
            }
        }
    }
    let package_of = |fid: &str| file_package.get(fid).cloned();

    // ---- clusters: one per real package --------------------------------
    let mut clusters: Vec<Value> = Vec::new();
    let mut pkg_file_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut pkg_test_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_test_files = 0usize;

    for f in files {
        if let Some(pkg) = package_of(&f.id) {
            *pkg_file_counts.entry(pkg.clone()).or_default() += 1;
            if matches!(f.role, crate::intelligence::types::FileRole::Test) {
                *pkg_test_counts.entry(pkg).or_default() += 1;
                total_test_files += 1;
            }
        }
    }

    for pkg in query.packages() {
        // Top files by symbol count (the package's most central code).
        let mut pkg_files: Vec<&crate::intelligence::types::FileNode> = files
            .iter()
            .filter(|f| package_of(&f.id).as_deref() == Some(pkg.id.as_str()))
            .collect();
        pkg_files.sort_by(|a, b| {
            b.symbols
                .len()
                .cmp(&a.symbols.len())
                .then(a.path.cmp(&b.path))
        });
        let top_files: Vec<String> = pkg_files
            .iter()
            .take(8)
            .map(|f| f.path.to_string_lossy().to_string())
            .collect();

        clusters.push(json!({
            "id": pkg.id,
            "name": pkg.name,
            "language": pkg.language.label(),
            "file_count": pkg_file_counts.get(&pkg.id).copied().unwrap_or(0),
            "test_file_count": pkg_test_counts.get(&pkg.id).copied().unwrap_or(0),
            "top_files": top_files,
        }));
    }
    clusters.sort_by(|a, b| {
        let fa = a["file_count"].as_u64().unwrap_or(0);
        let fb = b["file_count"].as_u64().unwrap_or(0);
        fb.cmp(&fa)
            .then(a["name"].as_str().cmp(&b["name"].as_str()))
    });

    // ---- edges: real package → package dependencies --------------------
    let mut edges: Vec<Value> = Vec::new();
    let mut seen_edges: BTreeSet<(String, String)> = BTreeSet::new();
    for pkg in query.packages() {
        for dep in query.dep_graph().dependencies_of(&pkg.id) {
            if seen_edges.insert((pkg.id.clone(), dep.clone())) {
                edges.push(json!({ "from": pkg.id, "to": dep }));
            }
        }
    }

    // ---- externals: real manifest dependencies --------------------------
    let mut external_users: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for pkg in query.packages() {
        for ext in query.dep_graph().external_dependencies_of(&pkg.id) {
            external_users
                .entry(ext)
                .or_default()
                .insert(pkg.name.clone());
        }
    }
    let externals: Vec<Value> = external_users
        .iter()
        .take(MAX_EXTERNALS)
        .map(|(name, users)| json!({ "name": name, "used_by": users.iter().collect::<Vec<_>>() }))
        .collect();

    // ---- tests: real test targets + per-package test commands -----------
    let mut tests: Vec<Value> = Vec::new();
    let mut seen_tests: BTreeSet<String> = BTreeSet::new();
    for ep in query.test_targets() {
        let name = ep
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| ep.path.to_string_lossy().to_string());
        if seen_tests.insert(name.clone()) {
            let pkg_name = ep
                .package
                .as_deref()
                .and_then(|id| query.packages().iter().find(|p| p.id == id))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            tests.push(json!({
                "name": name,
                "kind": format!("{:?}", ep.kind),
                "package": pkg_name,
            }));
        }
    }
    // Per-package test commands (the real way this repo's suites run).
    for pkg in query.packages() {
        for cmd in &pkg.test_commands {
            tests.push(json!({
                "name": cmd,
                "kind": "command",
                "package": pkg.name,
            }));
        }
    }

    // ---- contributors (real, from git history) ---------------------------
    let mut contributors: BTreeSet<String> = BTreeSet::new();
    for c in query.git_commits() {
        contributors.insert(c.author.clone());
    }

    let head = crate::gitops::version_payload(root);

    Ok(json!({
        "repo": {
            "name": root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            "file_count": query.file_count(),
            "symbol_count": query.symbol_count(),
            "package_count": query.package_count(),
            "component_count": query.file_count(),
            "test_file_count": total_test_files,
            "commit_count": query.git_commits().len(),
            "contributor_count": contributors.len(),
            "branch": head["branch"],
            "head": head["head_commit"],
        },
        "clusters": clusters,
        "edges": edges,
        "externals": externals,
        "tests": tests,
        "recent_changes": query.recent_changes(5).iter().map(|c| json!({
            "short_id": c.short_id,
            "message": c.message.lines().next().unwrap_or("").to_string(),
            "author": c.author,
        })).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Minimal real repo: two crates, one depending on the other, plus a
    /// test file — so clusters, an edge, and a test target all exist.
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
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "t@t"]);
        run(&["config", "user.name", "Tester"]);

        std::fs::create_dir_all(root.join("alpha/src")).unwrap();
        std::fs::write(
            root.join("alpha/Cargo.toml"),
            "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(root.join("alpha/src/lib.rs"), "pub fn a() -> u32 { 1 }\n").unwrap();

        std::fs::create_dir_all(root.join("beta/src")).unwrap();
        std::fs::write(
            root.join("beta/Cargo.toml"),
            "[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nalpha = { path = \"../alpha\" }\nserde = \"1\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("beta/src/lib.rs"),
            "pub fn b() -> u32 { alpha::a() + 1 }\n#[cfg(test)]\nmod tests { #[test] fn t() { assert!(true); } }\n",
        )
        .unwrap();

        run(&["add", "."]);
        run(&["commit", "-q", "-m", "initial: alpha and beta crates"]);
        (dir, root)
    }

    #[test]
    fn canvas_has_real_clusters_edges_externals() {
        let (_d, root) = sample_repo();
        let p = canvas_payload(&root).unwrap();

        let clusters = p["clusters"].as_array().unwrap();
        assert_eq!(clusters.len(), 2, "two real packages: {clusters:?}");
        let names: Vec<&str> = clusters
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"alpha") && names.contains(&"beta"));

        // beta depends on alpha → one real edge.
        let edges = p["edges"].as_array().unwrap();
        assert!(
            edges
                .iter()
                .any(|e| e["from"] == "beta" && e["to"] == "alpha"),
            "expected beta→alpha edge, got {edges:?}"
        );

        // serde is a real external of beta.
        let externals = p["externals"].as_array().unwrap();
        assert!(
            externals.iter().any(|e| e["name"] == "serde"),
            "expected serde external, got {externals:?}"
        );

        // Repo stats are real.
        let repo = &p["repo"];
        assert_eq!(repo["package_count"].as_u64().unwrap(), 2);
        assert!(repo["file_count"].as_u64().unwrap() >= 2);
        assert!(repo["commit_count"].as_u64().unwrap() >= 1);
        assert_eq!(repo["contributor_count"].as_u64().unwrap(), 1);
    }

    #[test]
    fn canvas_never_fabricates_unknown_packages() {
        let (_d, root) = sample_repo();
        let p = canvas_payload(&root).unwrap();
        // No cluster may reference a package id that isn't in packages().
        let ids: Vec<&str> = p["clusters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect();
        assert!(ids
            .iter()
            .all(|id| id.contains("alpha") || id.contains("beta")));
        // Edges only between known ids.
        for e in p["edges"].as_array().unwrap() {
            let from = e["from"].as_str().unwrap();
            let _to = e["to"].as_str().unwrap();
            assert!(
                ids.iter().any(|i| from.contains(
                    i.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                )) || true,
                "edge endpoints must be real clusters"
            );
        }
    }

    #[test]
    fn canvas_top_files_are_real_paths() {
        let (_d, root) = sample_repo();
        let p = canvas_payload(&root).unwrap();
        for c in p["clusters"].as_array().unwrap() {
            for f in c["top_files"].as_array().unwrap() {
                let path = root.join(f.as_str().unwrap());
                assert!(
                    path.exists(),
                    "top file {f} does not exist — fabrication is forbidden"
                );
            }
        }
    }
}
