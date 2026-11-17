//! Impact analysis (Phase 2B, slice B2): "what would this change touch?"
//!
//! # Why this exists
//!
//! Retrieval answers *where is this code*. Impact answers a different question:
//! given a proposed change, which files depend on it, which tests exercise it,
//! and **how do we know** each of those. A list of file names with no reason is
//! a guess with better formatting, so every entry here carries a [`Relation`]
//! and a human-readable reason naming the edge that produced it.
//!
//! # What is actually resolved
//!
//! The scanner records file roles and hashes, but `FileNode::imports` was never
//! populated — there was no edge to traverse. This module extracts import
//! statements and resolves them to file ids in two ways:
//!
//! - **TypeScript/JavaScript relative imports** (`./x`, `../y`) — resolved
//!   against the importer's directory with the usual extension and `/index`
//!   candidates. Deterministic; no guessing.
//! - **Rust `use crate::a::b::C`** — resolved by matching the module path as a
//!   file-path suffix inside the indexed tree. This is a **heuristic**: a
//!   `use` through a re-export or a macro-generated module will not match, and
//!   the report says so rather than pretending otherwise.
//!
//! Bare specifiers (`react`, `lodash`) are external and deliberately not
//! resolved: they are not in the repository, and inventing an edge to a
//! `node_modules` copy would make the graph look complete when it is not.
//!
//! # What this cannot answer
//!
//! - **Call graphs.** Symbols are `DEFINITION_INDEXED`: references are not
//!   resolved, so "who calls this function" is unknown. That is reported in
//!   [`ImpactReport::unknowns`] on every report, not omitted when inconvenient.
//! - **Dynamic imports, re-exports, path aliases** (`tsconfig` `paths`).
//! - Files the scanner skips (size, binary, ignore rules) are absent from the
//!   graph entirely.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::intelligence::query::RepoQuery;
use crate::intelligence::types::{FileRole, Language};

/// What is being changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeSpec {
    /// A repository-relative file path.
    File(String),
    /// A symbol name; the file that declares it is the starting point.
    Symbol(String),
}

impl ChangeSpec {
    fn describe(&self) -> String {
        match self {
            ChangeSpec::File(path) => format!("file {path}"),
            ChangeSpec::Symbol(name) => format!("symbol {name}"),
        }
    }
}

/// How an affected file relates to the change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    /// The change names this file (or the symbol lives in it).
    Named,
    /// Imports an affected file, directly or transitively.
    Imports,
    /// A test file that imports an affected file.
    TestedBy,
}

impl Relation {
    pub fn as_str(self) -> &'static str {
        match self {
            Relation::Named => "named",
            Relation::Imports => "imports",
            Relation::TestedBy => "tested_by",
        }
    }
}

/// One affected subject, with the evidence that put it in the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Impact {
    pub subject: String,
    pub relation: Relation,
    /// 0 for the entity the change names.
    pub depth: u8,
    pub reason: String,
}

/// The answer to "what would this touch", including what it could not determine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactReport {
    pub change: String,
    pub directly_affected: Vec<Impact>,
    pub transitively_affected: Vec<Impact>,
    pub tests_at_risk: Vec<Impact>,
    /// Statements of what this analysis did **not** determine, with a reason.
    pub unknowns: Vec<String>,
    /// High / Medium / Low, with the basis stated in `confidence_reason`.
    pub confidence: String,
    pub confidence_reason: String,
}

impl ImpactReport {
    pub fn total_affected(&self) -> usize {
        self.directly_affected.len() + self.transitively_affected.len()
    }
}

#[derive(Debug, Clone)]
struct Edge {
    importer: String,
    imported: String,
    /// Why this edge exists, in words an operator can check.
    reason: String,
}

/// File-level import graph built from the indexed tree.
pub struct ImportGraph {
    edges: Vec<Edge>,
    /// imported file id → indexes into `edges` (i.e. its importers).
    incoming: BTreeMap<String, Vec<usize>>,
    unresolved: usize,
}

impl ImportGraph {
    /// Build the graph by reading each indexed source file's imports.
    ///
    /// Unreadable files are skipped and counted as unresolved: an unread file is
    /// an unknown, not a file with no imports.
    pub fn build(query: &RepoQuery) -> Self {
        let known: BTreeSet<String> = query.files().iter().map(|f| normalize(&f.id)).collect();
        let mut edges = Vec::new();
        let mut unresolved = 0usize;

        for file in query.files() {
            if !matches!(
                file.language,
                Language::Rust | Language::TypeScript | Language::JavaScript
            ) {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&file.path) else {
                unresolved += 1;
                continue;
            };
            let importer = normalize(&file.id);
            for spec in import_specifiers(&content, &file.language) {
                let resolved = match file.language {
                    Language::TypeScript | Language::JavaScript => {
                        resolve_relative(&importer, &spec, &known).map(|target| (target, false))
                    }
                    Language::Rust => {
                        resolve_rust_module(&spec, &known).map(|target| (target, true))
                    }
                    _ => None,
                };
                match resolved {
                    Some((target, heuristic)) if target != importer => edges.push(Edge {
                        importer: importer.clone(),
                        imported: target,
                        reason: if heuristic {
                            format!("Rust `use` matched by module path: {spec}")
                        } else {
                            format!("imports \"{spec}\"")
                        },
                    }),
                    Some(_) => {}
                    None => unresolved += 1,
                }
            }
        }

        let mut incoming: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, edge) in edges.iter().enumerate() {
            incoming
                .entry(edge.imported.clone())
                .or_default()
                .push(index);
        }
        Self {
            edges,
            incoming,
            unresolved,
        }
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn unresolved_count(&self) -> usize {
        self.unresolved
    }

    fn importers_of(&self, file: &str) -> Vec<&Edge> {
        self.incoming
            .get(file)
            .map(|indexes| indexes.iter().map(|&i| &self.edges[i]).collect())
            .unwrap_or_default()
    }
}

/// Analyze the impact of a change against the indexed repository.
///
/// `max_depth` bounds how far the import closure is followed; the report states
/// the depth it used so "all callers" is never implied.
pub fn analyze(query: &RepoQuery, change: &ChangeSpec, max_depth: u8) -> ImpactReport {
    let graph = ImportGraph::build(query);
    let roles: BTreeMap<String, FileRole> = query
        .files()
        .iter()
        .map(|f| (normalize(&f.id), f.role.clone()))
        .collect();

    let mut report = ImpactReport {
        change: change.describe(),
        directly_affected: Vec::new(),
        transitively_affected: Vec::new(),
        tests_at_risk: Vec::new(),
        unknowns: Vec::new(),
        confidence: "Low".into(),
        confidence_reason: String::new(),
    };

    let seeds = match change {
        ChangeSpec::File(path) => {
            let wanted = normalize(path.trim_start_matches("./"));
            let matched = query
                .files()
                .iter()
                .map(|f| normalize(&f.id))
                .find(|id| *id == wanted || id.ends_with(&format!("/{wanted}")));
            match matched {
                Some(id) => vec![id],
                None => {
                    report
                        .unknowns
                        .push(format!("{path} is not in the index; nothing was inferred"));
                    report.confidence_reason =
                        "the named change is not indexed, so no impact could be computed".into();
                    return report;
                }
            }
        }
        ChangeSpec::Symbol(name) => {
            let ids: BTreeSet<String> = query
                .symbols()
                .iter()
                .filter(|s| s.name == *name)
                .map(|s| normalize(&s.file))
                .collect();
            if ids.is_empty() {
                report
                    .unknowns
                    .push(format!("no symbol named '{name}' is indexed"));
                report.confidence_reason =
                    "the named symbol is not indexed, so no impact could be computed".into();
                return report;
            }
            ids.into_iter().collect()
        }
    };

    for seed in &seeds {
        report.directly_affected.push(Impact {
            subject: seed.clone(),
            relation: Relation::Named,
            depth: 0,
            reason: format!("named by the change ({})", change.describe()),
        });
    }

    // Breadth-first over reverse import edges. `seen` starts with the seeds so a
    // cycle cannot re-add them or loop.
    let mut seen: BTreeSet<String> = seeds.iter().cloned().collect();
    let mut queue: VecDeque<(String, u8)> = seeds.iter().map(|s| (s.clone(), 0u8)).collect();
    while let Some((current, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        for edge in graph.importers_of(&current) {
            if !seen.insert(edge.importer.clone()) {
                continue;
            }
            let entry = Impact {
                subject: edge.importer.clone(),
                relation: if roles.get(&edge.importer) == Some(&FileRole::Test) {
                    Relation::TestedBy
                } else {
                    Relation::Imports
                },
                depth: depth + 1,
                reason: format!("{} affected {current}", edge.reason),
            };
            if entry.relation == Relation::TestedBy {
                report.tests_at_risk.push(entry.clone());
            }
            report.transitively_affected.push(entry);
            queue.push_back((edge.importer.clone(), depth + 1));
        }
    }

    // A test that lives in an affected file (inline `#[cfg(test)]`, co-located
    // `.test.ts`) is at risk even when it imports nothing.
    for seed in &seeds {
        if roles.get(seed) == Some(&FileRole::Test) {
            report.tests_at_risk.push(Impact {
                subject: seed.clone(),
                relation: Relation::TestedBy,
                depth: 0,
                reason: "the changed file is itself a test".into(),
            });
        }
    }

    report.transitively_affected.sort_by(|a, b| {
        a.depth
            .cmp(&b.depth)
            .then_with(|| a.subject.cmp(&b.subject))
    });
    report
        .tests_at_risk
        .sort_by(|a, b| a.subject.cmp(&b.subject));
    report.tests_at_risk.dedup_by(|a, b| a.subject == b.subject);

    // Unknowns are mandatory. An empty list would itself be a claim.
    report
        .unknowns
        .push("call graph is not resolved (symbols are DEFINITION_INDEXED); callers of a changed function are not listed".into());
    report
        .unknowns
        .push("dynamic imports, re-exports and tsconfig path aliases are not resolved".into());
    if graph.unresolved_count() > 0 {
        report.unknowns.push(format!(
            "{} import specifier(s) could not be resolved and contribute no edges",
            graph.unresolved_count()
        ));
    }
    report.unknowns.push(format!(
        " import closure is bounded at depth {max_depth}; deeper dependents are not listed"
    ));

    let heuristic_used = report
        .transitively_affected
        .iter()
        .any(|i| i.reason.contains("module path"));
    if graph.edge_count() == 0 {
        report.confidence = "Low".into();
        report.confidence_reason =
            "no import edges could be resolved, so absence of dependents is not evidence".into();
    } else if heuristic_used {
        report.confidence = "Medium".into();
        report.confidence_reason =
            "the closure includes Rust `use` edges matched by module path, which can miss re-exports"
                .into();
    } else {
        report.confidence = "High".into();
        report.confidence_reason =
            "every traversed edge is a relative TypeScript/JavaScript import resolved to a file"
                .into();
    }
    report
}

// ---------------------------------------------------------------------------
// Extraction and resolution
// ---------------------------------------------------------------------------

/// Import specifiers found in a file, in source order.
///
/// Deliberately textual: a full parser is C6 stage 2, and a regex that
/// over-matches would create edges that do not exist, which is worse than
/// missing one.
fn import_specifiers(content: &str, language: &Language) -> Vec<String> {
    let mut specs = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        match language {
            Language::Rust => {
                let Some(rest) = line
                    .strip_prefix("use ")
                    .or_else(|| line.strip_prefix("pub use "))
                else {
                    continue;
                };
                let path = rest.trim_end_matches(';').trim();
                if path.starts_with("crate::")
                    || path.starts_with("super::")
                    || path.starts_with("self::")
                {
                    specs.push(path.to_string());
                }
            }
            Language::TypeScript | Language::JavaScript => {
                // `import ... from "x"` / `import "x"` / `export ... from "x"`
                if let Some(start) = line.find("from \"") {
                    if let Some(end) = line[start + 6..].find('"') {
                        specs.push(line[start + 6..start + 6 + end].to_string());
                        continue;
                    }
                }
                if let Some(start) = line.find("from '") {
                    if let Some(end) = line[start + 6..].find('\'') {
                        specs.push(line[start + 6..start + 6 + end].to_string());
                        continue;
                    }
                }
                if let Some(rest) = line.strip_prefix("import \"") {
                    if let Some(end) = rest.find('"') {
                        specs.push(rest[..end].to_string());
                    }
                }
                if let Some(rest) = line.strip_prefix("import '") {
                    if let Some(end) = rest.find('\'') {
                        specs.push(rest[..end].to_string());
                    }
                }
            }
            _ => {}
        }
    }
    specs
}

fn normalize(path: &str) -> String {
    path.replace('\\', "/")
}

/// Resolve `./x` or `../y` from an importer's directory to an indexed file id.
fn resolve_relative(importer: &str, spec: &str, known: &BTreeSet<String>) -> Option<String> {
    if !spec.starts_with('.') {
        return None; // bare specifier: external package
    }
    let base_dir = Path::new(importer).parent()?;
    let joined = base_dir.join(spec);
    let normalized = normalize_relative_path(&joined)?;
    let candidates = [
        normalized.clone(),
        format!("{normalized}.ts"),
        format!("{normalized}.tsx"),
        format!("{normalized}.js"),
        format!("{normalized}.jsx"),
        format!("{normalized}/index.ts"),
        format!("{normalized}/index.tsx"),
        format!("{normalized}/index.js"),
    ];
    candidates.into_iter().find(|c| known.contains(c))
}

/// Join and collapse `.` / `..` without touching the filesystem.
fn normalize_relative_path(path: &Path) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                parts.pop()?;
            }
            std::path::Component::Normal(part) => parts.push(part.to_string_lossy().into()),
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

/// Resolve a Rust `crate::a::b::Item` path to a file by suffix matching.
///
/// Heuristic by construction: re-exports, `#[path]`, and macro-generated modules
/// are invisible here, and every edge produced carries `heuristic = true` so the
/// report can say the closure is not exact.
fn resolve_rust_module(spec: &str, known: &BTreeSet<String>) -> Option<String> {
    let mut path = spec;
    for prefix in ["crate::", "self::", "super::"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            path = rest;
            break;
        }
    }
    let mut segments: Vec<&str> = path.split("::").map(str::trim).collect();
    // Drop the item itself (a type/function) and any `{...}` glob group.
    while let Some(last) = segments.last() {
        if last.is_empty()
            || last.contains('{')
            || last.contains('}')
            || last.chars().next().is_some_and(|c| c.is_uppercase())
        {
            segments.pop();
        } else {
            break;
        }
    }
    if segments.is_empty() {
        return None;
    }
    let suffix = segments.join("/");
    let file = format!("{suffix}.rs");
    let module = format!("{suffix}/mod.rs");
    known
        .iter()
        .find(|id| id.as_str() == file || id.ends_with(&format!("/{file}")))
        .or_else(|| {
            known
                .iter()
                .find(|id| id.as_str() == module || id.ends_with(&format!("/{module}")))
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files_of(dir: &Path, names: &[&str]) {
        for name in names {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                path,
                if name.ends_with(".rs") {
                    "pub fn item() {}\n"
                } else {
                    "export const x = 1;\n"
                },
            )
            .unwrap();
        }
    }

    fn query_for(dir: &Path) -> RepoQuery {
        crate::intelligence::query::build_repo_query(dir).expect("build query")
    }

    #[test]
    fn relative_typescript_imports_are_resolved_to_files() {
        let dir = tempfile::tempdir().unwrap();
        files_of(dir.path(), &["src/app.ts", "src/widget.ts", "src/util.ts"]);
        std::fs::write(
            dir.path().join("src/app.ts"),
            "import { Widget } from \"./widget\";\nexport const app = 1;\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("src/widget.ts"),
            "import { helper } from \"./util\";\nexport class Widget {}\n",
        )
        .unwrap();

        let query = query_for(dir.path());
        let report = analyze(&query, &ChangeSpec::File("src/util.ts".into()), 2);

        let direct: Vec<&str> = report
            .directly_affected
            .iter()
            .map(|i| i.subject.as_str())
            .collect();
        assert!(direct.iter().any(|s| s.ends_with("util.ts")), "{direct:?}");

        let transitively: Vec<&str> = report
            .transitively_affected
            .iter()
            .map(|i| i.subject.as_str())
            .collect();
        assert!(
            transitively.iter().any(|s| s.ends_with("widget.ts")),
            "{transitively:?}"
        );
        assert!(
            transitively.iter().any(|s| s.ends_with("app.ts")),
            "{transitively:?}"
        );

        // Every entry explains itself.
        for item in report
            .directly_affected
            .iter()
            .chain(report.transitively_affected.iter())
        {
            assert!(!item.reason.trim().is_empty(), "{item:?}");
        }
        assert!(report.confidence == "High", "{}", report.confidence_reason);
    }

    #[test]
    fn a_symbol_change_starts_from_the_declaring_file() {
        let dir = tempfile::tempdir().unwrap();
        files_of(dir.path(), &["src/app.ts", "src/widget.ts"]);
        std::fs::write(
            dir.path().join("src/widget.ts"),
            "export class Widget { render(): void {} }\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("src/app.ts"),
            "import { Widget } from \"./widget\";\n",
        )
        .unwrap();

        let query = query_for(dir.path());
        let report = analyze(&query, &ChangeSpec::Symbol("Widget".into()), 2);
        assert!(report
            .directly_affected
            .iter()
            .any(|i| i.subject.ends_with("widget.ts")));
        assert!(report
            .transitively_affected
            .iter()
            .any(|i| i.subject.ends_with("app.ts")));
    }

    #[test]
    fn a_test_file_that_imports_the_change_is_listed_as_tests_at_risk() {
        let dir = tempfile::tempdir().unwrap();
        files_of(
            dir.path(),
            &["src/util.ts", "src/util.test.ts", "src/app.ts"],
        );
        std::fs::write(
            dir.path().join("src/util.test.ts"),
            "import { helper } from \"./util\";\n",
        )
        .unwrap();

        let query = query_for(dir.path());
        let report = analyze(&query, &ChangeSpec::File("src/util.ts".into()), 2);
        assert!(
            report
                .tests_at_risk
                .iter()
                .any(|i| i.subject.ends_with("util.test.ts")),
            "{:?}",
            report.tests_at_risk
        );
    }

    #[test]
    fn an_unindexed_file_produces_an_unknown_instead_of_a_guess() {
        let dir = tempfile::tempdir().unwrap();
        files_of(dir.path(), &["src/app.ts"]);
        let query = query_for(dir.path());
        let report = analyze(&query, &ChangeSpec::File("src/does_not_exist.ts".into()), 2);
        assert!(report.directly_affected.is_empty());
        assert!(report.transitively_affected.is_empty());
        assert!(report
            .unknowns
            .iter()
            .any(|u| u.contains("not in the index")));
        assert_eq!(report.confidence, "Low");
    }

    #[test]
    fn unresolved_specifiers_are_counted_and_reported() {
        let dir = tempfile::tempdir().unwrap();
        files_of(dir.path(), &["src/app.ts"]);
        std::fs::write(
            dir.path().join("src/app.ts"),
            "import React from \"react\";\nimport missing from \"./nope\";\n",
        )
        .unwrap();
        let query = query_for(dir.path());
        let report = analyze(&query, &ChangeSpec::File("src/app.ts".into()), 2);
        assert!(
            report
                .unknowns
                .iter()
                .any(|u| u.contains("could not be resolved")),
            "{:?}",
            report.unknowns
        );
    }

    #[test]
    fn the_depth_bound_is_stated_in_the_report() {
        let dir = tempfile::tempdir().unwrap();
        files_of(dir.path(), &["src/app.ts"]);
        let query = query_for(dir.path());
        let report = analyze(&query, &ChangeSpec::File("src/app.ts".into()), 1);
        assert!(report
            .unknowns
            .iter()
            .any(|u| u.contains("bounded at depth 1")));
    }

    #[test]
    fn rust_use_edges_are_marked_heuristic_and_lower_confidence() {
        let dir = tempfile::tempdir().unwrap();
        files_of(
            dir.path(),
            &["crates/a/src/lib.rs", "crates/a/src/inner.rs"],
        );
        std::fs::write(
            dir.path().join("crates/a/src/lib.rs"),
            "use crate::inner::Thing;\npub fn f() {}\n",
        )
        .unwrap();

        let query = query_for(dir.path());
        let report = analyze(&query, &ChangeSpec::File("inner.rs".into()), 2);
        if report
            .transitively_affected
            .iter()
            .any(|i| i.reason.contains("module path"))
        {
            assert_eq!(report.confidence, "Medium");
        } else {
            // If suffix matching found nothing, that must surface as an unknown,
            // never as a confident empty answer.
            assert!(report
                .unknowns
                .iter()
                .any(|u| u.contains("could not be resolved")));
        }
    }
}
