//! Repository Intelligence query API.
//!
//! Typed query layer that hides database internals from the AgentLoop.
//! Every result includes relevance, reason, and provenance.

use crate::intelligence::context::ContextRetriever;
use crate::intelligence::dependency::DependencyGraph;
use crate::intelligence::manifest::{discover_packages, resolve_internal_deps};
use crate::intelligence::scanner::{scan_repository, ScannerConfig};
use crate::intelligence::symbols::extract_symbols;
use crate::intelligence::types::{
    ArchitecturalFingerprint, ContextResult, EntryPoint, FileNode, GitCommit, Language, Package,
    Symbol,
};
use anyhow::Result;
use std::path::Path;

/// Repository Intelligence query interface.
pub struct RepoQuery {
    files: Vec<FileNode>,
    symbols: Vec<Symbol>,
    packages: Vec<Package>,
    dep_graph: DependencyGraph,
    entry_points: Vec<EntryPoint>,
    architecture: ArchitecturalFingerprint,
    git_commits: Vec<GitCommit>,
    retriever: ContextRetriever,
}

impl RepoQuery {
    /// Create a new query interface from indexed data.
    pub fn new(
        files: Vec<FileNode>,
        symbols: Vec<Symbol>,
        packages: Vec<Package>,
        dep_graph: DependencyGraph,
        entry_points: Vec<EntryPoint>,
        architecture: ArchitecturalFingerprint,
        git_commits: Vec<GitCommit>,
    ) -> Self {
        let retriever = ContextRetriever::new(
            &files,
            &symbols,
            &packages,
            &dep_graph,
            &entry_points,
            &architecture,
            &git_commits,
        );

        Self {
            files,
            symbols,
            packages,
            dep_graph,
            entry_points,
            architecture,
            git_commits,
            retriever,
        }
    }

    /// Get a high-level repository summary.
    pub fn repository_summary(&self) -> String {
        let mut parts = Vec::new();
        parts.push(format!(
            "Repository with {} files, {} symbols, {} packages",
            self.files.len(),
            self.symbols.len(),
            self.packages.len()
        ));

        if !self.packages.is_empty() {
            let names: Vec<&str> = self.packages.iter().map(|p| p.name.as_str()).collect();
            parts.push(format!("Packages: {}", names.join(", ")));
        }

        let languages: std::collections::HashSet<String> = self
            .files
            .iter()
            .map(|f| format!("{:?}", f.language))
            .collect();
        parts.push(format!(
            "Languages: {}",
            languages.into_iter().collect::<Vec<_>>().join(", ")
        ));

        parts.push(format!("Entry points: {}", self.entry_points.len()));

        if !self.architecture.facts.is_empty() {
            let facts: Vec<String> = self
                .architecture
                .facts
                .iter()
                .map(|f| format!("{}: {}", f.fact, f.value))
                .collect();
            parts.push(format!("Architecture: {}", facts.join("; ")));
        }

        parts.join("\n")
    }

    /// Find symbols by name.
    pub fn find_symbol(&self, name: &str) -> Vec<ContextResult> {
        let query_lower = name.to_lowercase();
        self.symbols
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower)
                    || s.id.to_lowercase().contains(&query_lower)
            })
            .map(|s| ContextResult {
                resource: s.id.clone(),
                resource_type: "symbol".to_string(),
                relevance: if s.name.to_lowercase() == query_lower {
                    1.0
                } else {
                    0.8
                },
                reason: format!(
                    "{} {} defined at {}:{}",
                    format!("{:?}", s.visibility).to_lowercase(),
                    format!("{:?}", s.kind).to_lowercase(),
                    s.file,
                    s.line
                ),
                provenance: crate::intelligence::types::Provenance::Parsed,
                snippet: None,
            })
            .collect()
    }

    /// Get the definition of a symbol.
    pub fn symbol_definition(&self, symbol_id: &str) -> Option<ContextResult> {
        self.symbols
            .iter()
            .find(|s| s.id == symbol_id)
            .map(|s| ContextResult {
                resource: s.id.clone(),
                resource_type: "symbol_definition".to_string(),
                relevance: 1.0,
                reason: format!("Defined in {} at line {}", s.file, s.line),
                provenance: crate::intelligence::types::Provenance::Parsed,
                snippet: None,
            })
    }

    /// Get all symbols in a file.
    pub fn symbols_in_file(&self, file_path: &str) -> Vec<ContextResult> {
        self.symbols
            .iter()
            .filter(|s| s.file == file_path)
            .map(|s| ContextResult {
                resource: s.id.clone(),
                resource_type: "symbol".to_string(),
                relevance: 1.0,
                reason: format!("Defined in {}", s.file),
                provenance: crate::intelligence::types::Provenance::Parsed,
                snippet: None,
            })
            .collect()
    }

    /// Get the package that owns a file.
    pub fn package_for_file(&self, file_path: &str) -> Option<ContextResult> {
        // Check file's package assignment
        if let Some(file) = self.files.iter().find(|f| f.id == file_path) {
            if let Some(pkg_name) = &file.package {
                if let Some(pkg) = self.packages.iter().find(|p| &p.name == pkg_name) {
                    return Some(ContextResult {
                        resource: pkg.name.clone(),
                        resource_type: "package".to_string(),
                        relevance: 1.0,
                        reason: format!("File {} belongs to package {}", file_path, pkg.name),
                        provenance: crate::intelligence::types::Provenance::Parsed,
                        snippet: None,
                    });
                }
            }
        }

        // Fallback: infer from path
        for pkg in &self.packages {
            if file_path.starts_with(&*pkg.root.to_string_lossy()) {
                return Some(ContextResult {
                    resource: pkg.name.clone(),
                    resource_type: "package".to_string(),
                    relevance: 0.7,
                    reason: format!(
                        "File {} is under package root {}",
                        file_path,
                        pkg.root.display()
                    ),
                    provenance: crate::intelligence::types::Provenance::Inferred,
                    snippet: None,
                });
            }
        }

        None
    }

    /// Get dependencies of a subject (package or file).
    pub fn dependencies_of(&self, subject: &str) -> Vec<ContextResult> {
        let deps = self.dep_graph.dependencies_of(subject);
        deps.into_iter()
            .map(|d| ContextResult {
                resource: d.clone(),
                resource_type: "dependency".to_string(),
                relevance: 1.0,
                reason: format!("{} depends on {}", subject, d),
                provenance: crate::intelligence::types::Provenance::Parsed,
                snippet: None,
            })
            .collect()
    }

    /// Get dependents of a subject (what depends on it).
    pub fn dependents_of(&self, subject: &str) -> Vec<ContextResult> {
        let deps = self.dep_graph.dependents_of(subject);
        deps.into_iter()
            .map(|d| ContextResult {
                resource: d.clone(),
                resource_type: "dependent".to_string(),
                relevance: 1.0,
                reason: format!("{} depends on {}", d, subject),
                provenance: crate::intelligence::types::Provenance::Parsed,
                snippet: None,
            })
            .collect()
    }

    /// Get all entry points.
    pub fn entry_points(&self) -> &[EntryPoint] {
        &self.entry_points
    }

    /// Get test targets.
    pub fn test_targets(&self) -> Vec<&EntryPoint> {
        self.entry_points
            .iter()
            .filter(|ep| ep.kind == crate::intelligence::types::EntryPointKind::Test)
            .collect()
    }

    /// Get build commands for all packages.
    pub fn build_commands(&self) -> Vec<String> {
        self.packages
            .iter()
            .flat_map(|p| p.build_commands.clone())
            .collect()
    }

    /// Get recent git commits.
    pub fn recent_changes(&self, limit: usize) -> &[GitCommit] {
        let end = self.git_commits.len().min(limit);
        &self.git_commits[..end]
    }

    /// Get the architectural fingerprint.
    pub fn architecture(&self) -> &ArchitecturalFingerprint {
        &self.architecture
    }

    /// Get relevant context for a task description.
    pub fn relevant_context(&self, task: &str) -> Vec<ContextResult> {
        self.retriever.retrieve(task)
    }

    /// The dependency graph (read access for specialized payloads).
    pub fn dep_graph(&self) -> &DependencyGraph {
        &self.dep_graph
    }

    /// Get total file count.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// The indexed file inventory (persisted-index reconstruction input).
    pub fn files(&self) -> &[FileNode] {
        &self.files
    }

    /// The extracted symbol inventory.
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// The discovered packages.
    pub fn packages(&self) -> &[Package] {
        &self.packages
    }

    /// The discovered entry points.
    pub fn entry_points_list(&self) -> &[EntryPoint] {
        &self.entry_points
    }

    /// The git history feeding recency and co-change evidence.
    pub fn git_commits(&self) -> &[GitCommit] {
        &self.git_commits
    }

    /// Get total symbol count.
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// Get total package count.
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }
}

/// Build a complete `RepoQuery` for a repository root (P1.2 product
/// surface). This is the single canonical indexing pipeline — scanner,
/// package discovery, dependency graph, entry points, architectural
/// fingerprint, and git history — shared by every product consumer
/// (CLI, future AgentLoop/ContextBuilder integration) so retrieval quality
/// proven in the benchmark is the quality the product delivers.
///
/// Git history is optional: indexing succeeds without a git repository,
/// losing recency and co-change signals but not core retrieval.
pub fn build_repo_query(root: &Path) -> Result<RepoQuery> {
    let config = ScannerConfig {
        max_depth: 10,
        max_file_size: 512 * 1024,
        follow_symlinks: false,
    };
    let scan_result = scan_repository(root, &config)?;

    let mut packages = discover_packages(root)?;
    resolve_internal_deps(&mut packages);

    let mut all_symbols = Vec::new();
    for file in &scan_result.files {
        if file.language == Language::Rust {
            if let Ok(content) = std::fs::read_to_string(&file.path) {
                all_symbols.extend(extract_symbols(
                    &file.id,
                    &file.path,
                    &content,
                    &file.language,
                ));
            }
        }
    }

    let dep_graph = DependencyGraph::from_packages(&packages);
    let entry_points = crate::intelligence::entry_points::discover_entry_points(root, &packages)?;
    let architecture = crate::intelligence::architecture::generate_fingerprint(root, &packages)?;
    // Co-change evidence is mined from the FULL history (sliding-window
    // defect: a fixed 40-commit window made coupling assertions HEAD-
    // sensitive — the agent.rs/crash_recovery coupling at commit 6f564ac
    // slid out of the window as commits accumulated). Recency is pinned to
    // the 20 newest commits downstream, so recency semantics are unchanged.
    let git_commits = crate::intelligence::git::get_full_history_commits(root).unwrap_or_default();

    Ok(RepoQuery::new(
        scan_result.files,
        all_symbols,
        packages,
        dep_graph,
        entry_points,
        architecture,
        git_commits,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::types::*;
    use std::path::PathBuf;

    fn make_test_query() -> RepoQuery {
        let files = vec![
            FileNode {
                id: "src/agent.rs".to_string(),
                path: PathBuf::from("src/agent.rs"),
                language: Language::Rust,
                role: FileRole::Source,
                size: 1000,
                content_hash: "abc".to_string(),
                package: Some("zylcode-core".to_string()),
                symbols: Vec::new(),
                imports: Vec::new(),
                indexed_at: chrono::Utc::now(),
                support_level: LanguageSupport::Parsed,
            },
            FileNode {
                id: "src/ledger.rs".to_string(),
                path: PathBuf::from("src/ledger.rs"),
                language: Language::Rust,
                role: FileRole::Source,
                size: 500,
                content_hash: "def".to_string(),
                package: Some("zylcode-core".to_string()),
                symbols: Vec::new(),
                imports: Vec::new(),
                indexed_at: chrono::Utc::now(),
                support_level: LanguageSupport::Parsed,
            },
        ];

        let symbols = vec![
            Symbol {
                id: "agent::AgentLoop".to_string(),
                name: "AgentLoop".to_string(),
                kind: SymbolKind::Struct,
                file: "src/agent.rs".to_string(),
                line: 10,
                end_line: None,
                visibility: Visibility::Public,
                parent: None,
                language: Language::Rust,
                support_level: LanguageSupport::Parsed,
            },
            Symbol {
                id: "ledger::LedgerStore".to_string(),
                name: "LedgerStore".to_string(),
                kind: SymbolKind::Trait,
                file: "src/ledger.rs".to_string(),
                line: 5,
                end_line: None,
                visibility: Visibility::Public,
                parent: None,
                language: Language::Rust,
                support_level: LanguageSupport::Parsed,
            },
        ];

        let packages = vec![Package {
            id: "zylcode-core".to_string(),
            name: "zylcode-core".to_string(),
            version: "0.2.0".to_string(),
            root: PathBuf::from("crates/zylcode-core"),
            language: Language::Rust,
            manifest: "Cargo.toml".to_string(),
            files: Vec::new(),
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            build_commands: vec!["cargo build".to_string()],
            test_commands: vec!["cargo test".to_string()],
            entry_points: Vec::new(),
        }];

        let dep_graph = DependencyGraph::from_packages(&packages);
        let architecture = ArchitecturalFingerprint { facts: Vec::new() };

        RepoQuery::new(
            files,
            symbols,
            packages,
            dep_graph,
            Vec::new(),
            architecture,
            Vec::new(),
        )
    }

    #[test]
    fn repository_summary_test() {
        let query = make_test_query();
        let summary = query.repository_summary();
        assert!(summary.contains("2 files"));
        assert!(summary.contains("2 symbols"));
        assert!(summary.contains("zylcode-core"));
    }

    #[test]
    fn find_symbol_test() {
        let query = make_test_query();
        let results = query.find_symbol("AgentLoop");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource, "agent::AgentLoop");
        assert_eq!(results[0].relevance, 1.0);
    }

    #[test]
    fn find_symbol_fuzzy() {
        let query = make_test_query();
        let results = query.find_symbol("agent");
        assert!(!results.is_empty());
    }

    #[test]
    fn symbols_in_file_test() {
        let query = make_test_query();
        let results = query.symbols_in_file("src/agent.rs");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource, "agent::AgentLoop");
    }

    #[test]
    fn package_for_file_test() {
        let query = make_test_query();
        let result = query.package_for_file("src/agent.rs");
        assert!(result.is_some());
        assert_eq!(result.unwrap().resource, "zylcode-core");
    }

    #[test]
    fn dependencies_of_test() {
        let query = make_test_query();
        let deps = query.dependencies_of("zylcode-core");
        // No internal deps in test data
        assert!(deps.is_empty());
    }

    #[test]
    fn entry_points_test() {
        let query = make_test_query();
        assert!(query.entry_points().is_empty());
    }

    #[test]
    fn file_count_test() {
        let query = make_test_query();
        assert_eq!(query.file_count(), 2);
    }
}
